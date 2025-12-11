use grammers_client::grammers_tl_types as tl;
use grammers_client::types::Chat;
use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::{Buffer, Promise};
use napi_ohos::threadsafe_function::ThreadsafeFunction;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

pub type LoadChatsCallback = ThreadsafeFunction<(), Promise<()>>;
pub type CacheSeenChatCallback = ThreadsafeFunction<NativeSeenChat, Promise<()>>;
pub type UpdateChatCallback = ThreadsafeFunction<(
    NativeSeenChat,
    NativeChat,
    Vec<NativeMessage>,
), Promise<()>>;
pub type IncomingMessageCallback = ThreadsafeFunction<(Option<NativeChat>, NativeMessage)>;

// (media_index, current_progress): void => {} 
pub type UpdateUploadProgressCallback = ThreadsafeFunction<(i64, i64), Promise<()>>;
#[derive(Debug, PartialEq)]
#[napi]
pub enum LoginState {
    WrongPhoneNumber,
    CodeRequired,
    WrongCode,
    PasswordRequired,
    WrongPassword,
    LoggedIn,
    LoginFailure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum MediaType {
    None,
    Photo,
    Document,
    Sticker,
    Contact,
    Poll,
    Geo,
    Dice,
    Venue,
    GeoLive,
    WebPage,
}

impl From<Option<grammers_client::types::Media>> for MediaType {
    fn from(value: Option<grammers_client::types::Media>) -> Self {
        use grammers_client::types::Media;
        match value {
            Some(Media::Photo(_)) => MediaType::Photo,
            Some(Media::Document(_)) => MediaType::Document,
            Some(Media::Sticker(_)) => MediaType::Sticker,
            Some(Media::Contact(_)) => MediaType::Contact,
            Some(Media::Poll(_)) => MediaType::Poll,
            Some(Media::Geo(_)) => MediaType::Geo,
            Some(Media::Dice(_)) => MediaType::Dice,
            Some(Media::Venue(_)) => MediaType::Venue,
            Some(Media::GeoLive(_)) => MediaType::GeoLive,
            Some(Media::WebPage(_)) => MediaType::WebPage,
            None => MediaType::None,
            _ => { MediaType::None }
        }
    }
}

/// 消息格式化实体类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum MessageEntityType {
    Unknown,
    Mention,        // @username
    Hashtag,        // #hashtag
    BotCommand,     // /command
    Url,            // https://...
    Email,          // email@example.com
    Bold,           // **bold**
    Italic,         // *italic*
    Code,           // `code`
    Pre,            // ```pre```
    TextUrl,        // [text](url)
    MentionName,    // 文本提及用户
    Phone,          // 电话号码
    Cashtag,        // $USD
    Underline,      // 下划线
    Strike,         // 删除线
    Spoiler,        // 剧透文本
    CustomEmoji,    // 自定义表情
    Blockquote,     // 引用块
}

impl From<&tl::enums::MessageEntity> for MessageEntityType {
    fn from(entity: &tl::enums::MessageEntity) -> Self {
        use tl::enums::MessageEntity::*;
        match entity {
            Unknown(_) => MessageEntityType::Unknown,
            Mention(_) => MessageEntityType::Mention,
            Hashtag(_) => MessageEntityType::Hashtag,
            BotCommand(_) => MessageEntityType::BotCommand,
            Url(_) => MessageEntityType::Url,
            Email(_) => MessageEntityType::Email,
            Bold(_) => MessageEntityType::Bold,
            Italic(_) => MessageEntityType::Italic,
            Code(_) => MessageEntityType::Code,
            Pre(_) => MessageEntityType::Pre,
            TextUrl(_) => MessageEntityType::TextUrl,
            MentionName(_) => MessageEntityType::MentionName,
            Phone(_) => MessageEntityType::Phone,
            Cashtag(_) => MessageEntityType::Cashtag,
            Underline(_) => MessageEntityType::Underline,
            Strike(_) => MessageEntityType::Strike,
            Spoiler(_) => MessageEntityType::Spoiler,
            CustomEmoji(_) => MessageEntityType::CustomEmoji,
            Blockquote(_) => MessageEntityType::Blockquote,
            _ => MessageEntityType::Unknown,
        }
    }
}

