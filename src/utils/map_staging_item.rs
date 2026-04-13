use crate::core::models::{MediaType, StagingItem};

pub fn map_staging_item(row: &rusqlite::Row) -> rusqlite::Result<StagingItem> {
    let t: String = row.get(2)?;
    Ok(StagingItem {
        path: row.get(0)?,
        name: row.get(1)?,
        media_type: if t == "Video" {
            MediaType::Video
        } else {
            MediaType::Image
        },
        modified: row.get(3)?,
    })
}
