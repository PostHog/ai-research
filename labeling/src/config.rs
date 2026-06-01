use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub in_dir: PathBuf,
    pub out_dir: PathBuf,

    pub reader_workers: usize,
    pub processor_workers: usize,
    pub writer_workers: usize,

    pub file_queue_cap: usize,
    pub chunk_queue_cap: usize,
    pub chunk_size_cap: usize,
    pub out_queue_cap: usize,

    pub reader_buf_cap: usize,
    pub writer_buf_cap: usize,
    pub outer_read_buf_cap: usize,

    pub dispatch_backoff_initial_ms: u64,
    pub dispatch_backoff_max_ms: u64,
    pub dispatch_max_idle_iters: u32,

    pub max_words_len: usize,

    pub metrics_interval_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8);
        Self {
            in_dir: PathBuf::from("in"),
            out_dir: PathBuf::from("out"),
            reader_workers: 2.max(cores / 8),
            processor_workers: cores.saturating_sub(1).max(2),
            writer_workers: 2.max(cores / 8),
            file_queue_cap: 1024,
            chunk_queue_cap: 4096,
            chunk_size_cap: 1 << 19,
            out_queue_cap: 4096,
            reader_buf_cap: 1 << 20,
            writer_buf_cap: 1 << 20,
            outer_read_buf_cap: 4 * 1024,
            dispatch_backoff_initial_ms: 50,
            dispatch_backoff_max_ms: 1000,
            dispatch_max_idle_iters: 10,
            max_words_len: 8,
            metrics_interval_ms: 1000,
        }
    }
}

impl Config {
    pub const REDACT_CHAR: char = '*';
    pub const NUMBER_CHAR: char = '#';
}
