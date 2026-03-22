mod command;
mod event;
mod loader;

use steel_api::{EventResult, ServerStartingEvent};

fn main() {
    println!("=== Steel Server (prototype) ===\n");

    // Discover and load plugins
    let plugin_paths = loader::discover_plugins();
    if plugin_paths.is_empty() {
        println!("[Host] No plugins found.");
        return;
    }

    let mut plugins = Vec::new();
    for path in &plugin_paths {
        if let Some(plugin) = loader::load_plugin(path) {
            plugins.push(plugin);
        }
    }

    if plugins.is_empty() {
        println!("[Host] No plugins loaded successfully.");
        return;
    }

    println!(
        "\n[Host] {} plugin(s) loaded: {}\n",
        plugins.len(),
        plugins
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Fire server starting event
    let mut starting = ServerStartingEvent { block_count: 0 };
    let fire_result = event::fire_event(&mut starting);
    if fire_result.result == EventResult::Panic {
        eprintln!("[Host] Plugin panicked during ServerStarting event. Shutting down.");
        return;
    }
    if fire_result.cancelled {
        println!("[Host] ServerStarting event was cancelled.");
    }

    // Simulate command execution
    println!("--- Simulating commands ---\n");

    println!("[Host] Steve runs: /hello");
    command::execute_command("hello", 1);
    println!();

    println!("[Host] Console runs: /hello");
    command::execute_command("hello", 0);
    println!();

    println!("[Host] Steve runs: /nonexistent");
    command::execute_command("nonexistent", 1);

    println!("\n=== Done ===");
}
