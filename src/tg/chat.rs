use crate::tg::types::{
    peer_ref_from_string, ChatType, NativeChat, NativePackedChat, NativeRawMessage, NativeSeenChat,
};
use crate::tg::utils::{get_profile_photo_path_and_count, ProfilePhotoPath};
use crate::tg::Backend;
use anyhow::Result;
use dashmap::{DashMap as HashMap, DashSet as HashSet};
use grammers_client::media::Downloadable;
use grammers_client::peer::Peer;
use grammers_client::tl;
use grammers_session::types::PeerRef;
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

    /// `load_chats_with_offset`
    /// 遍历所有对话，为每个聊天加载新消息（基于提供的偏移量），并通过回调通知外部代码更新缓存和 UI。
    ///
    /// ## 参数
    /// - `last_message_ids`: 可选的 `HashMap<i64, i32>`，键为聊天 ID（`i64`），值为最后已知消息 ID（`i32`）。
    ///   用于指定每个聊天的消息加载起点。如果为 `None`，则从最新消息开始加载。
    ///
    /// ## 返回值
    /// 返回 `Result<(), anyhow::Error>`：成功时返回 `Ok(())`，失败时返回包含错误信息的 `Err`。
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
            debug!("Loading chat:{}, chat_type: {:?}, forum: {}", chat.name, chat.chat_type, chat.forum);
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
            self.seen_packed_chats_map.insert(chat.chat_id, packed_chat);
            let native_seen_chat = NativeSeenChat::from_raw(raw_chat);
            self.cache_seen_chat_callback
                .as_ref()
                .unwrap()
                .call(Ok(native_seen_chat.clone()), ThreadsafeFunctionCallMode::NonBlocking);
            self.chats_map.insert(raw_chat.id().bare_id(), chat.clone());
            debug!("before update_chat_callback call: chat name: {}", chat.name);
            self.update_chat_callback.as_ref().unwrap().call(
                Ok((
                    native_seen_chat,
                    chat,
                    sorted_messages.values().cloned().collect(),
                )),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
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
            let profile_photo_path = get_profile_photo_path_and_count(sender.id().bare_id())?;
            if profile_photo_path.current.is_none() {
                self.download_chat_photo(&sender, true, &profile_photo_path).await
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

    pub(crate) async fn download_chat_photo(&self, chat: &Peer, big: bool,
                                            profile_photo_path: &ProfilePhotoPath) -> Result<()> {
        let ret: Result<()>;

        if self.check_chat_photo_downloading_and_wait(chat.id().bare_id()).await {
            ret = Ok(());
        } else {
            debug!("download_chat_photo Downloading profile photo for chat {}", chat.name().unwrap_or(""));
            let profile_photo = chat.photo(big).await;
            if let Some(profile_photo) = profile_photo {
                self.profile_photo_downloading_set.insert(chat.id().bare_id());

                {
                    // TODO: invoke this in high frequency may cause FLOOD_WAIT
                    // here, besides using the semaphore to limit the maximum number of concurrent downloads,
                    // we also need to consider limiting the frequency
                    // debug!("download_chat_photo acquiring global_semaphore");
                    // let _permit = self.global_semaphore.acquire().await?;
                    // debug!("download_chat_photo acquired global_semaphore");
                    debug!("download_chat_photo Downloading profile photo for chat {} at {:?}", chat.name().unwrap_or(""), profile_photo_path.current);
                    self.client.download_media(&profile_photo, &profile_photo_path.next).await?;
                }

                self.profile_photo_downloading_set.remove(&chat.id().bare_id());
                debug!("download_chat_photo Downloaded profile photo for chat {} at {}", chat.name().unwrap_or(""), profile_photo_path.next);
                ret = Ok(());
            } else {
                ret = Err(anyhow::anyhow!("download_chat_photo No profile photo found for chat {}", chat.name().unwrap_or("")));
            }
        }

        ret
    }

    pub async fn download_chat_photo_by_chat_id(&mut self, chat_id: i64, big: bool) -> Result<String> {
        debug!("download_chat_photo_by_chat_id Downloading chat photo for chat {}", chat_id);
        let profile_photo_path = get_profile_photo_path_and_count(chat_id)?;
        debug!("download_chat_photo_by_chat_id profile_photo_path: {:?}", profile_photo_path);
        if self.check_chat_photo_downloading_and_wait(chat_id).await {
            debug!("download_chat_photo_by_chat_id Chat photo for chat {} is already downloaded!", chat_id);
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
            debug!("download_chat_photo_by_chat_id unpacking chat for chat {}", chat_id);
            self.client.resolve_peer(*chat.unwrap()).await?
        };
        debug!("download_chat_photo_by_chat_id unpacked chat got: {:?}", chat);
        self.download_chat_photo(&chat, big, &profile_photo_path).await?;
        debug!("download_chat_photo_by_chat_id Chat photo for chat {} downloaded at {}", chat_id, profile_photo_path.next);
        Ok(profile_photo_path.next)
    }

    pub async fn get_chat_photo_thumb_by_chat_id(&self, chat_id: i64) -> Result<Option<Vec<u8>>> {
        let packed_chat = self.seen_packed_chats_map.get(&chat_id);
        match packed_chat {
            Some(packed_chat) => {
                let chat = self.client.resolve_peer(*packed_chat).await?;

                match chat {
                    Peer::User(user) => { Ok(user.photo().map(|photo| photo.stripped_thumb.clone()).unwrap_or(None)) }
                    Peer::Group(group) => { Ok(group.photo().map(|photo| photo.stripped_thumb.clone()).unwrap_or(None)) }
                    Peer::Channel(channel) => { Ok(channel.photo().map(|photo| photo.stripped_thumb.clone()).unwrap_or(None)) }
                }
            }
            None => {
                Err(anyhow::anyhow!("Chat with id {} not found in seen_packed_chats_map!", chat_id))
            }
        }
    }

    /// 获取聊天参与者列表
    ///
    /// ## 参数
    /// - `chat_id`: 聊天 ID
    ///
    /// ## 返回值
    /// 返回 `Result<Vec<NativeParticipant>, anyhow::Error>`：成功时返回参与者列表，失败时返回错误。
    pub async fn get_participants(&self, chat_id: i64) -> Result<Vec<crate::tg::types::NativeParticipant>> {
        let packed_chat = self.seen_packed_chats_map.get(&chat_id);
        match packed_chat {
            Some(packed_chat) => {
                let mut participants = Vec::new();
                let mut iter = self.client.iter_participants(*packed_chat);
                while let Some(participant) = iter.next().await? {
                    participants.push(crate::tg::types::NativeParticipant::from_raw(&participant));
                }
                debug!("get_participants: chat_id={}, count={}", chat_id, participants.len());
                Ok(participants)
            }
            None => {
                Err(anyhow::anyhow!("Chat with id {} not found in seen_packed_chats_map!", chat_id))
            }
        }
    }
}
