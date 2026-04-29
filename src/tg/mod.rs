use std::io::Write;
mod chat;
mod config;
mod login;
mod message;
mod reconnect;
mod run;
pub mod types;
mod utils;

use crate::tg::config::MAX_CONCURRENT_REQUESTS;
use crate::tg::reconnect::HomoReconnectPolicy;
use crate::tg::types::*;
use anyhow::Result;
use config::{SIGN_OUT_RETRIES, TELEGRAM_API_HASH, TELEGRAM_API_ID};
use const_format::{concatcp, formatcp};
use dashmap::{DashMap as HashMap, DashSet as HashSet};
use grammers_client::client::{MessageIter, UpdateStream, UpdatesConfiguration};
use grammers_client::peer::User;
use grammers_client::tl;
use grammers_client::{Client, SignInError};
use grammers_crypto::two_factor_auth::check_p_and_g;
use grammers_mtsender::{SenderPool, SenderPoolRunner};
use grammers_session::storages::SqliteSession;
use grammers_session::types::PeerRef;
use grammers_tl_types::enums::messages::Messages;
use grammers_tl_types::enums::InputPeer;
use grammers_tl_types::{Deserializable, Serializable};
use hilog::{Builder, LogDomain};
use libc::c_char;
use log::LevelFilter;
use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::Promise;
use napi_ohos::bindgen_prelude::*;
use napi_ohos::threadsafe_function::{
    ThreadsafeFunction, ThreadsafeFunctionCallMode, UnknownReturnValue,
};
use napi_ohos::tokio;
use napi_ohos::tokio::runtime;
use napi_ohos::tokio::sync::{mpsc, Mutex, MutexGuard, OnceCell, RwLock, Semaphore};
use ohos_hilog_binding::{debug, info, LogLevel, LogType};
use std::collections::{BTreeMap, VecDeque};
use std::ffi::CStr;
use std::ops::ControlFlow;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

const BASE_PATH: &str = "/data/storage/el2/base/";
const SESSION_FILE: &str = concatcp!(BASE_PATH, "session");

type ChatsMap = HashMap<i64, NativeChat>;

pub struct Backend {
    client: Client,
    session: Arc<SqliteSession>,
    updates: Mutex<UpdateStream>,
    sender_pool_runner: Option<tokio::task::JoinHandle<()>>,
    user: Option<User>,
    login_token: Option<grammers_client::client::LoginToken>,
    login_state: Option<LoginState>,
    password_token: Option<grammers_client::client::PasswordToken>,
    seen_packed_chats_map: HashMap<i64, PeerRef>,
    chats_map: HashMap<i64, NativeChat>,
    cache_seen_chat_callback: Option<CacheSeenChatCallback>,
    load_chats_callback: Option<LoadChatsCallback>,
    update_chat_callback: Option<UpdateChatCallback>,
    incoming_message_callback: Option<IncomingMessageCallback>,
    run_handler: Option<tokio::task::JoinHandle<Result<()>>>,
    profile_photo_downloading_set: HashSet<i64>,
    save_session_mutex: Mutex<()>,
    global_semaphore: Semaphore,
}

#[allow(static_mut_refs)]
static mut INSTANCE: OnceCell<Backend> = OnceCell::const_new();
// static mut INSTANCE: Option<Backend> = None;
// static INSTANCE: LazyLock<Mutex<Backend>> = LazyLock::new(|| {
//     let rt = runtime::Runtime::new().unwrap();
//     rt.block_on(async {
//         Mutex::new(Backend::new().await.unwrap())
//     })
// });

