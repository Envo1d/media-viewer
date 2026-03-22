#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MediaItem {
    pub path: String,
    pub name: String,
    pub media_type: MediaType,
    pub category: String,
    pub author: String,
    pub modified: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MediaType {
    Image,
    Video,
}

#[derive(Clone)]
pub enum ScanEvent {
    Item(MediaItem),
    Finished,
}
