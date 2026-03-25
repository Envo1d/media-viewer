use crate::core::models::{MediaItem, ScanEvent};
use crate::core::scanner::MediaScanner;
use crossbeam_channel::Receiver;
use std::collections::HashSet;

pub struct ScanManager {
    pub is_scanning: bool,
    pub rx: Option<Receiver<ScanEvent>>,
    pub seen_paths: HashSet<String>,
    pending_items: Vec<MediaItem>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            is_scanning: false,
            rx: None,
            seen_paths: HashSet::new(),
            pending_items: Vec::with_capacity(500),
        }
    }

    pub fn start(&mut self, root_path: String) {
        let (tx, rx) = crossbeam_channel::unbounded();
        self.rx = Some(rx);
        self.is_scanning = true;
        self.seen_paths.clear();
        self.pending_items.clear();

        MediaScanner::start(root_path, tx);
    }

    pub fn update(&mut self) -> (Vec<MediaItem>, bool) {
        let mut new_items = Vec::new();
        let mut finished = false;

        if let Some(rx) = &self.rx {
            for event in rx.try_iter() {
                match event {
                    ScanEvent::Item(item) => {
                        if self.seen_paths.insert(item.path.clone()) {
                            new_items.push(item);
                        }
                    }
                    ScanEvent::Finished => {
                        finished = true;
                    }
                }
            }
        }

        if finished {
            self.is_scanning = false;
            self.rx = None;
        }

        (new_items, finished)
    }
}
