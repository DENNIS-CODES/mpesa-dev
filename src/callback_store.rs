use std::path::{Path, PathBuf};

use chrono::Utc;
use rand::Rng;

use crate::daraja::models::StkCallbackEnvelope;
use crate::daraja::result_code;
use crate::error::{Error, Result};

/// Where `inspect` persists callbacks and `replay` reads them back from.
/// Gitignored — this is local scratch data, never meant to be committed.
fn dir() -> PathBuf {
    PathBuf::from("callbacks")
}

/// Saves a callback's parsed JSON to disk so `replay` can resend it later.
/// Filenames sort chronologically (UTC, so this holds even across DST
/// transitions) and carry the ResultCode (when the payload is a
/// recognized STK callback) so a directory listing alone is informative,
/// e.g. `20260728-153245_rc0_a1b2c3d4.json`.
pub fn save(value: &serde_json::Value) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir())?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let tag = serde_json::from_value::<StkCallbackEnvelope>(value.clone())
        .map(|envelope| format!("rc{}", envelope.body.stk_callback.result_code))
        .unwrap_or_else(|_| "raw".to_string());
    let short_id: String = {
        const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..8)
            .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
            .collect()
    };

    let path = dir().join(format!("{timestamp}_{tag}_{short_id}.json"));
    std::fs::write(
        &path,
        serde_json::to_string_pretty(value).unwrap_or_default(),
    )?;
    Ok(path)
}

/// All stored callbacks, newest first. Empty (not an error) if the
/// directory doesn't exist yet — nothing has been captured.
pub fn list() -> std::io::Result<Vec<PathBuf>> {
    let dir = dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();
    entries.reverse();
    Ok(entries)
}

/// Resolves a user-supplied selector — a 1-based index into [`list`], a
/// bare filename, or a path — to an actual stored callback file.
pub fn resolve(selector: &str) -> Result<PathBuf> {
    if let Ok(index) = selector.parse::<usize>() {
        if index == 0 {
            return Err(Error::Config(
                "stored callbacks are 1-indexed; try `mpesa-dev replay 1`".to_string(),
            ));
        }
        let files = list()?;
        return files.get(index - 1).cloned().ok_or_else(|| {
            Error::Config(format!(
                "no stored callback #{index}; run `mpesa-dev replay` with no arguments to see what's available"
            ))
        });
    }

    let direct = PathBuf::from(selector);
    if direct.is_file() {
        return Ok(direct);
    }

    let mut candidate = dir().join(selector);
    if candidate.is_file() {
        return Ok(candidate);
    }
    if candidate.extension().is_none() {
        candidate.set_extension("json");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(Error::Config(format!(
        "no stored callback matching '{selector}'; run `mpesa-dev replay` with no arguments to see what's available"
    )))
}

/// A short, human-readable description of a stored callback for listing —
/// its ResultCode and plain-English meaning, or a note that it isn't a
/// recognized STK callback shape.
pub fn summarize(path: &Path) -> String {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return "(unreadable)".to_string();
    };
    let Ok(envelope) = serde_json::from_str::<StkCallbackEnvelope>(&contents) else {
        return "(not a recognized STK callback shape)".to_string();
    };
    let callback = envelope.body.stk_callback;
    let (_, description) = result_code::describe(callback.result_code);
    format!("ResultCode {}: {description}", callback.result_code)
}
