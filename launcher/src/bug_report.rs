//! A bug report as a local file: what the player wrote, this machine, and optionally the end of
//! the game's log, with the player's worlds hidden. Nothing is sent: the player reads the file
//! and attaches it by hand.
use crate::backend;
use horizon_platform::system::Machine;
use std::path::PathBuf;

/// The newest log lines kept in a report.
const LOG_LINES: usize = 300;

/// The player's worlds' folder names, to hide from the report.
fn world_terms() -> Vec<String> {
    std::fs::read_dir(backend::game_home().join("saves"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .collect()
}

/// Case-insensitive removal of `terms` from `text`; each hit becomes `[hidden]`.
fn scrub(text: &str, terms: &[String]) -> String {
    let mut out = text.to_string();
    for term in terms {
        if !term.is_empty() {
            out = out.replace(term, "[hidden]");
        }
    }
    out
}

/// Writes `bug-reports/bug-report-<time>.txt` in the game folder and returns its path.
pub fn write(text: &str, with_log: bool) -> Result<PathBuf, String> {
    let home = backend::game_home();
    let terms = world_terms();
    let scrubber = |s: &str| scrub(s, &terms);
    let machine = Machine::detect_in(Some(&home));
    let mut report = format!(
        "{}\n\nHorizon launcher {}\n{} {} on {}, {} ({} cores, {} threads, {} GB)\n",
        scrubber(text.trim()),
        backend::VERSION,
        machine.os,
        machine.os_version,
        machine.arch,
        machine.cpu,
        machine.cores,
        machine.threads,
        machine.ram_gb,
    );
    if with_log {
        let (_, log) = backend::read_logs(LOG_LINES, false)?;
        let kept: Vec<&str> = log.lines().filter(|l| !l.contains("[CHAT]")).collect();
        report.push_str("\n--- end of the game's log ---\n");
        report.push_str(&scrubber(&kept.join("\n")));
        report.push('\n');
    }
    let dir = home.join("bug-reports");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!(
        "bug-report-{}.txt",
        chrono::Local::now().format("%Y-%m-%d_%H.%M.%S")
    ));
    std::fs::write(&path, report).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}
