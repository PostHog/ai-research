use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use crossbeam_channel::{Receiver, Sender, bounded};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::adapter::{InputHandle, StorageAdapter};
use crate::config::Config;
use crate::context::Ctx;
use crate::dict::AllowLists;
use crate::metrics::{self, Metrics};

#[derive(Debug)]
struct ChunkJob {
    file_id: u64,
    line_no_start: u64,
    line_no_end: u64,
    offsets: Vec<usize>,
    raw: Vec<u8>,
}

enum OutMsg {
    Open {
        handle: InputHandle,
    },
    Chunk {
        file_id: u64,
        line_no_start: u64,
        line_no_end: u64,
        content: Vec<u8>,
    },
    Eof {
        file_id: u64,
        total_lines: u64,
    },
}

pub fn run(cfg: &Config, adapter: Arc<dyn StorageAdapter>) -> Result<()> {
    let m = Arc::new(Metrics::default());
    let _reporter = metrics::spawn_reporter(Arc::clone(&m), cfg.metrics_interval_ms);

    let writer_workers = cfg.writer_workers.max(1);
    let (file_tx, file_rx) = bounded::<InputHandle>(cfg.file_queue_cap);
    let (chunk_tx, chunk_rx) = bounded::<ChunkJob>(cfg.chunk_queue_cap);
    let mut out_txs = Vec::with_capacity(writer_workers);
    let mut out_rxs = Vec::with_capacity(writer_workers);
    for _ in 0..writer_workers {
        let (tx, rx) = bounded::<OutMsg>(cfg.out_queue_cap);
        out_txs.push(tx);
        out_rxs.push(rx);
    }

    let mut handles: Vec<thread::JoinHandle<Result<()>>> = Vec::new();

    for (shard, rx) in out_rxs.into_iter().enumerate() {
        let adapter = Arc::clone(&adapter);
        let m = Arc::clone(&m);
        handles.push(
            thread::Builder::new()
                .name(format!("writer-{shard}"))
                .spawn(move || writer_loop(adapter, rx, m))?,
        );
    }

    let allow = Arc::new(AllowLists::default());

    for i in 0..cfg.processor_workers {
        let cfg = cfg.clone();
        let chunk_rx = chunk_rx.clone();
        let out_txs = out_txs.clone();
        let allow = Arc::clone(&allow);
        let m = Arc::clone(&m);
        handles.push(
            thread::Builder::new()
                .name(format!("proc-{i}"))
                .spawn(move || processor_loop(cfg, chunk_rx, out_txs, allow, m))?,
        );
    }

    for i in 0..cfg.reader_workers {
        let adapter = Arc::clone(&adapter);
        let file_rx = file_rx.clone();
        let chunk_tx = chunk_tx.clone();
        let out_txs = out_txs.clone();
        let cfg = cfg.clone();
        let m = Arc::clone(&m);
        handles.push(
            thread::Builder::new()
                .name(format!("reader-{i}"))
                .spawn(move || reader_loop(cfg, adapter, file_rx, chunk_tx, out_txs, m))?,
        );
    }
    drop(file_rx);
    drop(chunk_tx);
    drop(out_txs);

    main_loop(Arc::clone(&adapter), file_tx, cfg)?;

    for h in handles {
        match h.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("worker error: {e:#}"),
            Err(_) => eprintln!("worker panicked"),
        }
    }
    Ok(())
}

struct PendingChunk {
    line_no_end: u64,
    content: Vec<u8>,
}

struct FileState {
    handle: InputHandle,
    cursor: u64,
    pending: BTreeMap<u64, PendingChunk>,
    total_lines: Option<u64>,
}

