use crate::tg::types::{NativeDeletedMessages, NativeEvent, NativeEventKind, NativeMessage};
use crate::tg::utils::get_profile_photo_path_and_count;
use crate::tg::{Backend, SESSION_FILE};
use anyhow::Result;
use grammers_client::update::Update;
use grammers_session::Session;
use log::{debug, info};
use napi_ohos::threadsafe_function::ThreadsafeFunctionCallMode;
use napi_ohos::tokio;
use ohos_hilog_binding::debug;
use std::collections::BTreeMap;

impl Backend {
    pub async fn run(&'static self) -> Result<()> {
        loop {
            debug!("tg::Backend::run() Waiting for next update...");
            match self.updates.lock().await.next().await? {
                Update::NewMessage(ref raw_message) => {
                    self.incoming_message_handler(raw_message).await;
                }
                Update::MessageEdited(ref raw_message) => {
                    let message = NativeMessage::from_raw(raw_message);
                    let mut event = NativeEvent::new(NativeEventKind::MessageEdited);
                    event.message = Some(message);
                    self.emit_event(event).await;
                }
                Update::MessageDeleted(ref deleted) => {
                    let mut event = NativeEvent::new(NativeEventKind::MessagesDeleted);
                    event.deleted_messages = Some(NativeDeletedMessages {
                        chat_id: deleted.channel_id(),
                        message_ids: deleted.messages().to_vec(),
                    });
                    self.emit_event(event).await;
                }
                _ => {
                    let event = NativeEvent::new(NativeEventKind::UnknownUpdate);
                    self.emit_event(event).await;
                    info!("Update type is not implemented yet.");
                }
            }
            tokio::spawn(self.save_session());
        }
    }

    pub async fn set_run_handler(&self, handler: tokio::task::JoinHandle<Result<()>>) {
        self.run_handler.lock().await.replace(handler);
    }

    pub async fn is_run_handler_active(&self) -> bool {
        self.run_handler
            .lock()
            .await
            .as_ref()
            .map(|handler| !handler.is_finished())
            .unwrap_or(false)
    }

    pub async fn abort_run_handler(&self) -> bool {
        if let Some(handler) = self.run_handler.lock().await.take() {
            handler.abort();
            true
        } else {
            false
        }
    }
}
