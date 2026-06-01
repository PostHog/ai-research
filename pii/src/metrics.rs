use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Default, Debug)]
pub struct Metrics {
    pub read_bytes: AtomicU64,
    pub read_lines: AtomicU64,
    pub proc_bytes_in: AtomicU64,
    pub proc_bytes_out: AtomicU64,
    pub proc_lines: AtomicU64,
    pub write_bytes: AtomicU64,
    pub write_lines: AtomicU64,
}

#[derive(Clone, Copy)]
struct Snapshot {
    at: Instant,
    read_bytes: u64,
    read_lines: u64,
    proc_bytes_in: u64,
    proc_bytes_out: u64,
    proc_lines: u64,
    write_bytes: u64,
    write_lines: u64,
}

pub fn spawn_reporter(metrics: Arc<Metrics>, interval_ms: u64) -> Option<thread::JoinHandle<()>> {
    if interval_ms == 0 {
        return None;
    }
    let interval = Duration::from_millis(interval_ms);
    thread::Builder::new()
        .name("metrics".into())
        .spawn(move || {
            let mut last = snapshot(&metrics);
            let mut first_active: Option<Instant> = None;
            let mut last_active: Option<Instant> = None;
            loop {
                thread::sleep(interval);
                let now = snapshot(&metrics);
                if now.write_bytes > last.write_bytes {
                    first_active.get_or_insert(last.at);
                    last_active = Some(now.at);
                }
                report(&last, &now, first_active, last_active);
                last = now;
            }
        })
        .ok()
}

fn snapshot(m: &Metrics) -> Snapshot {
    Snapshot {
        at: Instant::now(),
        read_bytes: m.read_bytes.load(Ordering::Relaxed),
        read_lines: m.read_lines.load(Ordering::Relaxed),
        proc_bytes_in: m.proc_bytes_in.load(Ordering::Relaxed),
        proc_bytes_out: m.proc_bytes_out.load(Ordering::Relaxed),
        proc_lines: m.proc_lines.load(Ordering::Relaxed),
        write_bytes: m.write_bytes.load(Ordering::Relaxed),
        write_lines: m.write_lines.load(Ordering::Relaxed),
    }
}

fn report(
    prev: &Snapshot,
    now: &Snapshot,
    first_active: Option<Instant>,
    last_active: Option<Instant>,
) {
    let dt = now.at.duration_since(prev.at).as_secs_f64();
    if dt <= 0.0 {
        return;
    }

    let r_bps = (now.read_bytes - prev.read_bytes) as f64 / dt;
    let r_lps = (now.read_lines - prev.read_lines) as f64 / dt;
    let p_bps_in = (now.proc_bytes_in - prev.proc_bytes_in) as f64 / dt;
    let p_bps_out = (now.proc_bytes_out - prev.proc_bytes_out) as f64 / dt;
    let p_lps = (now.proc_lines - prev.proc_lines) as f64 / dt;
    let w_bps = (now.write_bytes - prev.write_bytes) as f64 / dt;
    let w_lps = (now.write_lines - prev.write_lines) as f64 / dt;

    let (e2e_bps, e2e_lps) = match (first_active, last_active) {
        (Some(s), Some(e)) => {
            let active = e.duration_since(s).as_secs_f64().max(1e-9);
            (
                now.write_bytes as f64 / active,
                now.write_lines as f64 / active,
            )
        }
        _ => (0.0, 0.0),
    };

    eprintln!(
        "[metrics] e2e {} {} (total {} {}) | read {} {} | proc {} → {} {} | write {} {}",
        fmt_bps(e2e_bps),
        fmt_lps(e2e_lps),
        fmt_bytes(now.write_bytes),
        fmt_count(now.write_lines),
        fmt_bps(r_bps),
        fmt_lps(r_lps),
        fmt_bps(p_bps_in),
        fmt_bps(p_bps_out),
        fmt_lps(p_lps),
        fmt_bps(w_bps),
        fmt_lps(w_lps),
    );
}

fn fmt_bps(bps: f64) -> String {
    if bps >= 1e9 {
        format!("{:>6.2} GB/s", bps / 1e9)
    } else if bps >= 1e6 {
        format!("{:>6.2} MB/s", bps / 1e6)
    } else if bps >= 1e3 {
        format!("{:>6.2} KB/s", bps / 1e3)
    } else {
        format!("{:>6.0}  B/s", bps)
    }
}

fn fmt_lps(lps: f64) -> String {
    if lps >= 1e6 {
        format!("{:>6.2}M lines/s", lps / 1e6)
    } else if lps >= 1e3 {
        format!("{:>6.2}k lines/s", lps / 1e3)
    } else {
        format!("{:>6.0}  lines/s", lps)
    }
}

fn fmt_bytes(b: u64) -> String {
    let b = b as f64;
    if b >= 1e9 {
        format!("{:.2} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.2} MB", b / 1e6)
    } else if b >= 1e3 {
        format!("{:.2} KB", b / 1e3)
    } else {
        format!("{} B", b as u64)
    }
}

fn fmt_count(n: u64) -> String {
    let n = n as f64;
    if n >= 1e6 {
        format!("{:.2}M lines", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.2}k lines", n / 1e3)
    } else {
        format!("{} lines", n as u64)
    }
}
