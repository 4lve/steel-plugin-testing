use std::sync::OnceLock;

use shared_events::PlayerGreetedEvent;
use steel_api::{
    AbiString, CommandContext, EventApiVtable, EventResult, PluginContext, ServerStartingEvent,
    plugin_metadata, steel_command, steel_handler, steel_plugin,
};

plugin_metadata! {
    id: "hello_world",
    name: "Hello World",
    version: "0.2.0",
}

static EVENT_VTABLE: OnceLock<&'static EventApiVtable> = OnceLock::new();

#[steel_command]
fn hello_command(ctx: &CommandContext) {
    let name = ctx.sender_name();
    ctx.reply(format!("Hello, {}! Welcome to Steel.", name.as_ref()));

    // Fire a custom event so other plugins can react
    if let Some(vtable) = EVENT_VTABLE.get() {
        let events = steel_api::EventApi::new(vtable, "hello_world");
        let mut event = PlayerGreetedEvent {
            player_name: AbiString::from(name.as_ref()),
        };
        events.emit(&mut event);
    }
}

#[steel_handler]
fn on_server_starting(event: &mut ServerStartingEvent, _cancelled: &mut bool) -> EventResult {
    println!(
        "[HelloWorld] Server is starting with {} blocks!",
        event.block_count
    );
    EventResult::Continue
}

#[steel_plugin]
fn init(ctx: &PluginContext) {
    EVENT_VTABLE.set(ctx.event_vtable()).ok();

    ctx.events()
        .on::<ServerStartingEvent>(on_server_starting)
        .before("some_other_plugin")
        .register();

    ctx.commands().register("hello", hello_command);
    println!("[HelloWorld] Initialized!");
}
