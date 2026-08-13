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

#[cfg(test)]
mod tests {
    use super::is_safe_id;

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
}
