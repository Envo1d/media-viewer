use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static LAST: AtomicI64 = AtomicI64::new(0);

pub fn current_timestamp() -> i64 {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    loop {
        let prev = LAST.load(Ordering::Relaxed);
        let next = ms.max(prev + 1);
        if LAST
            .compare_exchange(prev, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return next;
        }
    }
}
