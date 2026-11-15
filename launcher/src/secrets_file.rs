//! The one secret the launcher still keeps for the player: the Tailscale API token
//! used by the Servers page's friends feature (`tailscale::TOKEN_KEY`).
//!
//! The account system used to hold this in the OS keychain. The keychain is gone
//! from this fork (no accounts), and a short-lived API token does not need a
//! secret service: this file keeps it in the launcher's data folder, readable and
//! writable by the player only (0600 on Unix; the default ACLs on Windows), with
//! values base64-encoded. Base64 is NOT encryption: it keeps a token from turning
//! up in a log line or a paste by accident, not from someone who can read the
//! disk. That is the same trade the old secrets file offered before the account
//! system, and it is enough for a token the player can revoke in Tailscale's
//! admin console at any time.
//!
//! One JSON object, key -> base64 value, replaced whole on every write, under a
//! lock file beside it so two launcher processes never write half a file.

use std::path::{Path, PathBuf};

/// The secrets file's name in the launcher's data folder.
const SECRETS_FILE: &str = "secrets.json";

fn file() -> PathBuf {
    crate::backend::data_dir().join(SECRETS_FILE)
}

fn lock_file() -> PathBuf {
    file().with_extension("lock")
}

fn shown(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// Runs `f` holding the lock file, so each read-modify-write is whole across processes.
fn locked<T>(f: impl FnOnce(&PathBuf) -> Result<T, String>) -> Result<T, String> {
    let path = file();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| shown(dir, e))?;
    }
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(lock_file())
        .map_err(|e| shown(&lock_file(), e))?;
    lock.lock().map_err(|e| shown(&lock_file(), e))?;
    // Closing the file releases the lock, after `f` whatever it answers.
    f(&path)
}

fn read(path: &Path) -> Result<std::collections::BTreeMap<String, String>, String> {
    use base64::Engine;
    match std::fs::read(path) {
        Ok(bytes) => {
            let entries: std::collections::BTreeMap<String, String> =
                serde_json::from_slice(&bytes).map_err(|_| {
                    format!(
                        "{} can't be read. Remove it and save the token again.",
                        path.display()
                    )
                })?;
            entries
                .into_iter()
                .map(|(key, value)| {
                    base64::engine::general_purpose::STANDARD
                        .decode(value)
                        .ok()
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .map(|value| (key, value))
                        .ok_or_else(|| {
                            format!(
                                "{} can't be read. Remove it and save the token again.",
                                path.display()
                            )
                        })
                })
                .collect()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
        Err(e) => Err(shown(path, e)),
    }
}

/// Replaces the file through a temporary file of this write's own, so a crash
/// leaves the old file or the new one. Removed when empty.
fn write(path: &Path, entries: &std::collections::BTreeMap<String, String>) -> Result<(), String> {
    use base64::Engine;
    if entries.is_empty() {
        return match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(shown(path, e)),
            _ => Ok(()),
        };
    }
    let encoded: std::collections::BTreeMap<String, String> = entries
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                base64::engine::general_purpose::STANDARD.encode(value),
            )
        })
        .collect();
    let json = serde_json::to_vec(&encoded).map_err(|e| e.to_string())?;
    let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let written = options.open(&tmp).and_then(|mut file| {
        std::io::Write::write_all(&mut file, &json)?;
        file.sync_all()
    });
    if let Err(e) = written.and_then(|()| std::fs::rename(&tmp, path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(shown(path, e));
    }
    Ok(())
}

/// The secret for `key`, or `None` when there is none.
pub fn get(key: &str) -> Result<Option<String>, String> {
    locked(|path| read(path).map(|entries| entries.into_iter().find(|(k, _)| k == key).map(|(_, v)| v)))
}

/// Keeps `secret` for `key`.
pub fn set(key: &str, secret: &str) -> Result<(), String> {
    locked(|path| {
        let mut entries = read(path)?;
        entries.insert(key.into(), secret.into());
        write(path, &entries)
    })
}

/// Forgets `key`, if it is there.
pub fn delete(key: &str) -> Result<(), String> {
    locked(|path| {
        let mut entries = read(path)?;
        entries.remove(key);
        write(path, &entries)
    })
}
