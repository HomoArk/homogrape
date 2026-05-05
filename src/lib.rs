#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

mod tg;

use crate::tg::types::*;
use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::*;
use napi_ohos::{tokio, Error};
use std::collections::HashMap;
use std::sync::Arc;

type Result<T> = std::result::Result<T, Error>;

fn napi_error(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}

async fn auth_state(login_state: LoginState) -> NativeAuthState {
    NativeAuthState {
        authorized: tg::Backend::get_instance().await.is_logged_in().await,
        login_state,
    }
}

#[napi]
pub async fn initialize(options: Option<NativeRuntimeOptions>) -> Result<NativeRuntimeState> {
    let backend = tg::Backend::get_instance().await;
    backend
        .set_event_replay_limit(options.and_then(|options| options.event_replay_limit))
        .await;
    let state = backend.runtime_state().await;
    let mut event = NativeEvent::new(NativeEventKind::RuntimeState);
    event.seq = state.last_event_seq;
    backend.emit_event(event).await;
    Ok(backend.runtime_state().await)
}

#[napi]
pub async fn set_event_callback(cb: NativeEventCallback) {
    tg::Backend::get_instance()
        .await
        .register_event_callback(cb)
        .await;
}

#[napi]
pub async fn get_events_since(seq: i64, limit: u32) -> Vec<NativeEvent> {
    tg::Backend::get_instance()
        .await
        .events_since(seq, limit)
        .await
}

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
        .map_err(napi_error)
}

#[napi]
pub async fn request_login_code(phone_number: String) -> Result<NativeAuthState> {
    let state = tg::Backend::get_instance()
        .await
        .login_with_phone(phone_number)
        .await
        .map_err(napi_error)?;
    Ok(auth_state(state).await)
}

#[napi]
pub async fn submit_login_code(code: String) -> Result<NativeAuthState> {
    let state = tg::Backend::get_instance()
        .await
        .provide_verify_code(code)
        .await
        .map_err(napi_error)?;
    Ok(auth_state(state).await)
}

#[napi]
pub async fn submit_password(password: String) -> Result<NativeAuthState> {
    let state = tg::Backend::get_instance()
        .await
        .provide_password(password)
        .await
        .map_err(napi_error)?;
    Ok(auth_state(state).await)
}

#[napi]
pub async fn login(phone_number: String) -> Result<LoginState> {
    Ok(request_login_code(phone_number).await?.login_state)
}

#[napi]
pub async fn verify_code(code: String) -> Result<LoginState> {
    Ok(submit_login_code(code).await?.login_state)
}

#[napi]
pub async fn password(password: String) -> Result<LoginState> {
    Ok(submit_password(password).await?.login_state)
}

#[napi]
pub async fn sign_out() -> bool {
    tg::Backend::get_instance().await.sign_out().await
}

#[napi]
pub async fn start_runtime() -> Result<NativeRuntimeState> {
    let backend = tg::Backend::get_instance().await;
    if !backend.is_run_handler_active().await {
        let handler = tokio::spawn(tg::Backend::get_instance().await.run());
        tg::Backend::get_instance()
            .await
            .set_run_handler(handler)
            .await;
    }
    let state = backend.runtime_state().await;
    let mut event = NativeEvent::new(NativeEventKind::RuntimeState);
    event.seq = state.last_event_seq;
    backend.emit_event(event).await;
    Ok(backend.runtime_state().await)
}

#[napi]
pub async fn stop_runtime() -> Result<NativeRuntimeState> {
    let backend = tg::Backend::get_instance().await;
    backend.abort_run_handler().await;
    let state = backend.runtime_state().await;
    let mut event = NativeEvent::new(NativeEventKind::RuntimeState);
    event.seq = state.last_event_seq;
    backend.emit_event(event).await;
    Ok(backend.runtime_state().await)
}

#[napi]
pub async fn reconnect_runtime() -> Result<NativeRuntimeState> {
    tg::Backend::get_instance().await.reconnect().await;
    Ok(tg::Backend::get_instance().await.runtime_state().await)
}

#[napi]
pub async fn run() -> Result<()> {
    start_runtime().await.map(drop)
}

#[napi]
pub async fn stop() {
    let _ = stop_runtime().await;
}

#[napi]
pub async fn reconnect() -> Result<bool> {
    Ok(reconnect_runtime().await?.authorized)
}

#[napi]
pub async fn register_update_chat_callback(cb: UpdateChatCallback) {
    tg::Backend::get_instance()
        .await
        .register_update_chat_callback(cb);
}

#[napi]
pub async fn register_cache_seen_chat_callback(cb: CacheSeenChatCallback) {
    tg::Backend::get_instance()
        .await
        .register_cache_seen_chat_callback(cb);
}

#[napi]
pub async fn register_load_chats_callback(cb: LoadChatsCallback) {
    tg::Backend::get_instance()
        .await
        .register_load_chats_callback(cb);
}

#[napi]
pub async fn register_incoming_message_callback(cb: IncomingMessageCallback) {
    tg::Backend::get_instance()
        .await
        .register_incoming_message_callback(cb);
}

