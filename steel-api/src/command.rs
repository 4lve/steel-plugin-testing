use crate::AbiString;

#[stabby::stabby]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    Ok,
    /// The command handler panicked. Host should log the error.
    Panic,
}

// ── Command Context ──────────────────────────────────────────────────────────

/// Vtable for command context operations. Set by the host.
#[stabby::stabby]
pub struct CommandContextVtable {
    pub reply: extern "C" fn(handle: u64, message: AbiString),
    pub sender_name: extern "C" fn(handle: u64) -> AbiString,
}

/// Context passed to command handlers by value.
/// Provides `reply()` and sender info.
#[stabby::stabby]
pub struct CommandContext {
    handle: u64,
    vtable: &'static CommandContextVtable,
}

impl CommandContext {
    /// Create a new context. Only the host should call this.
    pub fn new(handle: u64, vtable: &'static CommandContextVtable) -> Self {
        Self { handle, vtable }
    }

    /// Send a reply message to the command sender.
    pub fn reply(&self, message: String) {
        (self.vtable.reply)(self.handle, AbiString::from(message));
    }

    /// Get the name of the command sender.
    pub fn sender_name(&self) -> AbiString {
        (self.vtable.sender_name)(self.handle)
    }
}

// ── Command API Vtable ───────────────────────────────────────────────────────

/// Command handler function pointer. Takes `CommandContext` by value.
/// The `#[steel_command]` macro borrows it for the plugin developer's function.
pub type CommandHandler = extern "C" fn(ctx: CommandContext) -> CommandResult;

/// Vtable for the command registration API. Set by the host.
#[stabby::stabby]
pub struct CommandApiVtable {
    pub register: extern "C" fn(name: AbiString, handler: CommandHandler),
}

/// Scoped API for command registration. Plugin-side wrapper — not an ABI type.
pub struct CommandApi<'a> {
    vtable: &'a CommandApiVtable,
}

impl<'a> CommandApi<'a> {
    pub(crate) fn new(vtable: &'a CommandApiVtable) -> Self {
        Self { vtable }
    }

    /// Register a command with the given name and handler.
    pub fn register(&self, name: &str, handler: CommandHandler) {
        (self.vtable.register)(AbiString::from(name), handler);
    }
}
