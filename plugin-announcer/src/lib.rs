use shared_events::PlayerGreetedEvent;
use steel_api::{EventResult, PluginContext, plugin_metadata, steel_handler, steel_plugin};

plugin_metadata! {
    id: "announcer",
    name: "Announcer",
    version: "0.1.0",
}

#[steel_handler]
fn on_player_greeted(event: &mut PlayerGreetedEvent, _cancelled: &mut bool) -> EventResult {
    println!(
        "[Announcer] {} was greeted! Spreading the word...",
        event.player_name.as_str()
    );
    EventResult::Continue
}

#[steel_plugin]
fn init(ctx: &PluginContext) {
    ctx.events()
        .register::<PlayerGreetedEvent>(on_player_greeted);
    println!("[Announcer] Initialized!");
}
