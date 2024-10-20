use crate::tg::types::{NativeChat, NativeMessage, NativeSeenChat};
use crate::tg::utils::get_profile_photo_path_and_count;
use crate::tg::{Backend, SESSION_FILE};
use anyhow::Result;
use grammers_client::Update;
use grammers_session::Session;
use log::{debug, info};
use std::collections::BTreeMap;
use tokio;

impl Backend {
    pub async fn run(&'static self) -> Result<()> {
        loop {
            debug!("tg::Backend::run() Waiting for next update...");
            match self.client.next_update().await? {
                Update::NewMessage(ref raw_message) => {
                    self.incoming_message_handler(raw_message).await;
                }
                _ => info!("Other update are not implemented currently."),
            }
            tokio::spawn(self.save_session());
        }
    }

    pub async fn set_run_handler(&mut self, handler: tokio::task::JoinHandle<Result<()>>) {
        self.run_handler.replace(handler);
    }

    pub async fn get_run_handler(&mut self) -> Option<tokio::task::JoinHandle<Result<()>>> {
        self.run_handler.take()
    }
}