#[napi]
pub async fn load_chats() -> Result<()> {
    tg::Backend::get_instance()
        .await
        .load_chats_with_offset(None)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn load_dialogs(request: NativeDialogLoadRequest) -> Result<NativeDialogLoadResult> {
    tg::Backend::get_instance()
        .await
        .load_dialogs_page(request)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn get_me() -> Result<NativePeer> {
    tg::Backend::get_instance()
        .await
        .get_me_peer()
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn load_chats_with_offset(last_message_ids: HashMap<String, i32>) -> Result<()> {
    let dash_map = dashmap::DashMap::with_capacity(last_message_ids.len());
    for (chat_id, last_message_id) in last_message_ids {
        let parsed_chat_id = chat_id
            .parse()
            .map_err(|_| Error::from_reason(format!("Invalid chat id: {chat_id}")))?;
        dash_map.insert(parsed_chat_id, last_message_id);
    }
    tg::Backend::get_instance()
        .await
        .load_chats_with_offset(Some(dash_map))
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn load_messages(request: NativeMessageLoadRequest) -> Result<NativeMessageLoadResult> {
    tg::Backend::get_instance()
        .await
        .load_messages_page(request)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn load_history_messages(
    chat_id: i64,
    last_message_id: Option<i32>,
    limit: Option<u32>,
) -> Result<Vec<NativeMessage>> {
    tg::Backend::get_instance()
        .await
        .load_history_messages(chat_id, last_message_id, limit)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn sync_caches_from_local_db(
    packed_chats: Vec<NativePackedChat>,
    chats: Vec<NativeChat>,
) -> Result<()> {
    tg::Backend::get_instance()
        .await
        .sync_caches_from_local_db(packed_chats, chats)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn send_message(request: NativeSendRequest) -> Result<NativeSendResult> {
    let media_paths = request.medias.map(|medias| {
        medias
            .into_iter()
            .map(|media| media.path)
            .collect::<Vec<_>>()
    });
    let messages = tg::Backend::get_instance()
        .await
        .send_message(
            request.chat_id,
            request.text,
            request.reply_to_message_id,
            media_paths,
            None,
        )
        .await
        .map_err(napi_error)?;
    Ok(NativeSendResult { messages })
}

#[napi]
pub async fn forward_messages(
    from_chat_id: i64,
    to_chat_id: i64,
    message_ids: Vec<i32>,
) -> Result<Vec<NativeMessage>> {
    let messages = tg::Backend::get_instance()
        .await
        .forward_messages(from_chat_id, to_chat_id, message_ids)
        .await
        .map_err(napi_error)?;
    Ok(messages.into_iter().flatten().collect())
}

#[napi]
pub async fn edit_message(chat_id: i64, message_id: i32, text: String) -> Result<NativeMessage> {
    tg::Backend::get_instance()
        .await
        .edit_message(chat_id, message_id, text)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn delete_messages(chat_id: i64, message_ids: Vec<i32>) -> Result<NativeActionResult> {
    tg::Backend::get_instance()
        .await
        .delete_messages_action(chat_id, message_ids)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn mark_chat_read(chat_id: i64) -> Result<NativeActionResult> {
    tg::Backend::get_instance()
        .await
        .mark_chat_read(chat_id)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn set_dialog_flags(
    chat_id: i64,
    flags: NativeDialogFlags,
) -> Result<NativeActionResult> {
    tg::Backend::get_instance()
        .await
        .set_dialog_flags(chat_id, flags)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn download_media(chat_id: i64, message_id: i32) -> Result<NativeMediaResult> {
    let local_path = tg::Backend::get_instance()
        .await
        .download_media_from_message(chat_id, message_id)
        .await
        .map_err(napi_error)?;
    Ok(NativeMediaResult {
        local_path,
        media_type: MediaType::Document,
    })
}

#[napi]
pub async fn download_media_from_message(chat_id: i64, message_id: i32) -> Result<String> {
    Ok(download_media(chat_id, message_id).await?.local_path)
}

#[napi]
pub async fn download_profile_photo(chat_id: i64) -> Result<NativeMediaResult> {
    let local_path = tg::Backend::get_instance()
        .await
        .download_chat_photo_by_chat_id(chat_id, true)
        .await
        .map_err(napi_error)?;
    Ok(NativeMediaResult {
        local_path,
        media_type: MediaType::Photo,
    })
}

#[napi]
pub async fn get_chat_photo_thumb(chat_id: i64) -> Result<Option<Vec<u8>>> {
    tg::Backend::get_instance()
        .await
        .get_chat_photo_thumb_by_chat_id(chat_id)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn get_contacts() -> Result<Vec<NativePeer>> {
    tg::Backend::get_instance()
        .await
        .get_contacts()
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn resolve_peer_by_username(username: String) -> Result<Option<NativeSearchResult>> {
    tg::Backend::get_instance()
        .await
        .resolve_peer_by_username(username)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn open_chat(chat_id: i64) -> Result<NativeSearchResult> {
    tg::Backend::get_instance()
        .await
        .open_chat(chat_id)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn search_peers(query: String, limit: Option<u32>) -> Result<NativeSearchPage> {
    tg::Backend::get_instance()
        .await
        .search_known_peers(query, limit)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn search_messages(
    chat_id: Option<i64>,
    query: String,
    offset_id: Option<i32>,
    limit: Option<u32>,
) -> Result<NativeSearchPage> {
    tg::Backend::get_instance()
        .await
        .search_messages_page(chat_id, query, offset_id, limit)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn get_participants(
    chat_id: i64,
    _offset: Option<u32>,
) -> Result<Vec<tg::types::NativeParticipant>> {
    tg::Backend::get_instance()
        .await
        .get_participants(chat_id)
        .await
        .map_err(napi_error)
}

#[napi]
pub async fn register_push(token_type: i32, token: String) -> String {
    tg::Backend::get_instance()
        .await
        .register_push(token_type, token)
        .await
}

#[napi]
pub async fn unregister_push(token_type: i32, token: String) -> String {
    tg::Backend::get_instance()
        .await
        .unregister_push(token_type, token)
        .await
}
