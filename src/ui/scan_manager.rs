use crate::core::models::{ScanEvent, WatchEvent};
use crate::core::scanner::MediaScanner;
use crate::core::watcher::FileWatcher;
use crate::data::db_worker::get_db;
use crossbeam_channel::Receiver;

pub struct ScanManager {
    pub is_scanning: bool,
    pub files_scanned: u64,

    scan_rx: Option<Receiver<ScanEvent>>,

    watcher: Option<FileWatcher>,
    watched_path: Option<String>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            is_scanning: false,
            files_scanned: 0,
            scan_rx: None,
            watcher: None,
            watched_path: None,
        }
    }

    pub fn start(&mut self, root_path: String) {
        if self.is_scanning {
            return;
        }

        if root_path.is_empty() {
            return;
        }

        self.watcher = None;
        self.watched_path = Some(root_path.clone());

        let (tx, rx) = crossbeam_channel::unbounded();
        self.scan_rx = Some(rx);
        self.is_scanning = true;
        self.files_scanned = 0;

        MediaScanner::start(root_path, tx, get_db().clone());
    }

    pub fn start_watching(&mut self, root_path: String) {
        if root_path.is_empty() || self.watcher.is_some() {
            return;
        }
        self.watched_path = Some(root_path.clone());
        self.watcher = FileWatcher::start(root_path);
    }

    fn drain_scan_events(&mut self) -> bool {
        let mut finished = false;

        if let Some(rx) = &self.scan_rx {
            for event in rx.try_iter() {
                match event {
                    ScanEvent::Progress(n) => {
                        self.files_scanned += n;
                    }
                    ScanEvent::Finished => {
                        finished = true;
                    }
                }
            }
        }

        if finished {
            self.is_scanning = false;
            self.scan_rx = None;

            if let Some(path) = self.watched_path.clone() {
                self.watcher = FileWatcher::start(path);
            }
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
                WatchEvent::Refresh => {
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn update(&mut self) -> (bool, bool) {
        let scan_finished = self.drain_scan_events();
        let watch_changed = self.drain_watch_events();
        (scan_finished, watch_changed)
    }
}
