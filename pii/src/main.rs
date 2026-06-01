#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod adapter;
mod config;
mod context;
mod dict;
mod metrics;
mod pipeline;
mod processor;
mod schema;
mod scrub;

use std::path::PathBuf;

use anyhow::Result;

use crate::adapter::FsAdapter;
use crate::config::Config;

fn main() -> Result<()> {
    let mut cfg = Config::default();
    let mut args = std::env::args().skip(1);
    if let Some(v) = args.next() {
        cfg.in_dir = PathBuf::from(v);
    }
    if let Some(v) = args.next() {
        cfg.out_dir = PathBuf::from(v);
    }

    let adapter = std::sync::Arc::new(FsAdapter::new(
        cfg.in_dir.clone(),
        cfg.out_dir.clone(),
        cfg.reader_buf_cap,
        cfg.writer_buf_cap,
    )?);

    pipeline::run(&cfg, adapter)
}
