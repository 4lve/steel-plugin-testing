use libloading::Library;
use stabby::libloading::StabbyLibrary;
use steel_api::{API_VERSION, InitResult, PluginContext, PluginInitFn, PluginMetadataFn};

use crate::command::COMMAND_API;
use crate::event::EVENT_API;

pub struct LoadedPlugin {
    pub name: String,
    pub _lib: Library,
}

pub fn discover_plugins() -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir("./plugins") else {
        eprintln!("[Host] No plugins/ directory found.");
        return vec![];
    };
    let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "so" || ext == "dll" || ext == "dylib")
        })
        .map(|e| e.path())
        .collect();
    paths.sort();
    paths
}

pub fn load_plugin(path: &std::path::Path) -> Option<LoadedPlugin> {
    let filename = path.file_name().unwrap().to_string_lossy();
    println!("[Host] Loading {filename}...");

    // SAFETY: plugin is a cdylib built against our steel-api crate.
    let lib = match unsafe { Library::new(path) } {
        Ok(lib) => lib,
        Err(e) => {
            eprintln!("[Host] Failed to load {filename}: {e}");
            return None;
        }
    };

    // Load metadata with full ABI verification via stabby TypeReport.
    let metadata_fn =
        match unsafe { lib.get_stabbied::<PluginMetadataFn>(b"steel_plugin_metadata") } {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[Host] Failed to load steel_plugin_metadata from {filename}: {e}");
                return None;
            }
        };

    let metadata = (*metadata_fn)();
    let id = metadata.id.as_ref().to_string();
    let name = metadata.name.as_ref().to_string();
    let version = metadata.version.as_ref().to_string();
    println!("[Host] Found: {name} ({id}) v{version}");

    // Version check
    if metadata.api_version != API_VERSION {
        eprintln!(
            "[Host] {name} requires API v{}, but host has API v{API_VERSION}. Skipping.",
            metadata.api_version
        );
        return None;
    }

    // Load init function with full ABI verification.
    let init_fn = match unsafe { lib.get_stabbied::<PluginInitFn>(b"steel_plugin_init") } {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[Host] Failed to load steel_plugin_init from {name}: {e}");
            return None;
        }
    };

    // Phase 2: init (phase 1 — init_registers — is not yet implemented)
    let ctx = PluginContext::new(&EVENT_API, &COMMAND_API);
    match (*init_fn)(ctx) {
        InitResult::Ok => println!("[Host] {name} initialized."),
        InitResult::Panic => {
            eprintln!("[Host] {name} panicked during init!");
            return None;
        }
    }

    Some(LoadedPlugin {
        name: id,
        _lib: lib,
    })
}
