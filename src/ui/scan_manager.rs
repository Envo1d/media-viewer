use crate::core::models::{ScanEvent, WatchEvent};
use crate::core::scanner::MediaScanner;
use crate::core::staging_scanner::StagingScanner;
use crate::core::watcher::FileWatcher;
use crate::data::db_worker::get_db;
use crate::infra::config::FolderMapping;
use crossbeam_channel::{bounded, Receiver};

const MAX_SCAN_EVENTS_PER_FRAME: usize = 32;
const SCAN_EVENT_CAPACITY: usize = 256;

pub struct ScanManager {
    pub is_scanning: bool,
    pub files_scanned: u64,

    pub is_staging_scanning: bool,
    pub staging_files_scanned: u64,

    scan_rx: Option<Receiver<ScanEvent>>,
    staging_scan_rx: Option<Receiver<ScanEvent>>,

    watcher: Option<FileWatcher>,
    watched_path: Option<String>,

    pending_mapping: Option<FolderMapping>,
    pending_char_sep: Option<String>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            is_scanning: false,
            files_scanned: 0,
            is_staging_scanning: false,
            staging_files_scanned: 0,
            scan_rx: None,
            staging_scan_rx: None,
            watcher: None,
            watched_path: None,
            pending_mapping: None,
            pending_char_sep: None,
        }
    }

    pub fn start(
        &mut self,
        root_path: String,
        mapping: FolderMapping,
        char_sep: String,
        excluded_dirs: Vec<String>,
    ) {
        if self.is_scanning || root_path.is_empty() {
            return;
        }

        self.watcher = None;
        self.watched_path = Some(root_path.clone());
        self.pending_mapping = Some(mapping.clone());
        self.pending_char_sep = Some(char_sep.clone());

        let (tx, rx) = bounded(SCAN_EVENT_CAPACITY);
        self.scan_rx = Some(rx);
        self.is_scanning = true;
        self.files_scanned = 0;

        MediaScanner::start(
            root_path,
            mapping,
            char_sep,
            excluded_dirs,
            tx,
            get_db().clone(),
        );
    }

    pub fn start_staging(&mut self, staging_path: String) {
        if self.is_staging_scanning || staging_path.is_empty() {
            return;
        }

        let (tx, rx) = bounded(SCAN_EVENT_CAPACITY);
        self.staging_scan_rx = Some(rx);
        self.is_staging_scanning = true;
        self.staging_files_scanned = 0;

        StagingScanner::start(staging_path, tx, get_db().clone());
    }

    pub fn start_watching(&mut self, root_path: String, mapping: FolderMapping, char_sep: String) {
        if root_path.is_empty() || self.watcher.is_some() {
            return;
        }
        self.watched_path = Some(root_path.clone());
        self.watcher = FileWatcher::start(root_path, mapping, char_sep);
    }

    fn drain_scan_events(&mut self) -> bool {
        let mut finished = false;

        if let Some(rx) = &self.scan_rx {
            for event in rx.try_iter().take(MAX_SCAN_EVENTS_PER_FRAME) {
                match event {
                    ScanEvent::Progress(n) => self.files_scanned += n,
                    ScanEvent::Finished => finished = true,
                }
            }
        }

        if finished {
            self.is_scanning = false;
            self.scan_rx = None;

            if let (Some(path), Some(mapping), Some(char_sep)) = (
                self.watched_path.clone(),
                self.pending_mapping.take(),
                self.pending_char_sep.take(),
            ) {
                self.watcher = FileWatcher::start(path, mapping, char_sep);
            }
        }

        finished
    }

    fn drain_staging_scan_events(&mut self) -> bool {
        let mut finished = false;

        if let Some(rx) = &self.staging_scan_rx {
            for event in rx.try_iter().take(MAX_SCAN_EVENTS_PER_FRAME) {
                match event {
                    ScanEvent::Progress(n) => self.staging_files_scanned += n,
                    ScanEvent::Finished => finished = true,
                }
            }
        }

        if finished {
            self.is_staging_scanning = false;
            self.staging_scan_rx = None;
        }

        finished
    }

    fn drain_watch_events(&mut self) -> bool {
        let Some(watcher) = &self.watcher else {
            return false;
        };

        let mut changed = false;

        for event in watcher.event_rx.try_iter() {
            match event {
                WatchEvent::Refresh => changed = true,
            }
        }
        changed
    }

    pub fn update(&mut self) -> (bool, bool, bool) {
        let scan_finished = self.drain_scan_events();
        let staging_finished = self.drain_staging_scan_events();
        let watch_changed = self.drain_watch_events();
        (scan_finished, staging_finished, watch_changed)
    }
}
