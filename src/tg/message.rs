use crate::tg::types::{MediaType, NativeChat, NativeMessage, NativeSeenChat};
use crate::tg::utils::{get_download_dir, get_media_path, get_profile_photo_path_and_count};
use crate::tg::Backend;
use anyhow::Result;
use grammers_client::client::messages::MessageIter;
use grammers_client::types::Message;
use grammers_client::{grammers_tl_types as tl, InputMedia};
use grammers_tl_types::enums::InputMessage;
use log::{debug, error};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio;

impl Backend {
    pub(crate) async fn incoming_message_handler(&'static self, raw_message: &Message) {
        self.seen_packed_chats_map.insert(raw_message.chat().id(), raw_message.chat().pack());

        if let Some(sender) = raw_message.sender() {
            self.seen_packed_chats_map.insert(sender.id(), sender.pack());
        }

        tokio::spawn(self.download_sender_chat_photo(raw_message.sender()));
        let old_chat = self.chats_map.get_mut(&raw_message.chat().id());
        match old_chat {
            Some(mut old_chat) => {
                debug!("tg::Backend::run() New message in chat {}", old_chat.name);
                let message = NativeMessage::from_raw(&raw_message);
                debug!("tg::Backend::run() message: {:?}", message);
                old_chat.last_message_sender_name = raw_message.sender().map(|s| s.name().to_string()).unwrap_or("".to_string());
                let raw_message = &raw_message.raw;
                old_chat.last_message_id = raw_message.id;
                old_chat.last_message_text = raw_message.message.clone();
                old_chat.last_message_timestamp = raw_message.date as i64;
                debug!("tg::Backend::run() old_chat: {:?}", old_chat);
                drop(old_chat);
            }
            None => {
                drop(old_chat);
                let raw_chat = raw_message.chat();
                tokio::spawn(self.download_sender_chat_photo(raw_message.sender()));
                debug!(
                                "New message in unknown chat {}: {}",
                                raw_chat.name(),
                                raw_message.text()
                            );
                // let chat = Chat::from_raw(raw_chat.clone()).await;
                let message = NativeMessage::from_raw(&raw_message);
                // let raw_message = raw_message.raw;
                let mut chat = NativeChat::from_raw(&raw_chat).await; // TODO: we current assume that this chat is not possibly a pinned chat
                chat.last_message_id = message.message_id;
                chat.last_message_sender_name = message.sender_name.clone();
                chat.last_message_text = message.text.clone();
                chat.last_message_timestamp = message.timestamp;
                debug!("Chat: {:?}", chat);
                debug!("Message: {:?}", message);
                self.chats_map.insert(raw_chat.id(), chat.clone());
                debug!("chats_map updated!");
            }
        };
    }

    pub(crate) async fn load_messages_from_iter(
        &'static self,
        message_iter: MessageIter,
        last_message_id: Option<i32>,
    ) -> Result<BTreeMap<i32, NativeMessage>> {
        debug!("Loading messages after {:?}", last_message_id);
        let mut sorted_messages = BTreeMap::new();
        let mut message_iter = message_iter.limit(100);
        let now = chrono::Utc::now().timestamp();
        message_iter = message_iter.max_date(now as i32); // TODO: What does this do?
        while let Some(raw_message) = message_iter.next().await? {
            if let Some(last_message_id) = last_message_id {
                if raw_message.id() <= last_message_id {
                    break;
                }
            }
            let sender = raw_message.sender();
            // tokio::spawn(self.download_sender_chat_photo(sender.clone()));
            if let Some(sender) = sender {
                self.seen_packed_chats_map.insert(sender.id(), sender.pack());
            }
            let message = NativeMessage::from_raw(&raw_message);
            sorted_messages.insert(message.message_id, message);
        }
        Ok(sorted_messages)
    }

    pub(crate) async fn get_sorted_messages(
        &self,
        chat: &grammers_client::types::Chat,
    ) -> Result<Vec<NativeMessage>> {
        let mut sorted_messages: Vec<NativeMessage> = Vec::new();
        let mut messages = self.client.iter_messages(chat).limit(5);
        while let Some(message) = messages.next().await? {
            sorted_messages.push(NativeMessage::from_raw(&message));
        }
        sorted_messages.reverse();
        Ok(sorted_messages)
    }

