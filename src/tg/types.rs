use grammers_client::grammers_tl_types as tl;
use grammers_client::types::Chat;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, PartialEq)]

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

#[derive(Clone)]

pub struct NativePackedChat {
    pub chat_id: i64,
    pub packed_chat: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct NativeMessage {
    pub message_id: i32,
    pub chat_id: i64,
    pub outgoing: bool,
    pub pinned: bool,
    pub sender_id: i64,
    pub sender_name: String,
    pub timestamp: i64,
    pub text: String,
    pub media_type: MediaType,
    pub edit_timestamp: Option<i64>,
    pub grouped_id: Option<i64>,
    pub reply_to_message_id: Option<i32>,
}

impl NativeMessage {
    pub fn from_raw(raw: &grammers_client::types::Message) -> Self {
        let mut sender_id = -1;
        let mut sender_name = "".to_string();
        if raw.sender().is_some() {
            sender_id = raw.sender().unwrap().id();
            sender_name = raw.sender().unwrap().name().to_string();
        }
        Self {
            message_id: raw.id(),
            chat_id: raw.chat().id(),
            outgoing: raw.outgoing(),
            pinned: raw.pinned(),
            sender_id,
            sender_name,
            timestamp: raw.date().timestamp(),
            text: raw.text().to_string(),
            media_type: MediaType::from(raw.media()),
            edit_timestamp: raw.edit_date().map(|d| d.timestamp()),
            grouped_id: raw.grouped_id(),
            reply_to_message_id: raw.reply_to_message_id(),
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