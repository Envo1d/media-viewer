use crate::core::models::{DbCommand, MediaItem, PendingKind, WatchEvent};
use crate::data::db_worker::get_db;
use crate::utils::current_timestamp;
use crate::utils::{build_media_item, is_media_path};
use crossbeam_channel::{bounded, Receiver, Sender};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 500;

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    pub event_rx: Receiver<WatchEvent>,
}

fn flush_ready(
    pending: &mut HashMap<PathBuf, (PendingKind, Instant)>,
    db_tx: &Sender<DbCommand>,
    watch_tx: &Sender<WatchEvent>,
    root: &str,
    debounce: Duration,
) {
    if pending.is_empty() {
        return;
    }

    let now = Instant::now();

    let ready: Vec<(PathBuf, PendingKind)> = pending
        .iter()
        .filter(|(_, (_, t))| now.duration_since(*t) >= debounce)
        .map(|(p, (k, _))| (p.clone(), k.clone()))
        .collect();

    if ready.is_empty() {
        return;
    }

    let scan_id = current_timestamp();
    let mut upserts: Vec<Arc<MediaItem>> = Vec::new();
    let mut changed = false;

    for (path, kind) in &ready {
        pending.remove(path);

        match kind {
            PendingKind::Delete => {
                db_tx
                    .send(DbCommand::DeleteByPath(path.to_string_lossy().to_string()))
                    .ok();
                changed = true;
            }
            PendingKind::Upsert => {
                if let Some(item) = build_media_item(root, path) {
                    upserts.push(item);
                    changed = true;
                }
            }
        }
    }

    if !upserts.is_empty() {
        db_tx.send(DbCommand::UpsertBatch(upserts, scan_id)).ok();
    }

    if changed {
        watch_tx.try_send(WatchEvent::Refresh).ok();
    }
}

fn debounce_loop(
    raw_rx: Receiver<notify::Result<Event>>,
    db_tx: Sender<DbCommand>,
    watch_tx: Sender<WatchEvent>,
    root: String,
) {
    let mut pending: HashMap<PathBuf, (PendingKind, Instant)> = HashMap::new();
    let debounce = Duration::from_millis(DEBOUNCE_MS);
    let half = debounce / 2;

    loop {
        match raw_rx.recv_timeout(half) {
            Ok(Ok(event)) => {
                let kind = match event.kind {
                    EventKind::Remove(_) => PendingKind::Delete,
                    EventKind::Create(_) | EventKind::Modify(_) => PendingKind::Upsert,
                    _ => {
                        flush_ready(&mut pending, &db_tx, &watch_tx, &root, debounce);
                        continue;
                    }
                };

                for path in event.paths {
                    let relevant = matches!(kind, PendingKind::Delete) || is_media_path(&path);
                    if relevant {
                        pending.insert(path, (kind.clone(), Instant::now()));
                    }
                }
            }
            Ok(Err(e)) => eprintln!("[watcher] notify error: {e:?}"),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }

        flush_ready(&mut pending, &db_tx, &watch_tx, &root, debounce);
    }
}

impl FileWatcher {
    pub fn start(root_path: String) -> Option<Self> {
        let (raw_tx, raw_rx) = bounded::<notify::Result<Event>>(1024);
        let (watch_tx, watch_rx) = bounded::<WatchEvent>(32);

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = raw_tx.try_send(res);
            },
            Config::default(),
        )
        .map_err(|e| eprintln!("[watcher] failed to create watcher: {e:?}"))
        .ok()?;

        watcher
            .watch(Path::new(&root_path), RecursiveMode::Recursive)
            .map_err(|e| eprintln!("[watcher] failed to watch {root_path:?}: {e:?}"))
            .ok()?;

        let db_tx = get_db().clone();
        let root = root_path.clone();
        let watch_tx_bg = watch_tx;

        thread::Builder::new()
            .name("nexa-watcher-debounce".into())
            .spawn(move || debounce_loop(raw_rx, db_tx, watch_tx_bg, root))
            .ok()?;

        eprintln!("[watcher] watching: {root_path}");

        Some(Self {
            _watcher: watcher,
            event_rx: watch_rx,
        })
    }
}
