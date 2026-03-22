use crate::AbiString;

/// ABI-safe Minecraft-style identifier (`namespace:path`).
///
/// Used for plugin IDs, block/item registration, event names, etc.
/// Uses owned `AbiString` so it works both with static literals and
/// runtime-constructed values.
#[stabby::stabby]
pub struct Identifier {
    pub namespace: AbiString,
    pub path: AbiString,
}

impl Identifier {
    pub const VANILLA_NAMESPACE: &'static str = "minecraft";

    /// Valid namespace characters: `[a-z0-9._-]`
    pub const fn valid_namespace_char(c: char) -> bool {
        matches!(c, 'a'..='z' | '0'..='9' | '_' | '-' | '.')
    }

    /// Valid path characters: `[a-z0-9._-/]`
    pub const fn valid_path_char(c: char) -> bool {
        matches!(c, 'a'..='z' | '0'..='9' | '_' | '-' | '.' | '/')
    }

    /// Const validation — usable in `const { }` blocks for compile-time checks.
    pub const fn validate_namespace(s: &str) -> bool {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return false;
        }
        let mut i = 0;
        while i < bytes.len() {
            if !Self::valid_namespace_char(bytes[i] as char) {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Const validation — usable in `const { }` blocks for compile-time checks.
    pub const fn validate_path(s: &str) -> bool {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return false;
        }
        let mut i = 0;
        while i < bytes.len() {
            if !Self::valid_path_char(bytes[i] as char) {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Create an Identifier at runtime. Panics if namespace or path is invalid.
    pub fn new(namespace: &str, path: &str) -> Self {
        assert!(
            Self::validate_namespace(namespace),
            "invalid identifier namespace"
        );
        assert!(Self::validate_path(path), "invalid identifier path");
        Self {
            namespace: AbiString::from(namespace),
            path: AbiString::from(path),
        }
    }

    pub fn vanilla(path: &str) -> Self {
        Self::new(Self::VANILLA_NAMESPACE, path)
    }
}

impl core::fmt::Display for Identifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.namespace.as_str(), self.path.as_str())
    }
}

impl core::fmt::Debug for Identifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.namespace.as_str(), self.path.as_str())
    }
}
