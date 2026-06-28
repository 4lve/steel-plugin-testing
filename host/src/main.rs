use std::collections::HashMap;

#[stabby::import(name = "plugin_announcer")]
extern "C" {
    pub fn steel_plugin_new() -> stabby::opaque::RefMut<steel_api::Plugin>;
}

#[stabby::import_interface(opaque = steel_api::Plugin, prefix = "steel_plugin", name = "plugin_announcer")]
pub trait ImportedPluginApi {
    extern "C" fn name(&self) -> stabby::str::Str<'static>;
    extern "C" fn on_server_start(&mut self, host: steel_api::HostCoreRefMut, ticks: u64) -> u32;
    extern "C" fn on_player_join(
        &mut self,
        host: steel_api::HostCoreRefMut,
        player: stabby::str::Str<'_>,
    ) -> u32;
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

fn main() {
    let mut host = HostState::default();
    let mut plugin = steel_plugin_new();

    println!("loaded plugin: {}", plugin.name());

    let starts_seen = plugin.on_server_start(bind_host(&mut host), 120);
    println!("host saw server-start result: {starts_seen}");

    let player = String::from("Alex");
    let joins_seen = plugin.on_player_join(bind_host(&mut host), stabby::str::Str::new(&player));
    println!("host saw player-join result: {joins_seen}");

    println!("host counters after plugin callbacks: {:?}", host.counters);
    println!("host log lines: {:?}", host.log_lines);
}
