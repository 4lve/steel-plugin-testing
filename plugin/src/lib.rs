use steel_api::{
    CommandContext, EventResult, PluginContext, ServerStartingEvent, plugin_metadata,
    steel_command, steel_handler, steel_plugin,
};

plugin_metadata! {
    id: "hello_world",
    name: "Hello World",
    version: "0.1.0",
}

#[steel_command]
fn hello_command(ctx: &CommandContext) {
    let name = ctx.sender_name();
    ctx.reply(format!("Hello, {}! Welcome to Steel.", name.as_ref()));
}

#[steel_handler]
fn on_server_starting(event: &mut ServerStartingEvent) -> EventResult {
    println!(
        "[HelloWorld] Server is starting with {} blocks!",
        event.block_count
    );
    EventResult::Continue
}

#[steel_plugin]
fn init(ctx: &PluginContext) {
    ctx.events()
        .register::<ServerStartingEvent>(on_server_starting);
    ctx.commands().register("hello", hello_command);
    println!("[HelloWorld] Initialized!");
}
