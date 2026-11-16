use super::*;

fn read_save(path: &str) -> io::Result<Vec<u8>> {
    const MAX_RETRIES: u32 = 3;

    for attempt in 0..=MAX_RETRIES {
        match std::fs::read(path) {
            Err(error) if is_transient_read_error(&error) && attempt < MAX_RETRIES => {
                std::thread::sleep(std::time::Duration::from_millis(10 << attempt));
            }
            result => return result,
        }
    }

    unreachable!("save read loop always returns after the final attempt")
}

fn is_transient_read_error(error: &io::Error) -> bool {
    let kind_is_transient = matches!(
        error.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    );
    #[cfg(unix)]
    let os_error_is_transient = error.raw_os_error() == Some(16);
    #[cfg(windows)]
    let os_error_is_transient = matches!(error.raw_os_error(), Some(32 | 33));
    #[cfg(not(any(unix, windows)))]
    let os_error_is_transient = false;

    kind_is_transient || os_error_is_transient
}

fn backup_timestamp() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn backup_path(path: &str, reason: &str) -> String {
    let base = format!("{path}.{reason}-{}", backup_timestamp());
    let mut candidate = base.clone();
    let mut suffix = 1;
    while std::path::Path::new(&candidate).exists() {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    candidate
}

/// Standalone IO so it can be called from a blocking task off the
/// main runtime. Atomic rename + parent-dir fsync mirror the
/// previous in-line behaviour.
pub fn write_save_to_disk(state: &SaveState, path: &str) -> io::Result<()> {
    let tmp_path = format!("{}.tmp", path);
    // Stream the JSON into the file instead of building the whole document
    // in memory first: a save is tens of MB, and the intermediate `String`
    // (grown by doubling) briefly cost about twice that again in RSS.
    let written = (|| -> io::Result<()> {
        let file = std::fs::File::create(&tmp_path)?;
        let mut out = io::BufWriter::with_capacity(1 << 20, file);
        serde_json::to_writer(&mut out, state).map_err(|e| {
            if e.is_io() {
                io::Error::from(e)
            } else {
                io::Error::new(io::ErrorKind::InvalidData, e)
            }
        })?;
        let file = out.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()
    })();
    if let Err(error) = written {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(error);
    }
    std::fs::rename(&tmp_path, path)?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        let dir = if parent.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            parent
        };
        if let Ok(d) = std::fs::File::open(dir) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

impl Simulation {
    /// A saved world, read without touching the file, or why it cannot be. Unlike [`Self::load_or_new`], an
    /// unreadable or newer save is an error rather than moved aside for a fresh world: for tools such as
    /// headless `--load`, where the file is an input and must stay where it is.
    pub fn load_strict(seed: u64, path: &str) -> Result<Self, String> {
        let data = read_save(path).map_err(|err| err.to_string())?;
        let loaded = Self::from_checkpoint_bytes(seed, &data)?;
        let version = serde_json::from_slice::<serde_json::Value>(&data)
            .ok()
            .and_then(|v| v.get("version").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        if version < u64::from(SAVE_SCHEMA_VERSION) {
            eprintln!("Migrated checkpoint schema {version} to {SAVE_SCHEMA_VERSION}: missing historical state was initialized; exact continuation of the old build is unavailable.");
        }
        Ok(loaded)
    }

    pub fn load_or_new(seed: u64, path: &str) -> Self {
        match read_save(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!("No save at {} - starting fresh world", path);
                Self::new(seed)
            }
            Err(e) => {
                let backup = backup_path(path, "unreadable");
                std::fs::rename(path, &backup).unwrap_or_else(|backup_error| {
                    panic!(
                        "Save at {} is unreadable ({}), and could not be preserved at {} ({}); refusing to start a fresh world",
                        path, e, backup, backup_error
                    )
                });
                tracing::warn!(
                    "Backed up unreadable save at {} to {} ({}) - starting fresh world",
                    path,
                    backup,
                    e
                );
                Self::new(seed)
            }
            Ok(data) => {
                match Self::from_checkpoint_bytes(seed, &data) {
                    Ok(sim) => sim,
                    Err(error) => {
                        let version = serde_json::from_slice::<serde_json::Value>(&data)
                            .ok()
                            .and_then(|v| v.get("version").and_then(|v| v.as_u64()));
                        let reason = match version {
                            Some(v) if v > u64::from(SAVE_SCHEMA_VERSION) => format!("future-v{v}"),
                            _ => "corrupt".to_owned(),
                        };
                        let backup = backup_path(path, &reason);
                        std::fs::rename(path, &backup).unwrap_or_else(|backup_error| {
                        panic!("Invalid checkpoint ({error}); could not preserve {path} at {backup}: {backup_error}; refusing to start a fresh world")
                    });
                        tracing::warn!("Checkpoint rejected ({error}); preserved at {backup}; recovery starts a fresh world");
                        Self::new(seed)
                    }
                }
            }
        }
    }
}
