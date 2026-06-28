#[stabby::opaque(module = "steel::plugin")]
pub struct Plugin;

#[stabby::opaque(module = "steel::host")]
pub struct Host;

#[stabby::interface(opaque = Host, prefix = "steel_host")]
pub trait HostApi {
    extern "C" fn log(&mut self, message: stabby::str::Str<'_>);
    extern "C" fn counter_len(&self) -> u64;
    extern "C" fn get_counter(&self, key: stabby::str::Str<'_>) -> stabby::option::Option<u32>;
    extern "C" fn insert_counter(
        &mut self,
        key: stabby::str::Str<'_>,
        value: u32,
    ) -> stabby::option::Option<u32>;
    extern "C" fn increment_counter(&mut self, key: stabby::str::Str<'_>, amount: u32) -> u32;
}

#[stabby::interface(opaque = Host, prefix = "steel_host_core", resolver)]
pub trait HostCore {
    extern "C" fn query_interface(
        &mut self,
        interface_id: u64,
        expected: &'static stabby::report::TypeReport,
    ) -> stabby::option::Option<stabby::opaque::ErasedInterfaceRefMut<Host>>;
}

pub trait PluginApi {
    extern "C" fn name(&self) -> stabby::str::Str<'static>;
    extern "C" fn on_server_start(&mut self, host: HostCoreRefMut, ticks: u64) -> u32;
    extern "C" fn on_player_join(
        &mut self,
        host: HostCoreRefMut,
        player: stabby::str::Str<'_>,
    ) -> u32;
}
