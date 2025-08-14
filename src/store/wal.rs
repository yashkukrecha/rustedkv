use crate::util::{LogEntry, Operation};
use std::{fs::{File, OpenOptions}, io::{BufReader, BufRead, BufWriter, Write, self}, path::{Path, PathBuf}};

pub struct Wal {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl Wal {
    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let create_new = !path.exists();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true) 
            .open(&path)?;

        // If file was just created, fsync the directory so the entry itself is durable.
        if create_new {
            if let Some(parent) = path.parent() {
                fsync_dir(parent)?;
            }
        }

        let writer = BufWriter::new(file);
        Ok(Self { path, writer })
    }

    pub fn append(&mut self, entry: &LogEntry) -> anyhow::Result<()> {
        let line = serde_json::to_string(entry)?;
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.writer.flush();
        self.writer.get_mut().sync_all()
    }

    pub fn append_sync(&mut self, entry: &LogEntry) -> anyhow::Result<()> {
        self.append(entry)?;
        self.sync()?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub fn replay_wal<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<LogEntry>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (idx, line_res) in reader.lines().enumerate() {
        let line = match line_res {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading WAL at line {}: {}", idx + 1, e);
                break; 
            }
        }

        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<LogEntry>(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                eprintln!("Error parsing WAL entry at line {}: {}. Stopping replay.", idx + 1, e);
                break; 
            }
        }
    }

    Ok(entries)
}

// Make files durable
fn fsync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let dir_file = OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY).open(dir)?;
        dir_file.sync_all()
    }
    #[cfg(not(target_family = "unix"))]
    {
        Ok(())
    }
}