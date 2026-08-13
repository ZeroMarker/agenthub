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
