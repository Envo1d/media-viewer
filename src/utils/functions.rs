use crate::models::{MediaItem, MediaType};

pub fn build_search_query(input: &str) -> String {
    input
        .split_whitespace()
        .map(|w| format!("{}*", w))
        .collect::<Vec<_>>()
        .join(" AND ")
}

pub fn map_media_item(row: &rusqlite::Row) -> rusqlite::Result<MediaItem> {
    let media_type_str: String = row.get(4)?;
    let media_type = match media_type_str.as_str() {
        "Video" => MediaType::Video,
        _ => MediaType::Image,
    };

    Ok(MediaItem {
        path: row.get(0)?,
        name: row.get(1)?,
        category: row.get(2)?,
        author: row.get(3)?,
        media_type,
        modified: row.get(5)?,
    })
}
