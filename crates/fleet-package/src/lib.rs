//! Stub for `fleet-package`. Real contents are added in the phase that owns this crate (see TODO.md).

/// Crate identifier used by the workspace smoke test.
pub const CRATE_NAME: &str = "fleet-package";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_set() {
        assert_eq!(CRATE_NAME, "fleet-package");
    }
}