/// 消息格式化实体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeMessageEntity {
    pub entity_type: MessageEntityType,
    pub offset: i32,      // UTF-16 偏移量
    pub length: i32,      // UTF-16 长度
    pub url: Option<String>,         // TextUrl 的链接
    pub user_id: Option<i64>,        // MentionName 的用户 ID
    pub language: Option<String>,    // Pre 的语言
    pub custom_emoji_id: Option<i64>, // CustomEmoji 的 ID
}

impl NativeMessageEntity {
    pub fn from_raw(entity: &tl::enums::MessageEntity) -> Self {
        use tl::enums::MessageEntity::*;
        
        let (offset, length) = match entity {
            Unknown(e) => (e.offset, e.length),
            Mention(e) => (e.offset, e.length),
            Hashtag(e) => (e.offset, e.length),
            BotCommand(e) => (e.offset, e.length),
            Url(e) => (e.offset, e.length),
            Email(e) => (e.offset, e.length),
            Bold(e) => (e.offset, e.length),
            Italic(e) => (e.offset, e.length),
            Code(e) => (e.offset, e.length),
            Pre(e) => (e.offset, e.length),
            TextUrl(e) => (e.offset, e.length),
            MentionName(e) => (e.offset, e.length),
            InputMessageEntityMentionName(e) => (e.offset, e.length),
            Phone(e) => (e.offset, e.length),
            Cashtag(e) => (e.offset, e.length),
            Underline(e) => (e.offset, e.length),
            Strike(e) => (e.offset, e.length),
            Spoiler(e) => (e.offset, e.length),
            CustomEmoji(e) => (e.offset, e.length),
            Blockquote(e) => (e.offset, e.length),
            BankCard(e) => (e.offset, e.length),
            _ => (0, 0),
        };
        
        let url = if let TextUrl(e) = entity {
            Some(e.url.clone())
        } else {
            None
        };
        
        let user_id = if let MentionName(e) = entity {
            Some(e.user_id)
        } else {
            None
        };
        
        let language = if let Pre(e) = entity {
            Some(e.language.clone())
        } else {
            None
        };
        
        let custom_emoji_id = if let CustomEmoji(e) = entity {
            Some(e.document_id)
        } else {
            None
        };
        
        Self {
            entity_type: MessageEntityType::from(entity),
            offset,
            length,
            url,
            user_id,
            language,
            custom_emoji_id,
        }
    }
}

#[derive(Clone)]
#[napi(object)]
pub struct NativePackedChat {
    pub chat_id: i64,
    pub packed_chat: String,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeRawMessage {
    pub chat_id: i64,
    pub message_id: i32,
    pub raw_message: Buffer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeMessage {
    pub message_id: i32,
    pub chat_id: i64,
    pub outgoing: bool,
    pub pinned: bool,
    pub sender_id: i64,
    pub sender_name: String,
    pub timestamp: i64,
    pub text: String,
    pub view_count: Option<i32>,
    pub forward_count: Option<i32>,
    pub reply_count: Option<i32>,
    pub media_type: MediaType,
    pub edit_timestamp: Option<i64>,
    pub grouped_id: Option<i64>,
    pub reply_to_message_id: Option<i32>,
    /// 消息格式化实体（粗体、斜体、链接等）
    pub fmt_entities: Option<Vec<NativeMessageEntity>>,
}

impl NativeMessage {
    pub fn from_raw(raw: &grammers_client::types::Message) -> Self {
        let mut sender_id = -1;
        let mut sender_name = "".to_string();
        if raw.sender().is_some() {
            sender_id = raw.sender().unwrap().id();
            sender_name = raw.sender().unwrap().name().to_string();
        }
        
        // 解析格式化实体
        let fmt_entities = raw.fmt_entities().map(|entities| {
            entities.iter().map(NativeMessageEntity::from_raw).collect()
        });
        
        Self {
            message_id: raw.id(),
            chat_id: raw.chat().id(),
            outgoing: raw.outgoing(),
            pinned: raw.pinned(),
            sender_id,
            sender_name,
            timestamp: raw.date().timestamp(),
            text: raw.text().to_string(),
            view_count: raw.view_count(),
            forward_count: raw.forward_count(),
            reply_count: raw.reply_count(),
            media_type: MediaType::from(raw.media()),
            edit_timestamp: raw.edit_date().map(|d| d.timestamp()),
            grouped_id: raw.grouped_id(),
            reply_to_message_id: raw.reply_to_message_id(),
            fmt_entities,
        }
    }
}

impl Hash for NativeMessage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.message_id.hash(state);
        self.chat_id.hash(state);
        self.outgoing.hash(state);
        self.sender_id.hash(state);
        self.sender_name.hash(state);
        self.timestamp.hash(state);
        self.text.hash(state);
    }
}