fn writer_loop(
    adapter: Arc<dyn StorageAdapter>,
    rx: Receiver<OutMsg>,
    m: Arc<Metrics>,
) -> Result<()> {
    let mut files: FxHashMap<u64, FileState> = FxHashMap::default();

    while let Ok(msg) = rx.recv() {
        match msg {
            OutMsg::Open { handle } => {
                adapter
                    .open_writer(&handle)
                    .with_context(|| format!("open writer for {}", handle.name))?;
                files.insert(
                    handle.id,
                    FileState {
                        handle,
                        cursor: 0,
                        pending: BTreeMap::new(),
                        total_lines: None,
                    },
                );
            }
            OutMsg::Chunk {
                file_id,
                line_no_start,
                line_no_end,
                content,
            } => {
                let Some(state) = files.get_mut(&file_id) else {
                    eprintln!("writer: chunk for unknown file {file_id}");
                    continue;
                };
                state.pending.insert(
                    line_no_start,
                    PendingChunk {
                        line_no_end,
                        content,
                    },
                );
                flush_contiguous(&adapter, state, &m)?;
                maybe_close(&adapter, &mut files, file_id)?;
            }
            OutMsg::Eof {
                file_id,
                total_lines,
            } => {
                if let Some(state) = files.get_mut(&file_id) {
                    state.total_lines = Some(total_lines);
                    flush_contiguous(&adapter, state, &m)?;
                }
                maybe_close(&adapter, &mut files, file_id)?;
            }
        }
    }

    for (_, mut state) in files.drain() {
        flush_contiguous(&adapter, &mut state, &m)?;
        adapter.close_writer(&state.handle)?;
    }
    Ok(())
}

fn flush_contiguous(
    adapter: &Arc<dyn StorageAdapter>,
    state: &mut FileState,
    m: &Metrics,
) -> Result<()> {
    while let Some(entry) = state.pending.first_entry() {
        if *entry.key() != state.cursor {
            break;
        }
        let (_, chunk) = entry.remove_entry();
        let lines = chunk.line_no_end - state.cursor;
        let bytes = chunk.content.len() as u64;
        adapter.append_chunk(&state.handle, &chunk.content)?;
        state.cursor = chunk.line_no_end;
        m.write_bytes.fetch_add(bytes, Ordering::Relaxed);
        m.write_lines.fetch_add(lines, Ordering::Relaxed);
    }
    Ok(())
}

fn maybe_close(
    adapter: &Arc<dyn StorageAdapter>,
    files: &mut rustc_hash::FxHashMap<u64, FileState>,
    file_id: u64,
) -> Result<()> {
    let done = files
        .get(&file_id)
        .map(|s| s.total_lines.is_some_and(|t| s.cursor >= t) && s.pending.is_empty())
        .unwrap_or(false);
    if done {
        if let Some(state) = files.remove(&file_id) {
            adapter.close_writer(&state.handle)?;
        }
    }
    Ok(())
}

fn processor_loop(
    cfg: Config,
    chunk_rx: Receiver<ChunkJob>,
    out_txs: Vec<Sender<OutMsg>>,
    allow: Arc<AllowLists>,
    m: Arc<Metrics>,
) -> Result<()> {
    let ctx = Ctx::new(&cfg, &allow);
    let mut worker = crate::processor::Worker::default();
    while let Ok(chunk) = chunk_rx.recv() {
        if let Err(e) = forward_chunk(&ctx, &mut worker, chunk, &out_txs, &m) {
            eprintln!("processor: {e:#}");
        }
    }
    Ok(())
}

fn forward_chunk(
    ctx: &Ctx<'_>,
    worker: &mut crate::processor::Worker,
    chunk: ChunkJob,
    out_txs: &[Sender<OutMsg>],
    m: &Metrics,
) -> Result<()> {
    let bytes_in = chunk.raw.len() as u64;
    let lines = chunk.line_no_end - chunk.line_no_start;
    let content = worker.process_chunk(ctx, &chunk.raw, &chunk.offsets);
    let bytes_out = content.len() as u64;
    m.proc_bytes_in.fetch_add(bytes_in, Ordering::Relaxed);
    m.proc_bytes_out.fetch_add(bytes_out, Ordering::Relaxed);
    m.proc_lines.fetch_add(lines, Ordering::Relaxed);
    let shard = get_shard(chunk.file_id, out_txs.len());
    let _ = out_txs[shard].send(OutMsg::Chunk {
        file_id: chunk.file_id,
        line_no_start: chunk.line_no_start,
        line_no_end: chunk.line_no_end,
        content,
    });
    Ok(())
}

