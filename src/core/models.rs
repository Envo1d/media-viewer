use crossbeam_channel::Sender;
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Instant;
// Filter

#[derive(Clone, PartialEq, Default)]
pub enum MediaFilter {
    #[default]
    All,
    Images,
    Videos,
}

impl MediaFilter {
    pub fn to_sql(&self) -> &'static str {
        match self {
            Self::All => "",
            Self::Images => "AND media_type = 'Image'",
            Self::Videos => "AND media_type = 'Video'",
        }
    }

    pub fn to_sql_fts(&self) -> &'static str {
        match self {
            Self::All => "",
            Self::Images => "AND m.media_type = 'Image'",
            Self::Videos => "AND m.media_type = 'Video'",
        }
    }
}

// Sort

#[derive(Clone, PartialEq, Default)]
pub enum SortOrder {
    #[default]
    NameAsc,
    NameDesc,
    DateDesc,
    DateAsc,
}

impl SortOrder {
    pub fn to_sql(&self) -> &'static str {
        match self {
            Self::NameAsc => "ORDER BY name ASC",
            Self::NameDesc => "ORDER BY name DESC",
            Self::DateDesc => "ORDER BY modified DESC",
            Self::DateAsc => "ORDER BY modified ASC",
        }
    }

    pub fn to_sql_fts(&self) -> &'static str {
        match self {
            Self::NameAsc => "ORDER BY m.name ASC",
            Self::NameDesc => "ORDER BY m.name DESC",
            Self::DateDesc => "ORDER BY m.modified DESC",
            Self::DateAsc => "ORDER BY m.modified ASC",
        }
    }
}

// Domain types

pub enum DbCommand {
    UpsertBatch(Vec<Arc<MediaItem>>, i64),
    DeleteNotSeen(i64),
    DeleteByPath(String),
    Query {
        id: u64,
        limit: usize,
        offset: usize,
        filter: MediaFilter,
        sort: SortOrder,
        resp: Sender<(u64, Vec<Arc<MediaItem>>)>,
    },
    Search {
        id: u64,
        query: String,
        limit: usize,
        offset: usize,
        filter: MediaFilter,
        sort: SortOrder,
        resp: Sender<(u64, Vec<Arc<MediaItem>>)>,
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MediaType {
    Image,
    Video,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Image => "Image",
            MediaType::Video => "Video",
        }
    }
}

// Events

#[derive(Clone)]
pub enum ScanEvent {
    Progress(u64),
    Finished,
}

pub enum WatchEvent {
    Refresh,
}

#[derive(Clone)]
pub enum PendingKind {
    Upsert,
    Delete,
}

// Texture task

pub struct TextureTask {
    pub priority: i32, // 0 = visible, 10 = prefetch
    pub path: String,
    pub timestamp: Instant,
    pub generation: u64,
}

impl Ord for TextureTask {
    fn cmp(&self, other: &Self) -> Ordering {
        let p = other.priority.cmp(&self.priority);
        if p != Ordering::Equal {
            return p;
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
