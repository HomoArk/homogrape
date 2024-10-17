#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

mod tg;

use crate::tg::types::{CacheSeenChatCallback, ChatType, IncomingMessageCallback, LoadChatsCallback, NativePackedChat, NativeSeenChat, UpdateChatCallback, UpdateUploadProgressCallback};
use crate::tg::types::{LoginState, NativeChat, NativeMessage};
use grammers_session::PackedChat;
use hilog::{Builder, LogDomain};
use log::{debug, error, LevelFilter};
use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::*;
use napi_ohos::threadsafe_function::ThreadsafeFunction;
use napi_ohos::tokio::sync::Mutex;
use napi_ohos::{tokio, Error};
use std::collections::HashMap;
use std::thread::sleep;
use tokio_util::sync::CancellationToken;

type Result<T> = std::result::Result<T, Error>;

#[napi]
pub async fn is_logged_in() -> bool {
    tg::Backend::get_instance().await.is_logged_in().await
}

#[napi]
pub async fn register_device(token: String) -> Result<bool> {
    tg::Backend::get_instance()
        .await
        .register_device(token)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub async fn login(phone_number: String) -> Result<LoginState> {
    tg::Backend::get_instance()
        .await
        .login_with_phone(phone_number)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub async fn verify_code(code: String) -> Result<LoginState> {
    tg::Backend::get_instance()
        .await
        .provide_verify_code(code)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub async fn password(password: String) -> Result<LoginState> {
    tg::Backend::get_instance()
        .await
        .provide_password(password)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub async fn sign_out() -> bool {
    tg::Backend::get_instance()
        .await
        .sign_out()
        .await
}

#[napi]
pub async fn run() -> Result<()> {
    debug!("homo::run() called");
    if let Some(handler) = tg::Backend::get_instance().await.get_run_handler().await {
        if !handler.is_finished() {
            return Err(Error::from_reason("homo::tg::Backend::run() already "));
        }
    }
    let handler = tokio::spawn(tg::Backend::get_instance().await.run());
    tg::Backend::get_instance()
        .await
        .set_run_handler(handler)
        .await;
    Ok(())
}

#[napi]
pub async fn stop() {
    debug!("homo::stop() called");
    match tg::Backend::get_instance().await.get_run_handler().await {
        Some(handler) => {
            handler.abort();
            debug!("homo::tg::Backend::run_handler aborted")
            // no need to drop manually
            // std::mem::drop(handler);
        }
        None => {
            error!("homo::tg::Backend::run() already stopped");
        }
    }
}

#[napi]
pub async fn register_update_chat_callback(cb: UpdateChatCallback) {
    let backend = tg::Backend::get_instance().await;
    backend.register_update_chat_callback(cb);
}

#[napi]
pub async fn register_cache_seen_chat_callback(cb: CacheSeenChatCallback) {
    let backend = tg::Backend::get_instance().await;
    backend.register_cache_seen_chat_callback(cb);
}

#[napi]
pub async fn register_load_chats_callback(cb: LoadChatsCallback) {
    let backend = tg::Backend::get_instance().await;
    backend.register_load_chats_callback(cb);
}

#[napi]
pub async fn register_incoming_message_callback(cb: IncomingMessageCallback) {
    let backend = tg::Backend::get_instance().await;
    backend.register_incoming_message_callback(cb);
}
#[napi]
pub async fn load_chats() -> Result<()> {
    let backend = tg::Backend::get_instance().await;
    backend
        .load_chats_with_offset(None)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(())
}

#[napi]
pub async fn get_me() -> Result<NativeSeenChat> {
    let backend = tg::Backend::get_instance().await;
    let me = backend.get_me().await.map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(me)
}

#[napi]
pub async fn load_chats_with_offset(last_message_ids: HashMap<String, i32>) -> Result<()> {
    let backend = tg::Backend::get_instance().await;
    let dash_map = dashmap::DashMap::with_capacity(last_message_ids.len());
    for (chat_id, last_message_id) in last_message_ids {
        dash_map.insert(chat_id.parse().unwrap(), last_message_id);
    }
    backend
        .load_chats_with_offset(Some(dash_map))
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(())
}

#[napi]
pub async fn sync_caches_from_local_db(
    packed_chats: Vec<NativePackedChat>,
    // seen_chats: Vec<NativeSeenChat>,
    chats: Vec<NativeChat>,
) -> Result<()> {
    let backend = tg::Backend::get_instance().await;
    backend
        .sync_caches_from_local_db(packed_chats, chats)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(())
}

// #[napi]
// pub async fn get_chat(id: i64, chat_type: ChatType) -> Result<NativeChat> {
//     let backend = tg::Backend::get_instance().await;
//     let chat = backend
//         .get_chat(id)
//         .await
//         .map_err(|e| Error::from_reason(e.to_string()))?;
//     Ok(NativeChat::from_raw(&chat).await)
// }

#[napi]
pub async fn send_message(chat_id: i64, text: String, medias: Option<Vec<String>>, update_upload_progress_callback: UpdateUploadProgressCallback) -> Result<Vec<NativeMessage>> {
    let backend = tg::Backend::get_instance().await;
    let messages = backend
        .send_message(chat_id, text, medias, update_upload_progress_callback)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(messages)
}

#[napi]
pub async fn download_media_from_message(chat_id: i64, message_id: i32) -> Result<String> {
    let backend = tg::Backend::get_instance().await;
    let path = backend
        .download_media_from_message(chat_id, message_id)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(path)
}

#[napi]
pub async fn download_profile_photo(chat_id: i64) -> Result<String> {
    let backend = tg::Backend::get_instance().await;
    let path = backend
        .download_chat_photo_by_chat_id(chat_id, true)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(path)
}

#[napi]
pub async fn get_chat_photo_thumb(chat_id: i64) -> Result<Option<Buffer>> {
    let backend = tg::Backend::get_instance().await;
    let thumb_vec = backend.get_chat_photo_thumb_by_chat_id(chat_id).await.map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(thumb_vec.map(Buffer::from))
}

#[napi]
pub async fn reconnect() -> Result<bool> {
    Ok(tg::Backend::get_instance().await.reconnect().await)
}