fn reader_loop(
    cfg: Config,
    adapter: Arc<dyn StorageAdapter>,
    file_rx: Receiver<InputHandle>,
    chunk_tx: Sender<ChunkJob>,
    out_txs: Vec<Sender<OutMsg>>,
    m: Arc<Metrics>,
) -> Result<()> {
    while let Ok(handle) = file_rx.recv() {
        if let Err(e) = read_one_file(&adapter, &handle, &chunk_tx, &out_txs, &cfg, &m) {
            eprintln!("reader: {} failed: {e:#}", handle.name);
        }
    }
    Ok(())
}

fn get_shard(id: u64, count: usize) -> usize {
    (id % count as u64) as usize
}

fn read_one_file(
    adapter: &Arc<dyn StorageAdapter>,
    handle: &InputHandle,
    chunk_tx: &Sender<ChunkJob>,
    out_txs: &[Sender<OutMsg>],
    cfg: &Config,
    m: &Metrics,
) -> Result<()> {
    let shard = get_shard(handle.id, out_txs.len());

    if out_txs[shard]
        .send(OutMsg::Open {
            handle: handle.clone(),
        })
        .is_err()
    {
        return Ok(());
    }

    let raw = adapter
        .open_reader(handle)
        .with_context(|| format!("open reader for {}", handle.name))?;
    let mut r = BufReader::with_capacity(cfg.outer_read_buf_cap, raw);
    let mut line_no: u64 = 0;

    loop {
        let mut content: Vec<u8> = Vec::with_capacity(cfg.chunk_size_cap);
        let mut offsets: Vec<usize> = Vec::new();

        while content.len() < cfg.chunk_size_cap {
            let start = content.len();
            let n = r
                .read_until(b'\n', &mut content)
                .with_context(|| format!("read {}", handle.name))?;
            if n == 0 {
                break;
            }
            offsets.push(start);
        }

        if offsets.is_empty() {
            break;
        }

        let line_no_start = line_no;
        line_no += offsets.len() as u64;

        let bytes = content.len() as u64;
        let lines = offsets.len() as u64;
        if chunk_tx
            .send(ChunkJob {
                file_id: handle.id,
                line_no_start,
                line_no_end: line_no,
                offsets,
                raw: content,
            })
            .is_err()
        {
            return Ok(());
        }
        m.read_bytes.fetch_add(bytes, Ordering::Relaxed);
        m.read_lines.fetch_add(lines, Ordering::Relaxed);
    }

    let _ = out_txs[shard].send(OutMsg::Eof {
        file_id: handle.id,
        total_lines: line_no,
    });
    Ok(())
}

fn main_loop(
    adapter: Arc<dyn StorageAdapter>,
    file_tx: Sender<InputHandle>,
    cfg: &Config,
) -> Result<()> {
    let backoff_initial = Duration::from_millis(cfg.dispatch_backoff_initial_ms);
    let backoff_max = Duration::from_millis(cfg.dispatch_backoff_max_ms);
    let max_idle = cfg.dispatch_max_idle_iters;

    let mut seen: FxHashSet<u64> = FxHashSet::default();
    let mut backoff = backoff_initial;
    let mut iters_without_change: u32 = 0;
    loop {
        let new_count = match adapter.list_inputs() {
            Ok(handles) => {
                let mut n = 0usize;
                for h in handles {
                    if seen.insert(h.id) {
                        if file_tx.send(h).is_err() {
                            return Ok(());
                        }
                        n += 1;
                    }
                }
                n
            }
            Err(e) => {
                eprintln!("dispatch: list_inputs failed: {e:#}");
                0
            }
        };
        if new_count > 0 {
            backoff = backoff_initial;
            iters_without_change = 0;
        } else {
            thread::sleep(backoff);
            backoff = (backoff * 2).min(backoff_max);
            iters_without_change += 1;
        }

        if iters_without_change > max_idle {
            break;
        }
    }
    Ok(())
}
