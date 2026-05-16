use crate::core::models::{MediaItem, MediaType};

pub fn map_media_item(row: &rusqlite::Row) -> rusqlite::Result<MediaItem> {
    let media_type_str: String = row.get(4)?;
    let media_type = match media_type_str.as_str() {
        "Video" => MediaType::Video,
        _ => MediaType::Image,
    };

    let characters_raw: String = row.get(6).unwrap_or_default();
    let tags_raw: String = row.get(7).unwrap_or_default();

    Ok(MediaItem {
        path: row.get(0)?,
        name: row.get(1)?,
        copyright: row.get(2)?,
        artist: row.get(3)?,
        media_type,
        modified: row.get(5)?,
        characters: MediaItem::parse_pipe_list(&characters_raw),
        tags: MediaItem::parse_pipe_list(&tags_raw),
    })
}
