use crate::tg::types::{
    peer_ref_from_string, ChatType, NativeActionResult, NativeChat, NativeDialog,
    NativeDialogFlags, NativeDialogLoadRequest, NativeDialogLoadResult, NativeEvent,
    NativeEventKind, NativeLoadSource, NativePackedChat, NativePeer, NativeRawMessage,
    NativeSearchPage, NativeSearchResult, NativeSearchResultKind, NativeSeenChat,
};
use crate::tg::utils::{profile_photo_path_and_count, ProfilePhotoPath};
use crate::tg::Backend;
use anyhow::Result;
use dashmap::{DashMap as HashMap, DashSet as HashSet};
use grammers_client::media::Downloadable;
use grammers_client::peer::Peer;
use grammers_client::tl;
use grammers_session::types::PeerRef;
use grammers_tl_types::Serializable;
use log::{debug, error};
use napi_ohos::bindgen_prelude::{Buffer, FnArgs};
use napi_ohos::threadsafe_function::ThreadsafeFunctionCallMode;
use napi_ohos::tokio;
use ohos_hilog_binding::debug;
impl Backend {
    // pub async fn load_profile_photos(&mut self) -> Result<()> {
    //     let mut dialog_iter = self.client.iter_dialogs();
    //     while let Some(dialog) = dialog_iter.next().await? {
    //         let raw_chat = dialog.chat();
    //         let chat = NativeChat::from_raw(raw_chat).await;
    //         let profile_photo_path = get_profile_photo_path_and_count(raw_chat.id(), true);
    //         if profile_photo_path.current.is_none() {
    //             self.download_profile_photo(raw_chat, true, &profile_photo_path).await?;
    //         }
    //     }
    //     Ok(())
    // }

    // pub async fn get_chat(&self, chat_id: i64, chat_type: ChatType) -> Result<grammers_client::types::Chat> {
    //     let request = match chat_type {
    //         ChatType::User => {
    //             tl::enums::InputUser::User()
    //         }
    //         ChatType::Group => {}
    //         ChatType::Channel => {}
    //     };
    // }

