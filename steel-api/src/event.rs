use crate::AbiString;

#[stabby::stabby]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    Continue,
    Cancel,
    /// The handler panicked. Host should initiate graceful shutdown.
    Panic,
}

/// Associates an event struct with its string name.
///
/// Event names are namespaced strings (e.g. `"steel:server_starting"`).
/// New events can be added without breaking the ABI — plugins simply
/// don't register for events they don't know about.
pub trait Event: Sized {
    const NAME: &'static str;
}

#[stabby::stabby]
pub struct ServerStartingEvent {
    pub block_count: u32,
}

impl Event for ServerStartingEvent {
    const NAME: &'static str = "steel:server_starting";
}

/// Type-erased event handler used internally for dispatch.
pub type RawEventHandler = extern "C" fn(*mut u8) -> EventResult;

// ── Event API Vtable ─────────────────────────────────────────────────────────

#[stabby::stabby]
pub struct EventApiVtable {
    pub register: extern "C" fn(name: AbiString, handler: RawEventHandler),
}

/// Scoped API for event registration. Plugin-side wrapper — not an ABI type.
pub struct EventApi<'a> {
    vtable: &'a EventApiVtable,
}

impl<'a> EventApi<'a> {
    pub(crate) fn new(vtable: &'a EventApiVtable) -> Self {
        Self { vtable }
    }

    /// Register a typed event handler.
    pub fn register<E: Event>(&self, handler: extern "C" fn(&mut E) -> EventResult) {
        // SAFETY: &mut E and *mut u8 have identical ABI representation.
        let raw: RawEventHandler = unsafe { core::mem::transmute(handler) };
        (self.vtable.register)(AbiString::from(E::NAME), raw);
    }
}
