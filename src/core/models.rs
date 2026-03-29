use crossbeam_channel::Sender;
use std::cmp::Ordering;
use std::time::Instant;

pub enum DbCommand {
    UpsertBatch(Vec<MediaItem>, i64),
    DeleteNotSeen(i64),
    Query {
        limit: usize,
        offset: usize,
        resp: Sender<Vec<MediaItem>>,
    },
    Search {
        query: String,
        limit: usize,
        offset: usize,
        resp: Sender<Vec<MediaItem>>,
    },
}

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

pub struct TextureTask {
    pub priority: i32, // 0 - visible, 10 - prefetch
    pub path: String,
    pub timestamp: Instant,
}

impl Ord for TextureTask {
    fn cmp(&self, other: &Self) -> Ordering {
        let p_res = other.priority.cmp(&self.priority);
        if p_res != Ordering::Equal {
            return p_res;
        }

        self.timestamp.cmp(&other.timestamp)
    }
}

impl PartialOrd for TextureTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TextureTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
            && self.timestamp == other.timestamp
            && self.path == other.path
    }
}

impl Eq for TextureTask {}
