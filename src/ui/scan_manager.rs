use crate::core::models::ScanEvent;
use crate::core::scanner::MediaScanner;
use crate::data::db_worker::get_db;
use crossbeam_channel::Receiver;

pub struct ScanManager {
    pub is_scanning: bool,
    pub files_scanned: u64,
    rx: Option<Receiver<ScanEvent>>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            is_scanning: false,
            files_scanned: 0,
            rx: None,
        }
    }

    pub fn start(&mut self, root_path: String) {
        if self.is_scanning {
            return;
        }

        if root_path.is_empty() {
            return;
        }

        let (tx, rx) = crossbeam_channel::unbounded();
        self.rx = Some(rx);
        self.is_scanning = true;
        self.files_scanned = 0;

        MediaScanner::start(root_path, tx, get_db().clone());
    }

    pub fn update(&mut self) -> bool {
        let mut finished = false;

        if let Some(rx) = &self.rx {
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
            self.rx = None;
        }

        finished
    }
}
