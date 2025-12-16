use crate::tg::BASE_PATH;
use const_format::concatcp;
use napi_derive_ohos::napi;
use napi_ohos::Error;

type Result<T> = std::result::Result<T, Error>;

const MEDIAS_DIR: &str = concatcp!(BASE_PATH, "downloads/");

pub fn get_download_dir(chat_id: i64) -> String {
    format!("{}/{}/", MEDIAS_DIR, chat_id)
}

/// 根据 MIME 类型推断文件扩展名
pub fn get_extension_from_mime(mime_type: Option<&str>) -> &'static str {
    match mime_type {
        Some("image/jpeg") | Some("image/jpg") => "jpg",
        Some("image/png") => "png",
        Some("image/gif") => "gif",
        Some("image/webp") => "webp",
        Some("video/mp4") => "mp4",
        Some("video/webm") => "webm",
        Some("video/quicktime") => "mov",
        Some("video/x-matroska") => "mkv",
        Some("audio/mpeg") | Some("audio/mp3") => "mp3",
        Some("audio/ogg") => "ogg",
        Some("audio/wav") => "wav",
        Some("audio/aac") => "aac",
        Some("application/pdf") => "pdf",
        Some("application/x-tgsticker") => "tgs",
        Some("application/zip") => "zip",
        Some("application/x-rar-compressed") => "rar",
        Some(mime) if mime.starts_with("video/") => "mp4",  // 默认视频
        Some(mime) if mime.starts_with("audio/") => "mp3",  // 默认音频
        Some(mime) if mime.starts_with("image/") => "jpg",  // 默认图片
        _ => "bin",  // 未知类型
    }
}

/// 从文件名中提取扩展名
pub fn get_extension_from_filename(filename: &str) -> Option<&str> {
    filename.rsplit('.').next().filter(|ext| !ext.is_empty() && ext.len() < 10)
}

/// 根据媒体信息构建下载路径（带正确扩展名）
pub fn get_media_path_with_extension(
    chat_id: i64, 
    message_id: i32, 
    mime_type: Option<&str>,
    file_name: Option<&str>
) -> String {
    // 优先使用文件名中的扩展名
    let extension = if let Some(fname) = file_name {
        get_extension_from_filename(fname).unwrap_or_else(|| get_extension_from_mime(mime_type))
    } else {
        get_extension_from_mime(mime_type)
    };
    
    format!("{}/{}/{}.{}", MEDIAS_DIR, chat_id, message_id, extension)
}

/// 旧的硬编码路径函数（保留以兼容性）
#[deprecated(note = "Use get_media_path_with_extension instead")]
pub fn get_media_path(chat_id: i64, message_id: i32) -> String {
    format!("{}/{}/{}.jpg", MEDIAS_DIR, chat_id, message_id)
}

pub fn get_sticker_path(chat_id: i64, message_id: i32, is_animated: bool, is_video: bool) -> String {
    let extension = if is_video {
        "webm"
    } else if is_animated {
        "tgs"
    } else {
        "webp"  // 静态 sticker 通常是 webp
    };
    format!("{}/{}/{}.{}", MEDIAS_DIR, chat_id, message_id, extension)
}

#[derive(Debug)]
#[napi]
pub struct ProfilePhotoPath {
    pub dir: String,
    pub current: Option<String>,
    pub next: String,
    pub count: i32,
}

/// Get the path of the profile photo of a chat.
///
/// # Arguments
///
/// * `chat_id` - the id of the chat.
/// * `current` - if true, return the path of the current profile photo,
/// otherwise return the path of the next profile photo.
#[napi]
pub fn get_profile_photo_path_and_count(chat_id: i64) -> Result<ProfilePhotoPath> {
    let dir = format!("{}/{}/{}", MEDIAS_DIR, chat_id, "profile_photos/");
    std::fs::create_dir_all(&dir)?;
    // check the count of current profile photos
    let n_profile_photos = std::fs::read_dir(&dir)?.count();

    // we let the first profile photo be 1.jpg
    Ok(ProfilePhotoPath {
        dir: dir.clone(),
        current: if n_profile_photos == 0 {
            None
        } else {
            Some(format!("{}{}.jpg", dir, n_profile_photos.to_string()))
        },
        next: format!("{}{}.jpg", dir, (n_profile_photos + 1).to_string()),
        count: n_profile_photos as i32,
    })
}
