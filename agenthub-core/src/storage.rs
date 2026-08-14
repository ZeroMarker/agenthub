use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Counter for unique temporary file names (per-process).
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write `content` to `path` atomically: the data is written to a unique
/// temporary file in the same directory and renamed over the target, so a
/// concurrent or crashing writer never leaves a torn/partial file behind.
pub(crate) fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "out".to_string());
    let tmp = dir.join(format!(
        ".{name}.tmp-{}-{}",
        std::process::id(),
        TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&tmp, content)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Best-effort cleanup of the temporary file.
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Returns whether a user-controlled identifier is safe to use as one path
/// component on every supported platform.
pub(crate) fn is_safe_id(id: &str) -> bool {
    !id.trim().is_empty()
        && id != "."
        && id != ".."
        && id.len() <= 255
        && !id.contains(['/', '\\'])
        && !id.chars().any(char::is_control)
}

/// Returns whether a path from imported data is a relative path made only of
/// normal components. This rejects absolute paths and `.`/`..` traversal.
pub(crate) fn is_safe_relative_path(path: &std::path::Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::{is_safe_id, is_safe_relative_path};

    #[test]
    fn accepts_normal_and_unicode_ids() {
        assert!(is_safe_id("codex-default"));
        assert!(is_safe_id("review.v2"));
        assert!(is_safe_id("代码审查"));
    }

    #[test]
    fn rejects_path_traversal_and_unsafe_ids() {
        for id in ["", "  ", ".", "..", "../escape", "a/b", "a\\b", "bad\nname"] {
            assert!(!is_safe_id(id), "{id:?} should be rejected");
        }
    }

    #[test]
    fn validates_imported_relative_paths() {
        assert!(is_safe_relative_path(std::path::Path::new(
            "projects/demo/note.md"
        )));
        for path in ["", "../escape.md", "/tmp/escape.md", "a/../../escape.md"] {
            assert!(!is_safe_relative_path(std::path::Path::new(path)));
        }
    }
}