impl Eq for NativeMessage {}

impl PartialOrd for NativeMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NativeMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.chat_id != other.chat_id {
            return if self.chat_id > other.chat_id {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Greater
            };
        }
        core::cmp::Ordering::Equal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi]
pub enum ChatType {
    User,
    Group,
    Channel,
}

impl ChatType {
    pub fn from_chat(raw: &grammers_client::types::Chat) -> Self {
        match raw {
            grammers_client::types::Chat::User(_) => ChatType::User,
            grammers_client::types::Chat::Group(_) => ChatType::Group,
            grammers_client::types::Chat::Channel(_) => ChatType::Channel,
        }
    }

    pub fn from_dialog(dialog: &grammers_client::types::Dialog) -> Self {
        Self::from_chat(dialog.chat())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeChat {
    pub chat_id: i64,
    pub chat_type: ChatType,
    pub name: String,
    pub pinned: bool,
    pub last_message_id: i32,
    pub last_message_sender_name: String,
    pub last_message_text: String,
    pub last_message_timestamp: i64,
    pub megagroup: bool,
    pub forum: bool,
    // pub forums: Option<Vec<i64>>,
}

impl NativeChat {
    pub async fn from_raw(raw: &grammers_client::types::Chat) -> Self {
        // let megagroup: bool = match raw {
        //     Chat::User(_) => { false }
        //     Chat::Group(g) => { g.is_megagroup() }
        //     Chat::Channel(_) => { false }
        // };
        // let forum = if megagroup {
        //     if let Chat::Group(g) = raw {
        //         if let tl::enums::Chat::Channel(c) = &g.raw {
        //             c.forum
        //         } else { false }
        //     } else { false }
        // } else { false };
        let megagroup = matches!(raw, Chat::Group(g) if g.is_megagroup());
        let forum = megagroup && matches!(raw, Chat::Group(g)
            if matches!(&g.raw, tl::enums::Chat::Channel(c) if c.forum)); // TODO: check this
        Self {
            chat_id: raw.id(),
            chat_type: ChatType::from_chat(raw),
            name: raw.name().to_string(),
            pinned: false,
            last_message_id: 0,
            last_message_sender_name: "".to_string(),
            last_message_text: "".to_string(),
            last_message_timestamp: 0,
            megagroup,
            forum,
        }
    }

    pub async fn from_dialog(dialog: grammers_client::types::Dialog) -> Self {
        let chat = dialog.chat();
        let mut last_message_id = 0;
        let mut last_message_sender_name = "".to_string();
        let mut last_message_text = "".to_string();
        let mut last_message_timestamp = 0;
        let megagroup: bool = match chat {
            Chat::User(_) => { false }
            Chat::Group(g) => { g.is_megagroup() }
            Chat::Channel(_) => { false }
        };
        let forum = if megagroup {
            if let Chat::Group(g) = chat {
                if let tl::enums::Chat::Channel(c) = &g.raw {
                    c.forum
                } else { false }
            } else { false }
        } else { false };
        if let Some(ref message) = dialog.last_message {
            last_message_id = message.id();
            last_message_sender_name = message.sender().map(|s| s.name().to_string()).unwrap_or("".to_string());
            last_message_text = message.text().to_string();
            last_message_timestamp = message.date().timestamp();
        }
        Self {
            chat_id: dialog.chat().id(),
            chat_type: ChatType::from_chat(chat),
            name: chat.name().to_string(),
            pinned: dialog.raw.pinned(),
            last_message_id,
            last_message_sender_name,
            last_message_text,
            last_message_timestamp,
            megagroup,
            forum,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeSeenChat {
    pub chat_id: i64,
    pub chat_type: ChatType,
    pub packed_chat: String,
    pub is_contact: bool,
    pub is_mutual_contact: bool,
    pub phone: Option<String>,
    pub username: Option<String>,
    pub photo_thumb: Option<Vec<u8>>,
    pub full_name: String,
    pub first_name: String,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub date_of_birth: Option<i64>,
    pub forum: bool,
}

#[napi]
impl NativeSeenChat {
    pub fn from_raw(raw: &grammers_client::types::Chat) -> Self {
        match raw {
            grammers_client::types::Chat::User(user) => Self::from_user(user),
            grammers_client::types::Chat::Group(group) => Self::from_group(group),
            grammers_client::types::Chat::Channel(channel) => Self::from_channel(channel),
        }
    }
    pub fn from_user(raw: &grammers_client::types::User) -> Self {
        Self {
            chat_id: raw.id(),
            chat_type: ChatType::User,
            packed_chat: raw.pack().to_hex(),
            is_contact: raw.contact(),
            is_mutual_contact: raw.mutual_contact(),
            phone: raw.phone().map(|p| p.to_string()),
            username: raw.username().map(|u| u.to_string()),
            full_name: raw.full_name().to_string(),
            first_name: raw.first_name().to_string(),
            last_name: raw.last_name().map(|l| l.to_string()),
            bio: None,
            photo_thumb: raw.photo().map(|p| p.stripped_thumb.clone()).unwrap_or(None),
            date_of_birth: None,
            forum: false,
        }
    }

    pub fn from_group(raw: &grammers_client::types::Group) -> Self {
        Self {
            chat_id: raw.id(),
            chat_type: ChatType::Group,
            packed_chat: raw.pack().to_hex(),
            is_contact: false,
            is_mutual_contact: false,
            phone: None,
            username: None,
            full_name: raw.title().to_string(),
            first_name: raw.title().to_string(),
            last_name: None,
            bio: None,
            photo_thumb: raw.photo().map(|p| p.stripped_thumb.clone()).unwrap_or(None),
            date_of_birth: None,
            forum: false,
        }
    }

    pub fn from_channel(raw: &grammers_client::types::Channel) -> Self {
        Self {
            chat_id: raw.id(),
            chat_type: ChatType::Channel,
            packed_chat: raw.pack().to_hex(),
            is_contact: false,
            is_mutual_contact: false,
            phone: None,
            username: raw.username().map(|u| u.to_string()),
            full_name: raw.title().to_string(),
            first_name: raw.title().to_string(),
            last_name: None,
            bio: None,
            photo_thumb: raw.photo().map(|p| p.stripped_thumb.clone()).unwrap_or(None),
            date_of_birth: None,
            forum: raw.raw.forum,
        }
    }
}

/// 参与者角色类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi]
pub enum NativeParticipantRole {
    /// 普通用户
    User,
    /// 群组/频道创建者
    Creator,
    /// 管理员
    Admin,
    /// 被封禁用户
    Banned,
    /// 已离开用户
    Left,
}

impl From<&grammers_client::types::participant::Role> for NativeParticipantRole {
    fn from(role: &grammers_client::types::participant::Role) -> Self {
        use grammers_client::types::participant::Role;
        match role {
            Role::User(_) => NativeParticipantRole::User,
            Role::Creator(_) => NativeParticipantRole::Creator,
            Role::Admin(_) => NativeParticipantRole::Admin,
            Role::Banned(_) => NativeParticipantRole::Banned,
            Role::Left(_) => NativeParticipantRole::Left,
            _ => NativeParticipantRole::User,
        }
    }
}

/// 聊天参与者
#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeParticipant {
    /// 用户 ID
    pub user_id: i64,
    /// 用户全名
    pub full_name: String,
    /// 用户名（可选）
    pub first_name: String,
    /// 用户名（可选）
    pub last_name: Option<String>,
    /// 用户名 @username（可选）
    pub username: Option<String>,
    /// 用户角色
    pub role: NativeParticipantRole,
    /// 头像缩略图
    pub photo_thumb: Option<Vec<u8>>,
}

impl NativeParticipant {
    pub fn from_raw(participant: &grammers_client::types::Participant) -> Self {
        Self {
            user_id: participant.user.id(),
            full_name: participant.user.full_name().to_string(),
            first_name: participant.user.first_name().to_string(),
            last_name: participant.user.last_name().map(|s| s.to_string()),
            username: participant.user.username().map(|s| s.to_string()),
            role: NativeParticipantRole::from(&participant.role),
            photo_thumb: participant.user.photo().and_then(|p| p.stripped_thumb.clone()),
        }
    }
}