    /// Iterate dialogs, load new messages from the supplied offsets, and notify
    /// the legacy callback surface plus the event-driven runtime surface.
    pub async fn load_chats_with_offset(
        &'static mut self,
        last_message_ids: Option<HashMap<i64, i32>>,
    ) -> Result<()> {
        let mut dialog_iter = self.client.iter_dialogs();
        while let Some(dialog) = dialog_iter.next().await? {
            tokio::spawn(self.save_session());
            // let dialog = Box::leak(Box::new(dialog));
            let raw_chat = dialog.peer();
            let mut chat = NativeChat::from_raw(raw_chat).await;
            debug!(
                "Loading chat:{}, chat_type: {:?}, forum: {}",
                chat.name, chat.chat_type, chat.forum
            );
            let packed_chat = dialog.peer_ref();
            // let profile_photo_path = Box::leak(Box::new(get_profile_photo_path_and_count(raw_chat.id())?));
            // if profile_photo_path.current.is_none() {
            //     let packed_chat = Box::leak(Box::new(packed_chat));
            //     tokio::spawn(
            //         self.download_chat_photo(packed_chat, true, profile_photo_path)
            //     );
            // }
            chat.pinned = dialog.raw.pinned();
            let last_message_id = if let Some(last_message_ids) = last_message_ids.as_ref() {
                last_message_ids.get(&chat.chat_id).map(|id| *id)
            } else {
                None
            };
            let message_iter = self.client.iter_messages(packed_chat);
            debug!("Loading chat: {} after {:?}", chat.name, last_message_id);
            let sorted_messages = self
                .load_messages_from_iter(message_iter, last_message_id)
                .await?;
            debug!(
                "Loaded chat: {:?} with {} messages",
                chat,
                sorted_messages.len()
            );
            if sorted_messages.is_empty() {
                // no new messages, then if the chat is not pinned, this technically means all chats
                // after this one will also have no new messages, so we can early stop here
                if !chat.pinned {
                    break;
                }
                continue;
            }

            let last_message = sorted_messages.values().last().unwrap();
            chat.last_message_id = last_message.message_id;
            chat.last_message_sender_name = last_message.sender_name.clone();
            chat.last_message_text = last_message.text.clone();
            chat.last_message_timestamp = last_message.timestamp;
            self.seen_packed_chats_map.insert(chat.chat_id, packed_chat);
            let native_peer = NativePeer::from_raw(raw_chat);
            let native_seen_chat = NativeSeenChat::from(native_peer.clone());
            if let Some(callback) = &self.cache_seen_chat_callback {
                callback.call(
                    Ok(native_seen_chat.clone()),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            self.chats_map.insert(raw_chat.id().bare_id(), chat.clone());
            debug!("before update_chat_callback call: chat name: {}", chat.name);
            let messages = sorted_messages.values().cloned().collect::<Vec<_>>();
            if let Some(callback) = &self.update_chat_callback {
                callback.call(
                    Ok(FnArgs::from((
                        native_seen_chat,
                        chat.clone(),
                        messages.clone(),
                    ))),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            let mut peer_event = NativeEvent::new(NativeEventKind::PeerUpserted);
            peer_event.peer = Some(native_peer.clone());
            self.emit_event(peer_event).await;

            let mut dialog_event = NativeEvent::new(NativeEventKind::DialogUpserted);
            dialog_event.peer = Some(native_peer);
            dialog_event.chat = Some(chat);
            dialog_event.messages = Some(messages);
            self.emit_event(dialog_event).await;
        }
        debug!("load_chats_with_offset done!");
        Ok(())
    }

    /// Sync cached chats from local database. This method normally should be called
    /// when the app starts. And it should be called only once.
    pub async fn sync_caches_from_local_db(
        &mut self,
        packed_chats: Vec<NativePackedChat>,
        // seen_chats: Vec<NativeSeenChat>,
        chats: Vec<NativeChat>,
    ) -> Result<()> {
        for packed_chat in packed_chats.iter() {
            self.seen_packed_chats_map.insert(
                packed_chat.chat_id,
                peer_ref_from_string(packed_chat.packed_chat.as_str())?,
            );
        }
        for chat in chats.iter() {
            self.chats_map.insert(chat.chat_id, chat.clone());
        }
        Ok(())
    }

    pub async fn load_dialogs_page(
        &self,
        request: NativeDialogLoadRequest,
    ) -> Result<NativeDialogLoadResult> {
        let requested_count = request.count.unwrap_or(50).clamp(1, 100) as usize;
        let mut dialog_iter = self.client.iter_dialogs().limit(requested_count + 1);
        let mut dialogs = Vec::new();
        let mut messages = Vec::new();

        while let Some(dialog) = dialog_iter.next().await? {
            if !dialog_matches_folder(&dialog.raw, request.folder_id) {
                continue;
            }

            let raw_peer = dialog.peer();
            let peer_ref = dialog.peer_ref();
            let peer = NativePeer::from_raw(raw_peer);
            let top_message = dialog
                .last_message
                .as_ref()
                .map(crate::tg::types::NativeMessage::from_raw);
            let chat = NativeChat::from_dialog(dialog.clone()).await;

            self.seen_packed_chats_map.insert(chat.chat_id, peer_ref);
            self.chats_map.insert(chat.chat_id, chat.clone());
            if let Some(message) = top_message.clone() {
                messages.push(message);
            }
            dialogs.push(NativeDialog {
                peer,
                chat,
                top_message,
            });

            if dialogs.len() > requested_count {
                break;
            }
        }

        let has_more = dialogs.len() > requested_count;
        if has_more {
            dialogs.truncate(requested_count);
        }

        let result = NativeDialogLoadResult {
            dialogs,
            messages,
            has_more,
            source: request.source.or(Some(NativeLoadSource::Network)),
        };

        let mut event = NativeEvent::new(NativeEventKind::DialogsLoaded);
        event.dialogs = Some(result.dialogs.clone());
        event.messages = Some(result.messages.clone());
        event.source = result.source.clone();
        self.emit_event(event).await;

        Ok(result)
    }

    pub async fn get_contacts(&self) -> Result<Vec<NativePeer>> {
        let response = self
            .client
            .invoke(&tl::functions::contacts::GetContacts { hash: 0 })
            .await?;
        let contacts = match response {
            tl::enums::contacts::Contacts::Contacts(contacts) => contacts
                .users
                .into_iter()
                .map(|user| {
                    let user = grammers_client::peer::User::from_raw(&self.client, user);
                    NativePeer::from_user(&user)
                })
                .collect(),
            tl::enums::contacts::Contacts::NotModified => Vec::new(),
        };
        Ok(contacts)
    }

    pub async fn resolve_peer_by_username(
        &self,
        username: String,
    ) -> Result<Option<NativeSearchResult>> {
        let Some(peer) = self
            .client
            .resolve_username(username.trim_start_matches('@'))
            .await?
        else {
            return Ok(None);
        };
        let peer_ref = peer
            .to_ref()
            .await
            .ok_or_else(|| anyhow::anyhow!("Resolved peer has no reusable peer reference"))?;
        let native_peer = NativePeer::from_raw(&peer);
        self.seen_packed_chats_map
            .insert(native_peer.chat_id, peer_ref);
        let chat = NativeChat::from_raw(&peer).await;
        self.chats_map.insert(chat.chat_id, chat.clone());
        Ok(Some(NativeSearchResult {
            kind: NativeSearchResultKind::Chat,
            chat: Some(chat),
            peer: Some(native_peer.clone()),
            seen_chat: Some(native_peer),
            message: None,
        }))
    }

    pub async fn open_chat(&self, chat_id: i64) -> Result<NativeSearchResult> {
        let peer_ref = self.cached_peer_ref(chat_id)?;
        let peer = self.client.resolve_peer(peer_ref).await?;
        let native_peer = NativePeer::from_raw(&peer);
        let chat = self
            .chats_map
            .get(&chat_id)
            .map(|chat| chat.clone())
            .unwrap_or(NativeChat::from_raw(&peer).await);
        Ok(NativeSearchResult {
            kind: NativeSearchResultKind::Chat,
            chat: Some(chat),
            peer: Some(native_peer.clone()),
            seen_chat: Some(native_peer),
            message: None,
        })
    }

    pub async fn search_known_peers(
        &self,
        query: String,
        limit: Option<u32>,
    ) -> Result<NativeSearchPage> {
        let normalized = query.to_lowercase();
        let max_count = limit.unwrap_or(20).clamp(1, 100) as usize;
        let mut results = Vec::new();

        for peer_ref in self.seen_packed_chats_map.iter() {
            if results.len() >= max_count {
                break;
            }
            let chat_id = *peer_ref.key();
            let peer = self.client.resolve_peer(*peer_ref.value()).await?;
            let native_peer = NativePeer::from_raw(&peer);
            let haystack = format!(
                "{} {}",
                native_peer.full_name.to_lowercase(),
                native_peer
                    .username
                    .clone()
                    .unwrap_or_default()
                    .to_lowercase()
            );
            if !haystack.contains(&normalized) {
                continue;
            }
            let chat = self
                .chats_map
                .get(&chat_id)
                .map(|chat| chat.clone())
                .unwrap_or(NativeChat::from_raw(&peer).await);
            results.push(NativeSearchResult {
                kind: NativeSearchResultKind::Chat,
                chat: Some(chat),
                peer: Some(native_peer.clone()),
                seen_chat: Some(native_peer),
                message: None,
            });
        }

        Ok(NativeSearchPage {
            results,
            has_more: false,
        })
    }

    pub async fn set_dialog_flags(
        &self,
        chat_id: i64,
        flags: NativeDialogFlags,
    ) -> Result<NativeActionResult> {
        let peer_ref = self.cached_peer_ref(chat_id)?;

        if let Some(pinned) = flags.pinned {
            self.client
                .invoke(&tl::functions::messages::ToggleDialogPin {
                    pinned,
                    peer: tl::types::InputDialogPeer {
                        peer: peer_ref.into(),
                    }
                    .into(),
                })
                .await?;
            if let Some(mut chat) = self.chats_map.get_mut(&chat_id) {
                chat.pinned = pinned;
            }
        }

        if let Some(muted) = flags.muted {
            let mute_until = if muted { Some(i32::MAX) } else { Some(0) };
            self.client
                .invoke(&tl::functions::account::UpdateNotifySettings {
                    peer: tl::types::InputNotifyPeer {
                        peer: peer_ref.into(),
                    }
                    .into(),
                    settings: tl::types::InputPeerNotifySettings {
                        show_previews: None,
                        silent: Some(muted),
                        mute_until,
                        sound: None,
                        stories_muted: None,
                        stories_hide_sender: None,
                        stories_sound: None,
                    }
                    .into(),
                })
                .await?;
            if let Some(mut chat) = self.chats_map.get_mut(&chat_id) {
                chat.muted = muted;
            }
        }

        if let Some(archived) = flags.archived {
            self.client
                .invoke(&tl::functions::folders::EditPeerFolders {
                    folder_peers: vec![tl::types::InputFolderPeer {
                        peer: peer_ref.into(),
                        folder_id: if archived { 1 } else { 0 },
                    }
                    .into()],
                })
                .await?;
            if let Some(mut chat) = self.chats_map.get_mut(&chat_id) {
                chat.archived = archived;
            }
        }

        let chat = self.chats_map.get(&chat_id).map(|chat| chat.clone());
        let mut event = NativeEvent::new(NativeEventKind::DialogUpserted);
        event.chat = chat;
        self.emit_event(event).await;

        Ok(NativeActionResult::ok("dialog flags updated"))
    }

    // pub async fn load_chats_once_with_offset(&mut self, last_message_ids: HashMap<i64, i32>) -> Result<HashMap<i64, Chat>> {
    //     let packed_raw_chats = self.chats_map.iter().map(|(_, (raw_chat, _))| raw_chat.clone().pack()).collect::<Vec<_>>();
    //     for packed_raw_chat in packed_raw_chats {
    //         let mut total = 0;
    //         let mut last_chunk = false;
    //         let request = tl::functions::messages::GetHistory {
    //             peer: packed_raw_chat.to_input_peer(),
    //             offset_date: 0,
    //             offset_id: 0,
    //             limit: 100,
    //             max_id: 0,
    //             min_id: 0,
    //             hash: 0,
    //             add_offset: 0,
    //         };
    //         let (messages, users, chats, rate) = match self.client.invoke(&request).await? {
    //             Messages::Messages(m) => {
    //                 total = m.messages.len();
    //                 (m.messages, m.users, m.chats, None)
    //             }
    //             Messages::Slice(m) => {
    //                 last_chunk = m.messages.is_empty() || m.messages[0].id() <= 100;
    //                 (m.messages, m.users, m.chats, m.next_rate)
    //             }
    //             Messages::ChannelMessages(m) => {
    //                 last_chunk = m.messages.is_empty() || m.messages[0].id() <= limit;
    //                 total = m.count as usize;
    //                 (m.messages, m.users, m.chats, None)
    //             }
    //             Messages::NotModified(_) => {
    //                 panic!("API returned Messages::NotModified even though hash = 0")
    //             }
    //         };
    //     }
    //     Ok(chats)
    // }

    pub(crate) async fn download_sender_chat_photo(&self, sender: Option<Peer>) -> Result<()> {
        if let Some(sender) = sender {
            debug!(
                "Downloading profile photo for sender: {}, id: {}",
                sender.name().unwrap_or(""),
                sender.id().bare_id()
            );
            let profile_photo_path = profile_photo_path_and_count(sender.id().bare_id())?;
            if profile_photo_path.current.is_none() {
                self.download_chat_photo(&sender, true, &profile_photo_path)
                    .await
            } else {
                Ok(())
            }
        } else {
            Err(anyhow::anyhow!("No sender found!"))
        }
    }

    async fn check_chat_photo_downloading_and_wait(&self, chat_id: i64) -> bool {
        if self.profile_photo_downloading_set.contains(&chat_id) {
            error!("Profile photo for chat {} is already downloading!", chat_id);
            while self.profile_photo_downloading_set.contains(&chat_id) {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            return true;
        }
        false
    }

    pub(crate) async fn download_chat_photo(
        &self,
        chat: &Peer,
        big: bool,
        profile_photo_path: &ProfilePhotoPath,
    ) -> Result<()> {
        let ret: Result<()>;

        if self
            .check_chat_photo_downloading_and_wait(chat.id().bare_id())
            .await
        {
            ret = Ok(());
        } else {
            debug!(
                "download_chat_photo Downloading profile photo for chat {}",
                chat.name().unwrap_or("")
            );
            let profile_photo = chat.photo(big).await;
            if let Some(profile_photo) = profile_photo {
                self.profile_photo_downloading_set
                    .insert(chat.id().bare_id());

                {
                    // TODO: invoke this in high frequency may cause FLOOD_WAIT
                    // here, besides using the semaphore to limit the maximum number of concurrent downloads,
                    // we also need to consider limiting the frequency
                    // debug!("download_chat_photo acquiring global_semaphore");
                    // let _permit = self.global_semaphore.acquire().await?;
                    // debug!("download_chat_photo acquired global_semaphore");
                    debug!(
                        "download_chat_photo Downloading profile photo for chat {} at {:?}",
                        chat.name().unwrap_or(""),
                        profile_photo_path.current
                    );
                    self.client
                        .download_media(&profile_photo, &profile_photo_path.next)
                        .await?;
                }

                self.profile_photo_downloading_set
                    .remove(&chat.id().bare_id());
                debug!(
                    "download_chat_photo Downloaded profile photo for chat {} at {}",
                    chat.name().unwrap_or(""),
                    profile_photo_path.next
                );
                ret = Ok(());
            } else {
                ret = Err(anyhow::anyhow!(
                    "download_chat_photo No profile photo found for chat {}",
                    chat.name().unwrap_or("")
                ));
            }
        }

        ret
    }

    pub async fn download_chat_photo_by_chat_id(
        &mut self,
        chat_id: i64,
        big: bool,
    ) -> Result<String> {
        debug!(
            "download_chat_photo_by_chat_id Downloading chat photo for chat {}",
            chat_id
        );
        let profile_photo_path = profile_photo_path_and_count(chat_id)?;
        debug!(
            "download_chat_photo_by_chat_id profile_photo_path: {:?}",
            profile_photo_path
        );
        if self.check_chat_photo_downloading_and_wait(chat_id).await {
            debug!(
                "download_chat_photo_by_chat_id Chat photo for chat {} is already downloaded!",
                chat_id
            );
            return Ok(profile_photo_path.next);
        }
        debug!("download_chat_photo_by_chat_id Chat photo for chat {} is not downloaded yet, getting its packed_chat!", chat_id);
        let chat = self.seen_packed_chats_map.get(&chat_id);
        debug!("download_chat_photo_by_chat_id packed_chat got: {:?}", chat);
        if chat.is_none() {
            error!("download_chat_photo_by_chat_id Chat with id {} not found in known_packed_chats_map!", chat_id);
            return Err(anyhow::anyhow!("download_chat_photo_by_chat_id Chat with id {} not found in known_packed_chats_map!", chat_id));
        }

        let chat = {
            // TODO: invoke this in high frequency may cause FLOOD_WAIT
            // here, besides using the semaphore to limit the maximum number of concurrent downloads,
            // we also need to consider limiting the frequency
            // debug!("download_chat_photo_by_chat_id acquiring global_semaphore");
            // let _permit = self.global_semaphore.acquire().await?;
            // debug!("download_chat_photo_by_chat_id acquired global_semaphore");
            debug!(
                "download_chat_photo_by_chat_id unpacking chat for chat {}",
                chat_id
            );
            self.client.resolve_peer(*chat.unwrap()).await?
        };
        debug!(
            "download_chat_photo_by_chat_id unpacked chat got: {:?}",
            chat
        );
        self.download_chat_photo(&chat, big, &profile_photo_path)
            .await?;
        debug!(
            "download_chat_photo_by_chat_id Chat photo for chat {} downloaded at {}",
            chat_id, profile_photo_path.next
        );
        Ok(profile_photo_path.next)
    }

    pub async fn get_chat_photo_thumb_by_chat_id(&self, chat_id: i64) -> Result<Option<Vec<u8>>> {
        let packed_chat = self.seen_packed_chats_map.get(&chat_id);
        match packed_chat {
            Some(packed_chat) => {
                let chat = self.client.resolve_peer(*packed_chat).await?;

                match chat {
                    Peer::User(user) => Ok(user
                        .photo()
                        .map(|photo| photo.stripped_thumb.clone())
                        .unwrap_or(None)),
                    Peer::Group(group) => Ok(group
                        .photo()
                        .map(|photo| photo.stripped_thumb.clone())
                        .unwrap_or(None)),
                    Peer::Channel(channel) => Ok(channel
                        .photo()
                        .map(|photo| photo.stripped_thumb.clone())
                        .unwrap_or(None)),
                }
            }
            None => Err(anyhow::anyhow!(
                "Chat with id {} not found in seen_packed_chats_map!",
                chat_id
            )),
        }
    }

    /// Load chat participants.
    pub async fn get_participants(
        &self,
        chat_id: i64,
    ) -> Result<Vec<crate::tg::types::NativeParticipant>> {
        let packed_chat = self.seen_packed_chats_map.get(&chat_id);
        match packed_chat {
            Some(packed_chat) => {
                let mut participants = Vec::new();
                let mut iter = self.client.iter_participants(*packed_chat);
                while let Some(participant) = iter.next().await? {
                    participants.push(crate::tg::types::NativeParticipant::from_raw(&participant));
                }
                debug!(
                    "get_participants: chat_id={}, count={}",
                    chat_id,
                    participants.len()
                );
                Ok(participants)
            }
            None => Err(anyhow::anyhow!(
                "Chat with id {} not found in seen_packed_chats_map!",
                chat_id
            )),
        }
    }
}

fn dialog_matches_folder(dialog: &tl::enums::Dialog, requested_folder_id: Option<i32>) -> bool {
    let Some(requested_folder_id) = requested_folder_id else {
        return true;
    };
    let folder_id = match dialog {
        tl::enums::Dialog::Dialog(raw) => raw.folder_id.unwrap_or(0),
        tl::enums::Dialog::Folder(_) => 0,
    };
    folder_id == requested_folder_id
}