impl Backend {
    #[allow(static_mut_refs)]
    async fn init() -> &'static Backend {
        unsafe {
            INSTANCE
                .get_or_init(|| async { Backend::new().await.unwrap() })
                .await
        }
    }

    #[allow(static_mut_refs)]
    pub async fn get_instance() -> &'static mut Backend {
        unsafe {
            if !INSTANCE.initialized() {
                Backend::init().await;
            }
            INSTANCE.get_mut().unwrap()
        }
    }

    async fn new() -> Result<Self> {
        let filter = EnvFilter::try_new("trace")?;
        let ohrs_writer_layer = tracing_ohos::layer(0x0000, "homogrape")?;

        tracing_subscriber::registry()
            .with(ohrs_writer_layer)
            .with(filter)
            .init();

        // tracing_subscriber::fmt()
        //     .with_writer(MakeHiLogWriter)
        //     .with_env_filter(filter)
        //     .with_ansi(false)
        //     .init();

        // let mut builder = Builder::new();
        // // use stdext::function_name;
        // builder
        //     .set_domain(LogDomain::new(0x0000))
        //     .filter_level(LevelFilter::Trace)
        //     // .filter_module("homoapp_native", LevelFilter::Trace)
        //     .format(|buf, record| {
        //         writeln!(
        //             buf,
        //             "{}:{} - {}",
        //             record.file().unwrap_or("unknown"),
        //             record.line().unwrap_or(0),
        //             record.args()
        //         )
        //     });
        // builder.init();

        info!("Constructing Telegram backend...");

        let api_id = TELEGRAM_API_ID.parse()?;
        info!("Connecting to Telegram...");
        // let session = unsafe {
        //     use std::io::Write;
        //     use std::fs::File;
        //     let file = File::open(SESSION_FILE)?;
        //     Session::load(memmap2::MmapOptions::new().map(&file)?.as_ref())?
        // };
        let session = Arc::new(SqliteSession::open(SESSION_FILE).await?);
        let SenderPool {
            runner,
            updates,
            handle,
        } = SenderPool::new(Arc::clone(&session), api_id);
        let client = Client::new(handle);
        let sender_pool_runner = tokio::spawn(runner.run());
        let updates = client
            .stream_updates(
                updates,
                UpdatesConfiguration {
                catch_up: true,
                ..Default::default()
                },
            )
            .await;
        info!("Connected!");

        Ok(Self {
            client,
            session,
            updates: Mutex::new(updates),
            sender_pool_runner: Some(sender_pool_runner),
            user: None,
            chats_map: HashMap::default(),
            login_token: None,
            login_state: None,
            password_token: None,
            cache_seen_chat_callback: None,
            load_chats_callback: None,
            update_chat_callback: None,
            incoming_message_callback: None,
            run_handler: None,
            profile_photo_downloading_set: HashSet::default(),
            seen_packed_chats_map: HashMap::default(),
            save_session_mutex: Mutex::new(()),
            global_semaphore: Semaphore::new(MAX_CONCURRENT_REQUESTS),
        })
    }

    async fn save_session(&self) {
        debug!("save_session Saving session...");
        let _guard = self.save_session_mutex.lock().await;
        debug!("save_session Session save mutex acquired!");
        debug!("save_session SqliteSession persists changes automatically");
    }

    pub async fn register_device(&self, token: String) -> Result<bool> {
        debug!("Registering device...");
        use grammers_client::tl::functions::account::RegisterDevice;
        let request = RegisterDevice {
            no_muted: false,
            token_type: 13, // Huawei Push, https://core.telegram.org/api/push-updates#subscribing-to-notifications
            token,
            app_sandbox: false,
            secret: vec![],
            other_uids: vec![],
        };
        let response = self.client.invoke(&request).await;
        match response {
            Ok(_) => {
                debug!("Device registered!");
                Ok(true)
            }
            Err(e) => {
                error!("Failed to register device: {e}");
                Ok(false)
            }
        }
    }

    pub(crate) fn register_load_chats_callback(&mut self, cb: LoadChatsCallback) {
        self.load_chats_callback.replace(cb);
    }

    pub(crate) fn register_cache_seen_chat_callback(&mut self, cb: CacheSeenChatCallback) {
        self.cache_seen_chat_callback.replace(cb);
    }

    pub(crate) fn register_update_chat_callback(&mut self, cb: UpdateChatCallback) {
        self.update_chat_callback.replace(cb);
    }

    pub(crate) fn register_incoming_message_callback(&mut self, cb: IncomingMessageCallback) {
        self.incoming_message_callback.replace(cb);
    }

    #[inline]
    pub async fn is_logged_in(&self) -> bool {
        self.client.is_authorized().await.unwrap()
    }

    #[inline]
    pub async fn sign_out(&self) -> bool {
        if self.client.sign_out().await.is_ok() {
            debug!("Signed out successfully!");
            true
        } else {
            error!("Sign out failed!");
            false
        }
    }

    #[inline]
    pub async fn get_me(&self) -> Result<NativeSeenChat> {
        Ok(NativeSeenChat::from_user(&self.client.get_me().await?))
    }

    /// 注册设备以接收推送通知
    ///
    /// 参数:
    /// - `token`: 设备推送令牌 (Simple push 类型)
    ///
    /// 返回值:
    /// - `Result<bool>`: 注册成功返回 true，失败返回 false
    #[inline]
    pub async fn register_push(&self, token_type: i32, token: String) -> String {
        debug!("Registering push device with Simple push...");
        let request = tl::functions::account::RegisterDevice {
            no_muted: true, // 不静音，接收所有通知
            token_type,
            token,              // 设备推送令牌
            app_sandbox: false, // 使用生产环境证书
            secret: vec![],     // 不需要加密密钥
            other_uids: vec![], // 其他用户ID列表（可选）
        };

        match self.client.invoke(&request).await {
            Ok(_) => "OK".into(),
            Err(e) => e.to_string(),
        }
    }

    #[inline]
    pub async fn unregister_push(&self, token_type: i32, token: String) -> String {
        debug!("Unregistering push device with Simple push...");
        let request = tl::functions::account::UnregisterDevice {
            token_type,
            token,              // 设备推送令牌
            other_uids: vec![], // 其他用户ID列表（可选）
        };

        match self.client.invoke(&request).await {
            Ok(_) => "OK".into(),
            Err(e) => e.to_string(),
        }
    }

    #[inline]
    fn insert_chat_to(&mut self, chat: &NativeChat) {
        self.chats_map.insert(chat.chat_id, chat.clone());
    }

}

impl Drop for Backend {
    fn drop(&mut self) {
        debug!("Dropping Backend...");
    }
}
