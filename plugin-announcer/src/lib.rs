struct AnnouncerPlugin {
    starts_seen: u32,
    joins_seen: u32,
}

#[stabby::export]
pub extern "C" fn steel_plugin_new() -> stabby::opaque::RefMut<steel_api::Plugin> {
    let plugin = Box::leak(Box::new(AnnouncerPlugin {
        starts_seen: 0,
        joins_seen: 0,
    }));

    unsafe { stabby::opaque::RefMut::from_mut(plugin) }
}

#[stabby::export_interface(opaque = steel_api::Plugin, prefix = "steel_plugin")]
impl steel_api::PluginApi for AnnouncerPlugin {
    extern "C" fn name(&self) -> stabby::str::Str<'static> {
        stabby::str::Str::new("announcer")
    }

    extern "C" fn on_server_start(
        &mut self,
        mut host: stabby::opaque::InterfaceRefMut<steel_api::Host, steel_api::HostCoreVTable>,
        ticks: u64,
    ) -> u32 {
        use steel_api::{HostApi, HostCoreInterfaceResolver};

        self.starts_seen = self.starts_seen.saturating_add(1);
        let mut host = host
            .resolve_interface::<steel_api::HostApiVTable>()
            .unwrap();
        host.log(stabby::str::Str::new("announcer observed server start"));
        let host_starts = host.increment_counter(stabby::str::Str::new("server_starts"), 1);

        println!(
            "[announcer] server start event #{}, ticks={ticks}, host counter={host_starts}",
            self.starts_seen,
        );
        self.starts_seen
    }

    extern "C" fn on_player_join(
        &mut self,
        mut host: stabby::opaque::InterfaceRefMut<steel_api::Host, steel_api::HostCoreVTable>,
        player: stabby::str::Str<'_>,
    ) -> u32 {
        use steel_api::{HostApi, HostCoreInterfaceResolver};

        self.joins_seen = self.joins_seen.saturating_add(1);
        let mut host = host
            .resolve_interface::<steel_api::HostApiVTable>()
            .unwrap();
        host.log(stabby::str::Str::new("announcer observed player join"));
        let player_joins = host.increment_counter(player, 1);
        let total_joins = host.increment_counter(stabby::str::Str::new("player_joins"), 1);

        println!(
            "[announcer] player joined: {player} (plugin join #{}, player counter={player_joins}, total joins={total_joins})",
            self.joins_seen,
        );
        total_joins
    }
}
