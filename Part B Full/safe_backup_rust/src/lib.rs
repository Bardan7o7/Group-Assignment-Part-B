//! Core library for safe_backup.
//! Secure file operations: backup, restore, delete, with validation & simple logging.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Validate a filename: not empty, not absolute, no parent traversal.
pub fn validate_path(name: &str) -> io::Result<PathBuf> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty file name"));
    }
    let p = Path::new(trimmed);
    if p.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "absolute paths not allowed"));
    }
    let s = trimmed.replace('\\', "/");
    if s.starts_with("../") || s.contains("/../") || s.starts_with("./../") {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "parent traversal not allowed"));
    }
    let mut cwd = std::env::current_dir()?;
    cwd.push(trimmed);
    Ok(cwd)
}

/// Build timestamped "<name>.<ts>.bak" in CWD.
fn ts_backup_for(original_name: &str, ts: u64) -> io::Result<PathBuf> {
    let base = Path::new(original_name)
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid file name"))?
        .to_string_lossy()
        .to_string();
    let mut cwd = std::env::current_dir()?;
    cwd.push(format!("{base}.{ts}.bak"));
    Ok(cwd)
}

/// Build convenience "name.bak" (just the stem + .bak) in CWD.
fn plain_backup_for(original_name: &str) -> io::Result<PathBuf> {
    let base = Path::new(original_name)
        .file_stem()
        .unwrap_or_else(|| Path::new(original_name).as_os_str())
        .to_string_lossy()
        .to_string();
    let mut cwd = std::env::current_dir()?;
    cwd.push(format!("{base}.bak"));
    Ok(cwd)
}

/// Find latest "<base>.<ts>.bak" for original; fall back to "name.bak".
pub fn find_latest_backup(original_name: &str) -> io::Result<PathBuf> {
    let mut newest: Option<(u64, PathBuf)> = None;
    let base = Path::new(original_name)
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid file name"))?
        .to_string_lossy()
        .to_string();

    for entry in fs::read_dir(std::env::current_dir()?)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() { continue; }
        let Some(fname) = path.file_name().and_then(|s| s.to_str()) else { continue };
        if fname.starts_with(&(base.clone() + ".")) and fname.ends_with(".bak") {
            if let Some(ts) = fname.trim_end_matches(".bak").rsplit('.').next().and_then(|n| n.parse::<u64>().ok()) {
                if newest.as_ref().map(|(t, _)| ts > *t).unwrap_or(true) {
                    newest = Some((ts, path.clone()));
                }
            }
        }
    }

    if let Some((_, p)) = newest { return Ok(p); }

    let plain = plain_backup_for(original_name)?;
    if plain.exists() { return Ok(plain); }

    Err(io::Error::new(io::ErrorKind::NotFound, "no backup file found"))
}

/// Backup: copies <name> to timestamped and also updates plain "<stem>.bak".
pub fn backup_file(name: &str) -> io::Result<PathBuf> {
    let src = validate_path(name)?;
    if !src.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "source file does not exist"));
    }
    let ts = now_unix();
    let ts_bak = ts_backup_for(name, ts)?;
    fs::copy(&src, &ts_bak)?;
    let plain_bak = plain_backup_for(name)?;
    fs::copy(&src, &plain_bak)?;
    log_action("backup", name, "ok")?;
    Ok(ts_bak)
}

/// Restore:
/// - If `name` ends with ".bak": restore from that file to a sensible target.
/// - If `name` is original (e.g., "test.txt"): restore from latest backup to "name".
pub fn restore_file(name: &str) -> io::Result<PathBuf> {
    let trimmed = name.trim();
    let cwd = std::env::current_dir()?;
    let dest: PathBuf;
    let src_bak: PathBuf;

    if trimmed.ends_with(".bak") {
        src_bak = validate_path(trimmed)?;
        if !src_bak.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "backup file not found"));
        }
        let fname = Path::new(trimmed).file_name().and_then(|s| s.to_str()).unwrap_or(trimmed);
        let maybe_ts = fname.trim_end_matches(".bak").rsplit('.').next();
        let ts_is_num = maybe_ts.and_then(|n| n.parse::<u64>().ok()).is_some();

        if ts_is_num {
            // "<orig>.<ts>.bak" → restore to "<orig>"
            let logical = fname.trim_end_matches(".bak").rsplitn(2, '.').last().unwrap_or("restored.out");
            dest = cwd.join(logical);
        } else {
            // "<stem>.bak" → restore to "<stem>.restored.<now>"
            let stem = Path::new(fname).file_stem().and_then(|s| s.to_str()).unwrap_or("restored");
            dest = cwd.join(format!("{stem}.restored.{}", now_unix()));
        }
    } else {
        // Original name passed → pick latest backup automatically
        src_bak = find_latest_backup(trimmed)?;
        dest = cwd.join(Path::new(trimmed).file_name().unwrap());
    }

    fs::copy(&src_bak, &dest)?;
    log_action("restore", name, "ok")?;
    Ok(dest)
}

/// Delete a given file (validated).
pub fn delete_file(name: &str) -> io::Result<()> {
    let p = validate_path(name)?;
    if p.exists() {
        fs::remove_file(p)?;
        log_action("delete", name, "ok")?;
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::NotFound, "file does not exist"))
    }
}

/// Minimal JSONL logger in ./logfile.txt
fn log_action(action: &str, file: &str, result: &str) -> io::Result<()> {
    let mut path = std::env::current_dir()?;
    path.push("logfile.txt");
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    let user = whoami::username();
    let ts = now_unix();
    writeln!(
        f,
        "{{\"ts\":{ts},\"user\":\"{user}\",\"action\":\"{action}\",\"file\":\"{file}\",\"result\":\"{result}\"}}"
    )?;
    Ok(())
}
