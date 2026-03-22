use crate::{API_VERSION, AbiStr, CommandApi, CommandApiVtable, EventApi, EventApiVtable};

#[stabby::stabby]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitResult {
    Ok,
    /// The init function panicked. Host should shut down gracefully.
    Panic,
}

// ── Plugin Context ───────────────────────────────────────────────────────────

/// Context passed to the plugin's init function by value.
/// Provides access to scoped API objects for registration.
#[stabby::stabby]
pub struct PluginContext {
    pub api_version: u32,
    event_api: &'static EventApiVtable,
    command_api: &'static CommandApiVtable,
}

impl PluginContext {
    pub fn new(event_api: &'static EventApiVtable, command_api: &'static CommandApiVtable) -> Self {
        Self {
            api_version: API_VERSION,
            event_api,
            command_api,
        }
    }

    /// Access the event registration API.
    pub fn events(&self) -> EventApi<'_> {
        EventApi::new(self.event_api)
    }

    /// Access the command registration API.
    pub fn commands(&self) -> CommandApi<'_> {
        CommandApi::new(self.command_api)
    }
}

// ── Plugin Metadata ──────────────────────────────────────────────────────────

/// Metadata about a plugin, returned by the exported `steel_plugin_metadata` symbol.
///
/// `id` is a short machine-friendly identifier (e.g. `"ae2"`), used for
/// dependency references, load ordering, and file paths.
/// `name` is the human-readable display name (e.g. `"Applied Energistics 2"`).
///
/// Uses borrowed `Str<'static>` since metadata values are always string literals.
#[stabby::stabby]
pub struct PluginMetadata {
    pub api_version: u32,
    pub id: AbiStr<'static>,
    pub name: AbiStr<'static>,
    pub version: AbiStr<'static>,
}

pub type PluginInitFn = extern "C" fn(ctx: PluginContext) -> InitResult;
pub type PluginMetadataFn = extern "C" fn() -> PluginMetadata;

/// Declare plugin metadata. Generates the exported `steel_plugin_metadata` symbol.
///
/// ```ignore
/// steel_api::plugin_metadata! {
///     id: "my-plugin",
///     name: "My Plugin",
///     version: "0.1.0",
/// }
/// ```
#[macro_export]
macro_rules! plugin_metadata {
    (id: $id:literal, name: $name:literal, version: $version:literal $(,)?) => {
        const _: () = {
            assert!(
                $crate::Identifier::validate_namespace($id),
                concat!("invalid plugin id: \"", $id, "\" — must match [a-z0-9._-]"),
            );
        };

        #[stabby::export]
        pub extern "C" fn steel_plugin_metadata() -> $crate::PluginMetadata {
            $crate::PluginMetadata {
                api_version: $crate::API_VERSION,
                id: $crate::AbiStr::new($id),
                name: $crate::AbiStr::new($name),
                version: $crate::AbiStr::new($version),
            }
        }
    };
}