    pub async fn send_message(&self, chat_id: i64, text: String, medias: Option<Vec<String>>)
                              -> Result<Vec<NativeMessage>> {
        debug!("Sending message to chat {}: {}", chat_id, text);
        // let chats_map = self.chats_map.read().await;
        let packed_chat = self.seen_packed_chats_map.get(&chat_id).unwrap_or_else(|| {
            error!("Chat with id {} not found in chats_map!", chat_id);
            panic!("Chat with id {} not found in chats_map!", chat_id)
        }).clone();
        use grammers_client::InputMessage;
        let mut album = Vec::new();

        if let Some(medias) = medias {
            debug!("Sending media message with {} messages and text {}", medias.len(), text);
            for (index, media) in medias.iter().enumerate() {
                let raw_file = std::fs::read(media)?;
                let len = raw_file.len();
                // use std::io::Cursor to keep track of the progress
                let mut stream = std::io::Cursor::new(raw_file);

                let file_name = std::path::Path::new(media).file_name().unwrap().to_str().unwrap().split("/").last().unwrap();
                let uploaded_file = self.client.upload_stream(&mut stream, len, file_name.to_string()).await;
                match uploaded_file {
                    Ok(file) => {
                        let input_media = if index == 0 {
                            InputMedia::caption(text.clone())
                        } else {
                            InputMedia::caption("")
                        }.photo(file);
                        album.push(input_media);
                    }
                    Err(e) => {
                        error!("Failed to upload media: {e}");
                        return Err(anyhow::Error::from(e));
                    }
                }
            }
            let album_sent = self.client.send_album(packed_chat, album).await?;
            debug!("Album sent: {:?}", album_sent);
            Ok(album_sent.iter().map(|m| NativeMessage::from_raw(m.as_ref().unwrap())).collect())
        } else {
            debug!("Sending text message: {}", text);
            let message_sent = self
                .client
                .send_message(packed_chat, InputMessage::text(text.clone()))
                .await;
            debug!("send_message returned: {:?}", message_sent);
            match message_sent {
                Err(e) => {
                    error!("Failed to send message: {e}");
                    Err(anyhow::Error::from(e))
                }
                Ok(message_sent) => {
                    debug!("Message sent: {:?}", message_sent);
                    Ok(vec![NativeMessage::from_raw(&message_sent)])
                }
            }
        }
    }

    pub async fn download_media_from_message(
        &self,
        chat_id: i64,
        message_id: i32,
    ) -> Result<String> {
        debug!("Downloading media from message with id {}", message_id);
        // let messages_map_pair = self.messages_of_chats.read().await;
        // let messages_map_pair = messages_map_pair.get(&chat_id).unwrap();
        let packed_chat = self.seen_packed_chats_map.get(&chat_id).unwrap_or_else(|| {
            error!("Chat with id {} not found in chats_map!", chat_id);
            panic!("Chat with id {} not found in chats_map!", chat_id)
        }).clone();
        let message = self.client.get_messages_by_id(packed_chat, &[message_id]).await;
        let mut message = match message {
            Ok(message) => message,
            Err(e) => {
                error!("Failed to get message: {e}");
                return Err(anyhow::Error::from(e));
            }
        };

        if message.is_empty() {
            return Err(anyhow::anyhow!("Message not found!"));
        }
        let message = match message.pop() {
            Some(message) => message.unwrap_or_else(|| {
                error!("Message not found!");
                panic!("Message not found!");
            }),
            None => {
                return Err(anyhow::anyhow!("Message not found!"));
            }
        };

        let download_dir = get_download_dir(chat_id);
        let download_path = get_media_path(chat_id, message_id);

        // check if the file exists
        if std::path::Path::new(&download_path).exists() {
            debug!("Media already downloaded!");
            Ok(download_path)
        } else {
            if !std::path::Path::new(&download_dir).exists() {
                std::fs::create_dir_all(download_dir)?;
            }
            match message.download_media(download_path.clone()).await {
                Ok(success) => {
                    if success {
                        debug!("Media downloaded successfully!");
                        Ok(download_path)
                    } else {
                        error!("Failed to download media!");
                        Err(anyhow::anyhow!(
                            "Failed to download media: {success} returned!"
                        ))
                    }
                }
                Err(e) => {
                    error!("Failed to download media: {e}");
                    Err(anyhow::Error::from(e))
                }
            }
        }
    }
}