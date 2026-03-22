use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use steel_api::{
    AbiString, CommandApiVtable, CommandContext, CommandContextVtable, CommandHandler,
    CommandResult,
};

// ── Registry ─────────────────────────────────────────────────────────────────

static COMMANDS: LazyLock<Mutex<HashMap<String, CommandHandler>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

extern "C" fn host_register_command(name: AbiString, handler: CommandHandler) {
    let name_str = name.as_str().to_owned();
    println!("[Host] Registered command: /{name_str}");
    COMMANDS.lock().unwrap().insert(name_str, handler);
}

pub(crate) static COMMAND_API: CommandApiVtable = CommandApiVtable {
    register: host_register_command,
};

// ── Context Vtable ───────────────────────────────────────────────────────────

extern "C" fn cmd_reply(_handle: u64, message: AbiString) {
    println!("[Chat] {}", message.as_str());
}

extern "C" fn cmd_sender_name(handle: u64) -> AbiString {
    match handle {
        0 => AbiString::from("Console"),
        1 => AbiString::from("Steve"),
        _ => AbiString::from("Unknown"),
    }
}

static CMD_CTX_VTABLE: CommandContextVtable = CommandContextVtable {
    reply: cmd_reply,
    sender_name: cmd_sender_name,
};

// ── Execution ────────────────────────────────────────────────────────────────

pub fn execute_command(name: &str, sender_handle: u64) {
    let lock = COMMANDS.lock().unwrap();

    if let Some(handler) = lock.get(name) {
        let ctx = CommandContext::new(sender_handle, &CMD_CTX_VTABLE);
        match handler(ctx) {
            CommandResult::Ok => {}
            CommandResult::Panic => {
                eprintln!("[Host] Command /{name} panicked!");
            }
        }
    } else {
        println!("[Host] Unknown command: /{name}");
    }
}
