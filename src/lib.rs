#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

mod tg;

use crate::tg::types::{ChatType, NativePackedChat, NativeSeenChat};
use crate::tg::types::{LoginState, NativeChat, NativeMessage};
use anyhow::{Error, Result};
use grammers_session::PackedChat;
use log::{debug, error, LevelFilter};
use tokio;

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::sleep;
use tokio_util::sync::CancellationToken;


pub async fn is_logged_in() -> bool {
    tg::Backend::get_instance().await.is_logged_in().await
}


pub async fn register_device(token: String) -> Result<bool> {
    tg::Backend::get_instance()
        .await
        .register_device(token)
        .await
}


pub async fn login(phone_number: String) -> Result<LoginState> {
    tg::Backend::get_instance()
        .await
        .login_with_phone(phone_number)
        .await
}


pub async fn verify_code(code: String) -> Result<LoginState> {
    tg::Backend::get_instance()
        .await
        .provide_verify_code(code)
        .await
}


pub async fn password(password: String) -> Result<LoginState> {
    tg::Backend::get_instance()
        .await
        .provide_password(password)
        .await
}


pub async fn sign_out() -> bool {
    tg::Backend::get_instance()
        .await
        .sign_out()
        .await
}


pub async fn run() -> Result<tokio::task::JoinHandle<Result<()>>> {
    debug!("homo::run() called");
    if let Some(handler) = tg::Backend::get_instance().await.get_run_handler().await {
        if !handler.is_finished() {
            return Err(Error::msg("homo::run() already running"));
        }
    };
    let handler = tokio::spawn(tg::Backend::get_instance().await.run());

    Ok(handler)
}

pub async fn load_chats() -> Result<()> {
    let backend = tg::Backend::get_instance().await;
    backend
        .load_chats_with_offset(None)
        .await
}


pub async fn get_me() -> Result<NativeSeenChat> {
    let backend = tg::Backend::get_instance().await;
    backend.get_me().await
}


pub async fn load_chats_with_offset(last_message_ids: HashMap<String, i32>) -> Result<()> {
    let backend = tg::Backend::get_instance().await;
    let dash_map = dashmap::DashMap::with_capacity(last_message_ids.len());
    for (chat_id, last_message_id) in last_message_ids {
        dash_map.insert(chat_id.parse().unwrap(), last_message_id);
    }
    backend
        .load_chats_with_offset(Some(dash_map))
        .await
}

pub async fn get_chats_map() -> &'static DashMap<i64, NativeChat> {
    tg::Backend::get_instance().await.get_chats_map().await
}

pub async fn sync_caches_from_local_db(
    packed_chats: Vec<NativePackedChat>,
    // seen_chats: Vec<NativeSeenChat>,
    chats: Vec<NativeChat>,
) -> Result<()> {
    let backend = tg::Backend::get_instance().await;
    backend
        .sync_caches_from_local_db(packed_chats, chats)
        .await
}


pub async fn send_message(chat_id: i64, text: String, medias: Option<Vec<String>>) -> Result<Vec<NativeMessage>> {
    let backend = tg::Backend::get_instance().await;
    backend.send_message(chat_id, text, medias).await
}


pub async fn download_media_from_message(chat_id: i64, message_id: i32) -> Result<String> {
    let backend = tg::Backend::get_instance().await;
    backend
        .download_media_from_message(chat_id, message_id)
        .await
}


pub async fn download_profile_photo(chat_id: i64) -> Result<String> {
    let backend = tg::Backend::get_instance().await;
    backend
        .download_chat_photo_by_chat_id(chat_id, true)
        .await
}


pub async fn get_chat_photo_thumb(chat_id: i64) -> Result<Option<Vec<u8>>> {
    let backend = tg::Backend::get_instance().await;
    backend.get_chat_photo_thumb_by_chat_id(chat_id).await
}


pub async fn reconnect() -> Result<bool> {
    Ok(tg::Backend::get_instance().await.reconnect().await)
}