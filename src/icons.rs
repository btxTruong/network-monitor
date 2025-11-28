//! Flag icons module - provides embedded country flag lookup
//! Flags are generated at build time from circle-flags SVGs.

// Include auto-generated flag data
include!(concat!(env!("OUT_DIR"), "/icons_data.rs"));

/// Icon dimensions for tray display
pub const ICON_SIZE: u32 = 64;

/// Represents a flag icon with PNG data
pub struct FlagIcon {
    /// Raw PNG bytes
    pub data: &'static [u8],
}

/// Gets the flag icon for a given ISO 3166-1 alpha-2 country code.
/// Returns a fallback globe icon if the country code is not found.
pub fn get_flag(country_code: &str) -> FlagIcon {
    let code = country_code.to_lowercase();

    let data = FLAGS
        .get(code.as_str())
        .copied()
        .unwrap_or_else(|| {
            // Fallback: try "xx" (unknown) or use first available flag
            FLAGS.get("xx").copied().unwrap_or_else(|| {
                FLAGS.values().next().copied().unwrap_or(&[])
            })
        });

    FlagIcon { data }
}

/// Checks if a flag exists for the given country code
#[allow(dead_code)]
pub fn has_flag(country_code: &str) -> bool {
    FLAGS.contains_key(country_code.to_lowercase().as_str())
}

/// Returns the number of available flag icons
pub fn flag_count() -> usize {
    FLAGS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_flag_existing() {
        let flag = get_flag("us");
        assert!(!flag.data.is_empty());
    }

    #[test]
    fn test_get_flag_uppercase() {
        let flag = get_flag("US");
        assert!(!flag.data.is_empty());
    }

    #[test]
    fn test_get_flag_nonexistent() {
        // Should return fallback, not panic
        let flag = get_flag("zz");
        assert!(!flag.data.is_empty());
    }

    #[test]
    fn test_flag_count() {
        assert!(flag_count() > 100, "Expected more than 100 flags");
    }
}
