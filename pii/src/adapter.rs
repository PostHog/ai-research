
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use rustc_hash::FxHashMap;
use walkdir::WalkDir;

pub trait StorageAdapter: Send + Sync {
    fn list_inputs(&self) -> Result<Vec<InputHandle>>;
    fn open_reader(&self, h: &InputHandle) -> Result<Box<dyn Read + Send>>;
    fn open_writer(&self, h: &InputHandle) -> Result<()>;
    fn append_chunk(&self, h: &InputHandle, chunk: &[u8]) -> Result<()>;
    fn close_writer(&self, h: &InputHandle) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputHandle {
    pub id: u64,
    pub name: String,
}

pub struct FsAdapter {
    in_dir: PathBuf,
    out_dir: PathBuf,
    reader_buf_cap: usize,
    writer_buf_cap: usize,
    writers: Mutex<FxHashMap<u64, BufWriter<File>>>,
    name_ids: Mutex<FxHashMap<String, u64>>,
    next_id: AtomicU64,
}

impl FsAdapter {
    pub fn new(
        in_dir: PathBuf,
        out_dir: PathBuf,
        reader_buf_cap: usize,
        writer_buf_cap: usize,
    ) -> Result<Self> {
        fs::create_dir_all(&out_dir).with_context(|| format!("mkdir -p {:?}", out_dir))?;
        Ok(Self {
            in_dir,
            out_dir,
            reader_buf_cap,
            writer_buf_cap,
            writers: Mutex::new(FxHashMap::default()),
            name_ids: Mutex::new(FxHashMap::default()),
            next_id: AtomicU64::new(0),
        })
    }

    fn id_for(&self, name: &str) -> u64 {
        let mut map = self.name_ids.lock().unwrap();
        if let Some(&id) = map.get(name) {
            return id;
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        map.insert(name.to_string(), id);
        id
    }

    fn in_path(&self, name: &str) -> PathBuf {
        self.in_dir.join(name)
    }

    fn out_path(&self, name: &str) -> PathBuf {
        self.out_dir.join(name)
    }
}

impl StorageAdapter for FsAdapter {
    fn list_inputs(&self) -> Result<Vec<InputHandle>> {
        let mut out = Vec::new();
        if !self.in_dir.exists() {
            return Ok(out);
        }
        for entry in WalkDir::new(&self.in_dir).follow_links(false).into_iter() {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&self.in_dir)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .into_owned();
            if rel.starts_with('.') {
                continue;
            }
            let id = self.id_for(&rel);
            out.push(InputHandle { id, name: rel });
        }
        Ok(out)
    }

    fn open_reader(&self, h: &InputHandle) -> Result<Box<dyn Read + Send>> {
        let p = self.in_path(&h.name);
        let f = File::open(&p).with_context(|| format!("open input {:?}", p))?;
        Ok(Box::new(BufReader::with_capacity(self.reader_buf_cap, f)))
    }

    fn open_writer(&self, h: &InputHandle) -> Result<()> {
        let p = self.out_path(&h.name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).with_context(|| format!("mkdir -p {:?}", parent))?;
        }
        let f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&p)
            .with_context(|| format!("open output {:?}", p))?;
        let bw = BufWriter::with_capacity(self.writer_buf_cap, f);
        self.writers.lock().unwrap().insert(h.id, bw);
        Ok(())
    }

    fn append_chunk(&self, h: &InputHandle, chunk: &[u8]) -> Result<()> {
        let mut guard = self.writers.lock().unwrap();
        let w = guard
            .get_mut(&h.id)
            .with_context(|| format!("writer not open for {}", h.name))?;
        w.write_all(chunk).context("write chunk")
    }

    fn close_writer(&self, h: &InputHandle) -> Result<()> {
        if let Some(mut w) = self.writers.lock().unwrap().remove(&h.id) {
            w.flush().context("flush writer")?;
        }
        Ok(())
    }
}
