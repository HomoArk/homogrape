use crate::tg::BASE_PATH;
use anyhow::Result;
use const_format::concatcp;

const MEDIAS_DIR: &str = concatcp!(BASE_PATH, "downloads/");

pub fn get_download_dir(chat_id: i64) -> String {
    format!("{}/{}/", MEDIAS_DIR, chat_id)
}

pub fn get_media_path(chat_id: i64, message_id: i32) -> String {
    format!("{}/{}/{}.jpg", MEDIAS_DIR, chat_id, message_id)
}

#[derive(Debug)]

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
