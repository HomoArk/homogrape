use grammers_client::media::Media;
use grammers_client::message::Message;
use grammers_client::peer::{Channel, Dialog, Group, Participant, Peer, Role, User};
use grammers_client::tl;
use grammers_session::types::{PeerAuth, PeerId, PeerInfo, PeerRef};
use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::{FnArgs, Promise};
use napi_ohos::threadsafe_function::ThreadsafeFunction;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

pub type LoadChatsCallback = ThreadsafeFunction<(), Promise<()>>;
pub type CacheSeenChatCallback = ThreadsafeFunction<NativeSeenChat, Promise<()>>;
pub type UpdateChatCallback =
    ThreadsafeFunction<FnArgs<(NativeSeenChat, NativeChat, Vec<NativeMessage>)>, Promise<()>>;
pub type IncomingMessageCallback =
    ThreadsafeFunction<FnArgs<(Option<NativeChat>, NativeMessage)>, Promise<()>>;
pub type NativeEventCallback = ThreadsafeFunction<NativeEvent, Promise<()>>;

// (media_index, current_progress): void => {}
pub type UpdateUploadProgressCallback = ThreadsafeFunction<FnArgs<(i64, i64)>, Promise<()>>;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[napi(object)]
pub struct NativeAuthState {
    pub login_state: LoginState,
    pub authorized: bool,
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

impl From<Option<Media>> for MediaType {
    fn from(value: Option<Media>) -> Self {
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
            _ => MediaType::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum NativeSendState {
    Pending,
    Sent,
    Failed,
}

pub fn peer_ref_to_string(peer_ref: PeerRef) -> String {
    format!(
        "{}:{}",
        peer_ref.id.bot_api_dialog_id(),
        peer_ref.auth.hash()
    )
}

fn peer_info_to_string(peer_info: PeerInfo) -> String {
    peer_ref_to_string(PeerRef {
        id: peer_info.id(),
        auth: peer_info.auth().unwrap_or_default(),
    })
}

pub fn peer_ref_from_string(value: &str) -> std::result::Result<PeerRef, anyhow::Error> {
    let (id, auth) = value
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Invalid packed chat format"))?;
    let id = id.parse::<i64>()?;
    let auth = auth.parse::<i64>()?;
    let peer_id = if id > 0 {
        PeerId::user(id)
    } else if id <= -1_000_000_000_000 {
        PeerId::channel(-id - 1_000_000_000_000)
    } else {
        PeerId::chat(-id)
    };
    Ok(PeerRef {
        id: peer_id,
        auth: PeerAuth::from_hash(auth),
    })
}

/// Rich-text entity type used by Telegram messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum MessageEntityType {
    Unknown,
    Mention,
    Hashtag,
    BotCommand,
    Url, // https://...
    Email,
    Bold,
    Italic,
    Code,
    Pre,
    TextUrl, // [text](url)
    MentionName,
    Phone,
    Cashtag, // $USD
    Underline,
    Strike,
    Spoiler,
    CustomEmoji,
    Blockquote,
}

/// Sticker metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct StickerInfo {
    /// Associated emoji from Telegram's alt field.
    pub emoji: Option<String>,
    /// Whether this sticker is animated in TGS/Lottie format.
    pub is_animated: bool,
    /// Whether this sticker is a WEBM video sticker.
    pub is_video: bool,
    /// MIME type.
    pub mime_type: Option<String>,
    /// File size in bytes.
    pub file_size: Option<i64>,
    /// Pixel width.
    pub width: Option<i32>,
    /// Pixel height.
    pub height: Option<i32>,
}

/// Generic media metadata for photos, documents, videos, and similar payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct MediaInfo {
    /// MIME type, such as image/jpeg, video/mp4, or application/pdf.
    pub mime_type: Option<String>,
    /// Original file name when Telegram provides one.
    pub file_name: Option<String>,
    /// File size in bytes.
    pub file_size: Option<i64>,
    /// Pixel width for image and video media.
    pub width: Option<i32>,
    /// Pixel height for image and video media.
    pub height: Option<i32>,
    /// Duration in seconds for video and audio media.
    pub duration: Option<i32>,
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

/// Rich-text entity span in a Telegram message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeMessageEntity {
    pub entity_type: MessageEntityType,
    pub offset: i32,
    pub length: i32,
    pub url: Option<String>,
    pub user_id: Option<i64>,
    pub language: Option<String>,
    pub custom_emoji_id: Option<i64>,
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

#[napi(object)]
pub struct NativeRawMessage {
    pub chat_id: i64,
    pub message_id: i32,
    pub raw_message: Vec<u8>,
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
    /// Rich-text entities such as bold, italic, and links.
    pub fmt_entities: Option<Vec<NativeMessageEntity>>,
    /// Sticker metadata.
    pub sticker_info: Option<StickerInfo>,
    /// Generic media metadata for photos, documents, videos, and similar payloads.
    pub media_info: Option<MediaInfo>,
    pub deleted: bool,
    pub send_state: NativeSendState,
}

impl NativeMessage {
    pub fn from_raw(raw: &Message) -> Self {
        let mut sender_id = -1;
        let mut sender_name = "".to_string();
        if raw.sender().is_some() {
            sender_id = raw.sender().unwrap().id().bare_id();
            sender_name = raw.sender().unwrap().name().unwrap_or("").to_string();
        }

        // Extract sticker metadata.
        let sticker_info = if let Some(Media::Sticker(sticker)) = raw.media() {
            let document = &sticker.document;
            let mime = document.mime_type().unwrap_or("");

            // Classify sticker payloads from their MIME type.
            // application/x-tgsticker is an animated TGS/Lottie sticker.
            // video/webm is a video sticker.
            let is_animated = mime == "application/x-tgsticker";
            let is_video = mime == "video/webm" || mime.starts_with("video/");

            // Extract dimensions from the raw document payload.
            // document.raw is already a MessageMediaDocument value.
            let (width, height) =
                if let Some(tl::enums::Document::Document(doc)) = &document.raw.document {
                    // Look for DocumentAttributeImageSize.
                    let mut w = None;
                    let mut h = None;
                    for attr in &doc.attributes {
                        match attr {
                            tl::enums::DocumentAttribute::ImageSize(size) => {
                                w = Some(size.w);
                                h = Some(size.h);
                                break;
                            }
                            _ => {}
                        }
                    }
                    (w, h)
                } else {
                    (None, None)
                };

            Some(StickerInfo {
                emoji: Some(sticker.raw_attrs.alt.clone()),
                is_animated,
                is_video,
                mime_type: Some(mime.to_string()),
                file_size: document.size().map(|size| size as i64),
                width,
                height,
            })
        } else {
            None
        };

        // Parse formatted text entities.
        let fmt_entities: Option<Vec<NativeMessageEntity>> = raw
            .fmt_entities()
            .map(|entities| entities.iter().map(NativeMessageEntity::from_raw).collect());

        // Extract generic media metadata.
        let media_info = match raw.media() {
            Some(Media::Photo(_photo)) => {
                // Photo does not expose a simple file-size accessor here.
                Some(MediaInfo {
                    mime_type: Some("image/jpeg".to_string()),
                    file_name: None,
                    file_size: None,
                    width: None,
                    height: None,
                    duration: None,
                })
            }
            Some(Media::Document(doc)) => {
                let mime = doc.mime_type().unwrap_or("application/octet-stream");

                // Extract metadata from document attributes.
                let (file_name, width, height, duration) =
                    if let Some(tl::enums::Document::Document(d)) = &doc.raw.document {
                        let mut fname = None;
                        let mut w = None;
                        let mut h = None;
                        let mut dur = None;

                        for attr in &d.attributes {
                            match attr {
                                tl::enums::DocumentAttribute::Filename(f) => {
                                    fname = Some(f.file_name.clone());
                                }
                                tl::enums::DocumentAttribute::ImageSize(size) => {
                                    w = Some(size.w);
                                    h = Some(size.h);
                                }
                                tl::enums::DocumentAttribute::Video(video) => {
                                    w = Some(video.w);
                                    h = Some(video.h);
                                    dur = Some(video.duration as i32);
                                }
                                _ => {}
                            }
                        }
                        (fname, w, h, dur)
                    } else {
                        (None, None, None, None)
                    };

                Some(MediaInfo {
                    mime_type: Some(mime.to_string()),
                    file_name,
                    file_size: doc.size().map(|size| size as i64),
                    width,
                    height,
                    duration,
                })
            }
            _ => None,
        };

        Self {
            message_id: raw.id(),
            chat_id: raw.peer_id().bare_id(),
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
            sticker_info,
            media_info,
            deleted: false,
            send_state: NativeSendState::Sent,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum ChatType {
    User,
    Group,
    Channel,
}

impl ChatType {
    pub fn from_chat(raw: &Peer) -> Self {
        match raw {
            Peer::User(_) => ChatType::User,
            Peer::Group(_) => ChatType::Group,
            Peer::Channel(_) => ChatType::Channel,
        }
    }

    pub fn from_dialog(dialog: &Dialog) -> Self {
        Self::from_chat(dialog.peer())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeChat {
    pub chat_id: i64,
    pub chat_type: ChatType,
    pub name: String,
    pub pinned: bool,
    pub unread_count: i32,
    pub read_inbox_max_id: i32,
    pub read_outbox_max_id: i32,
    pub muted: bool,
    pub archived: bool,
    pub last_message_id: i32,
    pub last_message_sender_name: String,
    pub last_message_text: String,
    pub last_message_timestamp: i64,
    pub megagroup: bool,
    pub forum: bool,
    // pub forums: Option<Vec<i64>>,
}

impl NativeChat {
    pub async fn from_raw(raw: &Peer) -> Self {
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
        let megagroup = matches!(raw, Peer::Group(g) if g.is_megagroup());
        let forum = megagroup
            && matches!(raw, Peer::Group(g)
            if matches!(&g.raw, tl::enums::Chat::Channel(c) if c.forum)); // TODO: check this
        Self {
            chat_id: raw.id().bare_id(),
            chat_type: ChatType::from_chat(raw),
            name: raw.name().unwrap_or("").to_string(),
            pinned: false,
            unread_count: 0,
            read_inbox_max_id: 0,
            read_outbox_max_id: 0,
            muted: false,
            archived: false,
            last_message_id: 0,
            last_message_sender_name: "".to_string(),
            last_message_text: "".to_string(),
            last_message_timestamp: 0,
            megagroup,
            forum,
        }
    }

    pub async fn from_dialog(dialog: Dialog) -> Self {
        let chat = dialog.peer();
        let mut last_message_id = 0;
        let mut last_message_sender_name = "".to_string();
        let mut last_message_text = "".to_string();
        let mut last_message_timestamp = 0;
        let (unread_count, read_inbox_max_id, read_outbox_max_id, muted, archived) =
            match &dialog.raw {
                tl::enums::Dialog::Dialog(raw) => {
                    let muted = match &raw.notify_settings {
                        tl::enums::PeerNotifySettings::Settings(settings) => settings
                            .mute_until
                            .map(|until| until as i64 > chrono::Utc::now().timestamp())
                            .unwrap_or(false),
                    };
                    (
                        raw.unread_count,
                        raw.read_inbox_max_id,
                        raw.read_outbox_max_id,
                        muted,
                        raw.folder_id == Some(1),
                    )
                }
                tl::enums::Dialog::Folder(raw) => (
                    raw.unread_muted_messages_count + raw.unread_unmuted_messages_count,
                    0,
                    0,
                    false,
                    false,
                ),
            };
        let megagroup: bool = match chat {
            Peer::User(_) => false,
            Peer::Group(g) => g.is_megagroup(),
            Peer::Channel(_) => false,
        };
        let forum = if megagroup {
            if let Peer::Group(g) = chat {
                if let tl::enums::Chat::Channel(c) = &g.raw {
                    c.forum
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        if let Some(ref message) = dialog.last_message {
            last_message_id = message.id();
            last_message_sender_name = message
                .sender()
                .map(|s| s.name().unwrap_or("").to_string())
                .unwrap_or("".to_string());
            last_message_text = message.text().to_string();
            last_message_timestamp = message.date().timestamp();
        }
        Self {
            chat_id: dialog.peer().id().bare_id(),
            chat_type: ChatType::from_chat(chat),
            name: chat.name().unwrap_or("").to_string(),
            pinned: dialog.raw.pinned(),
            unread_count,
            read_inbox_max_id,
            read_outbox_max_id,
            muted,
            archived,
            last_message_id,
            last_message_sender_name,
            last_message_text,
            last_message_timestamp,
            megagroup,
            forum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativePeer {
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
impl NativePeer {
    pub fn from_raw(raw: &Peer) -> Self {
        match raw {
            Peer::User(user) => Self::from_user(user),
            Peer::Group(group) => Self::from_group(group),
            Peer::Channel(channel) => Self::from_channel(channel),
        }
    }
    pub fn from_user(raw: &User) -> Self {
        Self {
            chat_id: raw.id().bare_id(),
            chat_type: ChatType::User,
            packed_chat: peer_info_to_string(PeerInfo::from(raw)),
            is_contact: raw.contact(),
            is_mutual_contact: raw.mutual_contact(),
            phone: raw.phone().map(|p| p.to_string()),
            username: raw.username().map(|u| u.to_string()),
            full_name: raw.full_name().to_string(),
            first_name: raw.first_name().unwrap_or("").to_string(),
            last_name: raw.last_name().map(|l| l.to_string()),
            bio: None,
            photo_thumb: raw
                .photo()
                .map(|p| p.stripped_thumb.clone())
                .unwrap_or(None),
            date_of_birth: None,
            forum: false,
        }
    }

    pub fn from_group(raw: &Group) -> Self {
        Self {
            chat_id: raw.id().bare_id(),
            chat_type: ChatType::Group,
            packed_chat: peer_info_to_string(PeerInfo::from(raw)),
            is_contact: false,
            is_mutual_contact: false,
            phone: None,
            username: None,
            full_name: raw.title().unwrap_or("").to_string(),
            first_name: raw.title().unwrap_or("").to_string(),
            last_name: None,
            bio: None,
            photo_thumb: raw
                .photo()
                .map(|p| p.stripped_thumb.clone())
                .unwrap_or(None),
            date_of_birth: None,
            forum: false,
        }
    }

    pub fn from_channel(raw: &Channel) -> Self {
        Self {
            chat_id: raw.id().bare_id(),
            chat_type: ChatType::Channel,
            packed_chat: peer_info_to_string(PeerInfo::from(raw)),
            is_contact: false,
            is_mutual_contact: false,
            phone: None,
            username: raw.username().map(|u| u.to_string()),
            full_name: raw.title().to_string(),
            first_name: raw.title().to_string(),
            last_name: None,
            bio: None,
            photo_thumb: raw
                .photo()
                .map(|p| p.stripped_thumb.clone())
                .unwrap_or(None),
            date_of_birth: None,
            forum: raw.raw.forum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl From<NativePeer> for NativeSeenChat {
    fn from(peer: NativePeer) -> Self {
        Self {
            chat_id: peer.chat_id,
            chat_type: peer.chat_type,
            packed_chat: peer.packed_chat,
            is_contact: peer.is_contact,
            is_mutual_contact: peer.is_mutual_contact,
            phone: peer.phone,
            username: peer.username,
            photo_thumb: peer.photo_thumb,
            full_name: peer.full_name,
            first_name: peer.first_name,
            last_name: peer.last_name,
            bio: peer.bio,
            date_of_birth: peer.date_of_birth,
            forum: peer.forum,
        }
    }
}

impl From<NativeSeenChat> for NativePeer {
    fn from(peer: NativeSeenChat) -> Self {
        Self {
            chat_id: peer.chat_id,
            chat_type: peer.chat_type,
            packed_chat: peer.packed_chat,
            is_contact: peer.is_contact,
            is_mutual_contact: peer.is_mutual_contact,
            phone: peer.phone,
            username: peer.username,
            photo_thumb: peer.photo_thumb,
            full_name: peer.full_name,
            first_name: peer.first_name,
            last_name: peer.last_name,
            bio: peer.bio,
            date_of_birth: peer.date_of_birth,
            forum: peer.forum,
        }
    }
}

impl NativeSeenChat {
    pub fn from_raw(raw: &Peer) -> Self {
        NativePeer::from_raw(raw).into()
    }

    pub fn from_user(raw: &User) -> Self {
        NativePeer::from_user(raw).into()
    }

    pub fn from_group(raw: &Group) -> Self {
        NativePeer::from_group(raw).into()
    }

    pub fn from_channel(raw: &Channel) -> Self {
        NativePeer::from_channel(raw).into()
    }
}

/// Chat participant role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum NativeParticipantRole {
    /// Regular user.
    User,
    /// Group or channel creator.
    Creator,
    /// Administrator.
    Admin,
    /// Banned user.
    Banned,
    /// User who left the chat.
    Left,
}

impl From<&Role> for NativeParticipantRole {
    fn from(role: &Role) -> Self {
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

/// Chat participant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeParticipant {
    /// User ID.
    pub user_id: i64,
    /// Full name.
    pub full_name: String,
    /// First name.
    pub first_name: String,
    /// Last name.
    pub last_name: Option<String>,
    /// Public username without the @ prefix.
    pub username: Option<String>,
    /// Participant role.
    pub role: NativeParticipantRole,
    /// Stripped profile photo thumbnail.
    pub photo_thumb: Option<Vec<u8>>,
}

impl NativeParticipant {
    pub fn from_raw(participant: &Participant) -> Self {
        Self {
            user_id: participant.user.id().bare_id(),
            full_name: participant.user.full_name().to_string(),
            first_name: participant.user.first_name().unwrap_or("").to_string(),
            last_name: participant.user.last_name().map(|s| s.to_string()),
            username: participant.user.username().map(|s| s.to_string()),
            role: NativeParticipantRole::from(&participant.role),
            photo_thumb: participant
                .user
                .photo()
                .and_then(|p| p.stripped_thumb.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum NativeLoadSource {
    Cache,
    Network,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum NativeMessageLoadType {
    Initial,
    Backward,
    Forward,
    Around,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum NativeEventKind {
    RuntimeState,
    PeerUpserted,
    DialogUpserted,
    DialogsLoaded,
    MessagesUpserted,
    MessagesLoaded,
    MessageEdited,
    MessagesDeleted,
    ReadStateChanged,
    UploadProgress,
    DownloadProgress,
    ActionFailed,
    UnknownUpdate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi]
pub enum NativeSearchResultKind {
    Chat,
    Message,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeRuntimeOptions {
    pub event_replay_limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeRuntimeState {
    pub authorized: bool,
    pub running: bool,
    pub last_event_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeActionResult {
    pub ok: bool,
    pub message: String,
}

impl NativeActionResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeError {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeDialog {
    pub peer: NativePeer,
    pub chat: NativeChat,
    pub top_message: Option<NativeMessage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeDialogPage {
    pub dialogs: Vec<NativeDialog>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeDialogLoadRequest {
    pub folder_id: Option<i32>,
    pub offset: Option<i64>,
    pub count: Option<u32>,
    pub source: Option<NativeLoadSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeDialogLoadResult {
    pub dialogs: Vec<NativeDialog>,
    pub messages: Vec<NativeMessage>,
    pub has_more: bool,
    pub source: Option<NativeLoadSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeMessagePage {
    pub chat_id: i64,
    pub messages: Vec<NativeMessage>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeMessageLoadRequest {
    pub chat_id: i64,
    pub thread_id: Option<i32>,
    pub count: Option<u32>,
    pub max_id: Option<i32>,
    pub offset_date: Option<i64>,
    pub load_type: Option<NativeMessageLoadType>,
    pub source: Option<NativeLoadSource>,
    pub last_message_id: Option<i32>,
    pub first_unread_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeMessageLoadResult {
    pub chat_id: i64,
    pub chat: Option<NativeChat>,
    pub messages: Vec<NativeMessage>,
    pub has_more: bool,
    pub source: Option<NativeLoadSource>,
    pub load_type: Option<NativeMessageLoadType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeMediaInput {
    pub path: String,
    pub media_type: MediaType,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeSendRequest {
    pub chat_id: i64,
    pub text: String,
    pub reply_to_message_id: Option<i32>,
    pub medias: Option<Vec<NativeMediaInput>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeSendResult {
    pub messages: Vec<NativeMessage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeMediaResult {
    pub local_path: String,
    pub media_type: MediaType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeProfilePhotoPathAndCount {
    pub dir: String,
    pub current: Option<String>,
    pub next: String,
    pub count: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeSearchResult {
    pub kind: NativeSearchResultKind,
    pub chat: Option<NativeChat>,
    pub peer: Option<NativePeer>,
    pub seen_chat: Option<NativePeer>,
    pub message: Option<NativeMessage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeSearchPage {
    pub results: Vec<NativeSearchResult>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeDeletedMessages {
    pub chat_id: Option<i64>,
    pub message_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeReadState {
    pub chat_id: i64,
    pub max_id: i32,
    pub unread_count: i32,
    pub inbox: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeProgress {
    pub chat_id: Option<i64>,
    pub message_id: Option<i32>,
    pub media_index: Option<i32>,
    pub current_progress: i32,
    pub total: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeDialogFlags {
    pub pinned: Option<bool>,
    pub muted: Option<bool>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[napi(object)]
pub struct NativeEvent {
    pub seq: i64,
    pub kind: NativeEventKind,
    pub timestamp: i64,
    pub peer: Option<NativePeer>,
    pub chat: Option<NativeChat>,
    pub dialogs: Option<Vec<NativeDialog>>,
    pub messages: Option<Vec<NativeMessage>>,
    pub message: Option<NativeMessage>,
    pub deleted_messages: Option<NativeDeletedMessages>,
    pub read_state: Option<NativeReadState>,
    pub progress: Option<NativeProgress>,
    pub source: Option<NativeLoadSource>,
    pub load_type: Option<NativeMessageLoadType>,
    pub error: Option<NativeError>,
}

impl NativeEvent {
    pub fn new(kind: NativeEventKind) -> Self {
        Self {
            seq: 0,
            kind,
            timestamp: chrono::Utc::now().timestamp(),
            peer: None,
            chat: None,
            dialogs: None,
            messages: None,
            message: None,
            deleted_messages: None,
            read_state: None,
            progress: None,
            source: None,
            load_type: None,
            error: None,
        }
    }
}
