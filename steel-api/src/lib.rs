//! ABI-stable plugin API for Steel.
//!
//! Plugin developers depend on this crate to write plugins.
//! The host (server) uses this crate to define the ABI boundary.

pub use steel_api_macros::{steel_command, steel_handler, steel_plugin};

// Re-exports for plugin convenience and macro use.
pub use stabby::alloc::string::String as AbiString;
pub use stabby::str::Str as AbiStr;

mod command;
mod event;
mod identifier;
mod plugin;

pub use command::*;
pub use event::*;
pub use identifier::*;
pub use plugin::*;

/// Log a caught panic payload. Used by proc macros — not intended for direct use.
pub fn log_panic(context: &str, e: &(dyn std::any::Any + Send)) {
    let msg = if let Some(s) = e.downcast_ref::<&str>() {
        s
    } else if let Some(s) = e.downcast_ref::<String>() {
        s.as_str()
    } else {
        "unknown panic"
    };
    eprintln!("[Steel] {context} panicked: {msg}");
}
