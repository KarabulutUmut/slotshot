//! Opt-in local timing only: no image bytes, paths, titles, or selection coordinates.
use std::{
    io::Write,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
static NEXT: AtomicU64 = AtomicU64::new(1);
static ENABLED: OnceLock<bool> = OnceLock::new();
#[derive(Clone, Copy)]
pub struct Timing {
    id: u64,
    start: Instant,
}
impl Timing {
    pub fn new(source: &'static str) -> Self {
        let timing = Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            start: Instant::now(),
        };
        timing.mark(source);
        timing
    }
    pub fn mark(self, stage: &'static str) {
        if !*ENABLED.get_or_init(|| std::env::args().any(|arg| arg == "--capture-timings")) {
            return;
        }
        let line = format!(
            "{{\"capture\":{},\"stage\":\"{}\",\"ms\":{:.3}}}\n",
            self.id,
            stage,
            self.start.elapsed().as_secs_f64() * 1000.0
        );
        eprint!("{line}");
        let path = std::env::temp_dir().join(format!(
            "slotshot-capture-timings-{}.jsonl",
            std::process::id()
        ));
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            && file.metadata().is_ok_and(|m| m.len() < 512 * 1024)
        {
            let _ = file.write_all(line.as_bytes());
        }
    }
}
