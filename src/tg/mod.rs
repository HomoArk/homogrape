use std::io::Write;
pub mod types;
mod login;
mod run;
mod message;
mod chat;
mod reconnect;
mod utils;
mod config;

use crate::tg::config::MAX_CONCURRENT_REQUESTS;
use crate::tg::reconnect::HomoReconnectPolicy;
use crate::tg::types::*;
use anyhow::Result;
use config::{SIGN_OUT_RETRIES, TELEGRAM_API_HASH, TELEGRAM_API_ID};
use const_format::{concatcp, formatcp};
use dashmap::{DashMap as HashMap, DashSet as HashSet};
use grammers_client::client::messages::MessageIter;
use grammers_client::session::Session;
use grammers_client::types::{Dialog, LoginToken, PasswordToken, User};
use grammers_client::{grammers_tl_types as tl, InitParams};
use grammers_client::{Client, Config, SignInError, Update};
use grammers_crypto::two_factor_auth::check_p_and_g;
use grammers_mtsender::ReconnectionPolicy;
use grammers_session::PackedChat;
use grammers_tl_types::enums::messages::Messages;
use grammers_tl_types::enums::InputPeer;
use grammers_tl_types::{Deserializable, Serializable};
use hilog::{hilog_writer::MakeHiLogWriter, Builder, LogDomain};
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
    user: Option<User>,
    login_token: Option<LoginToken>,
    login_state: Option<LoginState>,
    password_token: Option<PasswordToken>,
    seen_packed_chats_map: HashMap<i64, PackedChat>,
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

static mut INSTANCE: OnceCell<Backend> = OnceCell::const_new();
// static mut INSTANCE: Option<Backend> = None;
// static INSTANCE: LazyLock<Mutex<Backend>> = LazyLock::new(|| {
//     let rt = runtime::Runtime::new().unwrap();
//     rt.block_on(async {
//         Mutex::new(Backend::new().await.unwrap())
//     })
// });

impl Backend {
    async fn init() -> &'static Backend {
        unsafe {
            INSTANCE
                .get_or_init(|| async { Backend::new().await.unwrap() })
                .await
        }
    }

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
        let api_hash = TELEGRAM_API_HASH.to_string();
        info!("Connecting to Telegram...");
        // let session = unsafe {
        //     use std::io::Write;
        //     use std::fs::File;
        //     let file = File::open(SESSION_FILE)?;
        //     Session::load(memmap2::MmapOptions::new().map(&file)?.as_ref())?
        // };
        let client = Client::connect(Config {
            session: Session::load_file_or_create(SESSION_FILE)?,
            // session,
            api_id,
            api_hash: api_hash.clone(),
            params: InitParams {
                catch_up: true,
                reconnection_policy: &HomoReconnectPolicy,
                ..Default::default()
            },
        })
            .await?;
        info!("Connected!");

        Ok(Self {
            client,
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
        match self.client.session().save_to_file(SESSION_FILE) {
            Ok(_) => {
                debug!("save_session Session saved to {}", SESSION_FILE);
            }
            Err(e) => {
                error!("save_session failed to save the session to {SESSION_FILE}: {e}");
                // error!("failed to save the session: {e}. Logging out...");
                // if self.is_logged_in().await {
                //     for _ in 0..SIGN_OUT_RETRIES {
                //         if self.sign_out().await {
                //             return Err(anyhow::anyhow!(
                //                 "Failed to save the session and sign out!"
                //             ));
                //         }
                //     }
                //     panic!("Failed to save the session and sign out!"); // TODO: handle this better
                // }
            }
        }
    }

    pub async fn register_device(&self, token: String) -> Result<bool> {
        debug!("Registering device...");
        use grammers_client::grammers_tl_types::functions::account::RegisterDevice;
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

    #[inline]
    fn insert_chat_to(&mut self, chat: &NativeChat) {
        self.chats_map.insert(chat.chat_id, chat.clone());
    }

    #[inline]
    fn insert_seen_packed_chat(&mut self, seen_packed_chat: &PackedChat) {
        self.seen_packed_chats_map.insert(seen_packed_chat.id, *seen_packed_chat);
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        debug!("Dropping Backend...");
    }
}
