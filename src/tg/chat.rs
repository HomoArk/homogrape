use crate::tg::types::{ChatType, NativeChat, NativePackedChat, NativeRawMessage, NativeSeenChat};
use crate::tg::utils::{get_profile_photo_path_and_count, ProfilePhotoPath};
use crate::tg::Backend;
use anyhow::Result;
use dashmap::{DashMap as HashMap, DashSet as HashSet};
use grammers_client::types::{Chat, Downloadable};
use grammers_client::{grammers_tl_types as tl, InitParams};
use grammers_session::PackedChat;
use grammers_tl_types::Serializable;
use log::{debug, error};
use napi_ohos::bindgen_prelude::Buffer;
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

    pub async fn load_chats_with_offset(
        &'static mut self,
        last_message_ids: Option<HashMap<i64, i32>>,
    ) -> Result<()> {
        let mut dialog_iter = self.client.iter_dialogs();
        while let Some(dialog) = dialog_iter.next().await? {
            tokio::spawn(self.save_session());
            let dialog = Box::leak(Box::new(dialog));
            let raw_chat = dialog.chat();
            let mut chat = NativeChat::from_raw(raw_chat).await;
            debug!("Loading chat:{}, chat_type: {:?}, forum: {}", chat.name, chat.chat_type, chat.forum);
            let packed_chat = raw_chat.pack();
            // let profile_photo_path = Box::leak(Box::new(get_profile_photo_path_and_count(raw_chat.id())?));
            // if profile_photo_path.current.is_none() {
            //     let packed_chat = Box::leak(Box::new(packed_chat));
            //     tokio::spawn(
            //         self.download_chat_photo(packed_chat, true, profile_photo_path)
            //     );
            // }
            chat.pinned = dialog.raw.pinned();
            let last_message_id = if let Some(last_message_ids) = last_message_ids.as_ref() {
                last_message_ids.get(&chat.chat_id).and_then(|id| Some(*id))
            } else {
                None
            };
            let message_iter = self.client.iter_messages(raw_chat);
            debug!("Loading chat: {} after {:?}", chat.name, last_message_id);
            let sorted_messages =
                self.load_messages_from_iter(message_iter, last_message_id).await?;
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
            self.seen_packed_chats_map.insert(packed_chat.id, packed_chat)
                .expect(format!("seen_packed_chats_map insert {} failed", packed_chat.id).as_str());
            let native_seen_chat = NativeSeenChat::from_raw(raw_chat);
            self.cache_seen_chat_callback
                .as_ref()
                .unwrap()
                .call(Ok(native_seen_chat.clone()), ThreadsafeFunctionCallMode::NonBlocking);
            self.chats_map.insert(raw_chat.id(), chat.clone())
                .expect(format!("chats_map insert {} failed", raw_chat.id()).as_str());
            self.update_chat_callback.as_ref().unwrap().call(
                Ok((
                    native_seen_chat,
                    chat,
                    sorted_messages.values().cloned().collect(),
                )),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }
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
            self.seen_packed_chats_map.insert(packed_chat.chat_id, PackedChat::from_hex(packed_chat.packed_chat.as_str())?);
        }
        for chat in chats.iter() {
            self.chats_map.insert(chat.chat_id, chat.clone());
        }
        Ok(())
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

    pub(crate) async fn download_sender_chat_photo(&self, sender: Option<grammers_client::types::Chat>) -> Result<()> {
        if let Some(sender) = sender {
            debug!("Downloading profile photo for sender: {}, id: {}", sender.name(), sender.id());
            let profile_photo_path = get_profile_photo_path_and_count(sender.id())?;
            if profile_photo_path.current.is_none() {
                self.download_chat_photo(&sender.pack(), true, &profile_photo_path).await
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
                tokio::time::sleep(std::time::Duration::from_secs(200)).await;
            }
            return true;
        }
        false
    }

    pub(crate) async fn download_chat_photo(&self, chat: &PackedChat, big: bool,
                                            profile_photo_path: &ProfilePhotoPath) -> Result<()> {
        // TODO: invoke this in high frequency may cause FLOOD_WAIT
        // here, besides using the semaphore to limit the maximum number of concurrent downloads,
        // we also need to consider limiting the frequency of unpack_chat()
        let _permit = self.global_semaphore.acquire().await?;
        let chat = self.client.unpack_chat(*chat).await?;
        let ret: Result<()>;

        if self.check_chat_photo_downloading_and_wait(chat.id()).await {
            ret = Ok(());
        } else {
            debug!("Downloading profile photo for chat {}", chat.name());
            let profile_photo = chat.photo_downloadable(big);
            if let Some(profile_photo) = profile_photo {
                self.profile_photo_downloading_set.insert(chat.id());
                self.client.download_media(&profile_photo, &profile_photo_path.next).await?;
                self.profile_photo_downloading_set.remove(&chat.id());
                debug!("Downloaded profile photo for chat {} at {}", chat.name(), profile_photo_path.next);
                ret = Ok(());
            } else {
                ret = Err(anyhow::anyhow!("No profile photo found for chat {}", chat.name()));
            }
        }

        drop(_permit);
        ret
    }

    pub async fn download_chat_photo_by_chat_id(&mut self, chat_id: i64, big: bool) -> Result<String> {
        debug!("Downloading chat photo for chat {}", chat_id);
        let profile_photo_path = get_profile_photo_path_and_count(chat_id)?;
        debug!("profile_photo_path: {:?}", profile_photo_path);
        if self.check_chat_photo_downloading_and_wait(chat_id).await {
            return Ok(profile_photo_path.next);
        }
        let chat = self.seen_packed_chats_map.get(&chat_id);
        if chat.is_none() {
            error!("Chat with id {} not found in known_packed_chats_map!", chat_id);
            return Err(anyhow::anyhow!("Chat with id {} not found in known_packed_chats_map!", chat_id));
        }
        self.download_chat_photo(&chat.unwrap(), big, &profile_photo_path).await?;
        Ok(profile_photo_path.next)
    }

    pub async fn get_chat_photo_thumb_by_chat_id(&self, chat_id: i64) -> Result<Option<Vec<u8>>> {
        let packed_chat = self.seen_packed_chats_map.get(&chat_id);
        match packed_chat {
            Some(packed_chat) => {
                let chat = self.client.unpack_chat(*packed_chat).await?;

                match chat {
                    Chat::User(user) => { Ok(user.photo().map(|photo| photo.stripped_thumb.clone()).unwrap_or(None)) }
                    Chat::Group(group) => { Ok(group.photo().map(|photo| photo.stripped_thumb.clone()).unwrap_or(None)) }
                    Chat::Channel(channel) => { Ok(channel.photo().map(|photo| photo.stripped_thumb.clone()).unwrap_or(None)) }
                }
            }
            None => {
                Err(anyhow::anyhow!("Chat with id {} not found in seen_packed_chats_map!", chat_id))
            }
        }
    }
}