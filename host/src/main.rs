use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

use stabby::libloading::StabbyLibrary;
use steel_api::PluginApi;

struct LoadedPlugin {
    plugin: stabby::opaque::RefMut<steel_api::Plugin>,
    name: steel_api::PluginName,
    on_server_start: steel_api::PluginOnServerStart,
    on_player_join: steel_api::PluginOnPlayerJoin,
    _library: libloading::Library,
}

impl LoadedPlugin {
    fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // SAFETY: Loading a dynamic library and resolving foreign symbols is inherently
        // unsafe. Each symbol is checked by stabby before its function pointer is used,
        // and the library is kept alive for at least as long as those pointers.
        unsafe {
            let library = libloading::Library::new(path)?;
            let new = *library.get_stabbied::<steel_api::PluginNew>(b"steel_plugin_new")?;
            let name = *library.get_stabbied::<steel_api::PluginName>(b"steel_plugin_name")?;
            let on_server_start = *library
                .get_stabbied::<steel_api::PluginOnServerStart>(b"steel_plugin_on_server_start")?;
            let on_player_join = *library
                .get_stabbied::<steel_api::PluginOnPlayerJoin>(b"steel_plugin_on_player_join")?;
            let plugin = new();

            Ok(Self {
                plugin,
                name,
                on_server_start,
                on_player_join,
                _library: library,
            })
        }
    }
}

impl steel_api::PluginApi for LoadedPlugin {
    extern "C" fn name(&self) -> stabby::str::Str<'static> {
        (self.name)(self.plugin.as_ref())
    }

    extern "C" fn on_server_start(&mut self, host: steel_api::HostCoreRefMut, ticks: u64) -> u32 {
        (self.on_server_start)(self.plugin.reborrow(), host, ticks)
    }

    extern "C" fn on_player_join(
        &mut self,
        host: steel_api::HostCoreRefMut,
        player: stabby::str::Str<'_>,
    ) -> u32 {
        // The generated import/export interface ABI erases argument lifetimes to
        // 'static. The plugin call is synchronous, so this borrowed string cannot
        // escape through the ABI-stable Str handle.
        let player = unsafe { core::mem::transmute::<_, stabby::str::Str<'static>>(player) };
        (self.on_player_join)(self.plugin.reborrow(), host, player)
    }
}

#[derive(Default)]
struct HostState {
    counters: HashMap<String, u32>,
    log_lines: Vec<String>,
}

#[stabby::export_interface(
    opaque = steel_api::Host,
    prefix = "steel_host",
    vtable = steel_api::HostApiVTable
)]
impl steel_api::HostApi for HostState {
    extern "C" fn log(&mut self, message: stabby::str::Str<'_>) {
        let message = message.as_str().to_owned();
        println!("[host api] {message}");
        self.log_lines.push(message);
    }

    extern "C" fn counter_len(&self) -> u64 {
        self.counters.len() as u64
    }

    extern "C" fn get_counter(&self, key: stabby::str::Str<'_>) -> stabby::option::Option<u32> {
        self.counters.get(key.as_str()).copied().into()
    }

    extern "C" fn insert_counter(
        &mut self,
        key: stabby::str::Str<'_>,
        value: u32,
    ) -> stabby::option::Option<u32> {
        self.counters.insert(key.as_str().to_owned(), value).into()
    }

    extern "C" fn increment_counter(&mut self, key: stabby::str::Str<'_>, amount: u32) -> u32 {
        let count = self.counters.entry(key.as_str().to_owned()).or_insert(0);
        *count = count.saturating_add(amount);
        *count
    }
}

#[stabby::export_interface(
    opaque = steel_api::Host,
    prefix = "steel_host_core",
    vtable = steel_api::HostCoreVTable
)]
impl steel_api::HostCore for HostState {
    extern "C" fn query_interface(
        &mut self,
        interface_id: u64,
        expected: &'static stabby::report::TypeReport,
    ) -> stabby::option::Option<stabby::opaque::ErasedInterfaceRefMut<steel_api::Host>> {
        let mut this = unsafe { stabby::opaque::RefMut::<steel_api::Host>::from_mut(self) };
        steel_host_interface_query(&mut this, interface_id, expected)
    }
}

fn bind_host(host: &mut HostState) -> steel_api::HostCoreRefMut {
    let host = unsafe { stabby::opaque::RefMut::<steel_api::Host>::from_mut(host) };
    steel_host_core_interface_bind(host)
}

fn plugin_path_from_args() -> PathBuf {
    let mut args = std::env::args_os();
    let program = args.next().unwrap_or_else(|| OsString::from("host"));
    match (args.next(), args.next()) {
        (Some(path), None) => PathBuf::from(path),
        _ => {
            eprintln!(
                "usage: {} <path-to-plugin-dynamic-library>",
                Path::new(&program)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("host")
            );
            eprintln!(
                "example: cargo run -p host -- target/debug/deps/{}",
                plugin_library_hint()
            );
            std::process::exit(2);
        }
    }
}

fn plugin_library_hint() -> &'static str {
    if cfg!(target_os = "windows") {
        "your_plugin.dll"
    } else if cfg!(target_os = "macos") {
        "libyour_plugin.dylib"
    } else {
        "libyour_plugin.so"
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let plugin_path = plugin_path_from_args();
    let mut host = HostState::default();
    let mut plugin = LoadedPlugin::load(&plugin_path)?;

    println!("loaded plugin: {}", plugin.name());

    let starts_seen = plugin.on_server_start(bind_host(&mut host), 120);
    println!("host saw server-start result: {starts_seen}");

    let player = String::from("Alex");
    let joins_seen = plugin.on_player_join(bind_host(&mut host), stabby::str::Str::new(&player));
    println!("host saw player-join result: {joins_seen}");

    println!("host counters after plugin callbacks: {:?}", host.counters);
    println!("host log lines: {:?}", host.log_lines);

    Ok(())
}
