use crate::AbiString;

// ── Results ─────────────────────────────────────────────────────────

#[stabby::stabby]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// Handler completed normally.
    Continue,
    /// The handler panicked. Host should initiate graceful shutdown.
    Panic,
}

/// Result of firing an event through the host.
#[stabby::stabby]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireResult {
    /// Whether any handler panicked.
    pub result: EventResult,
    /// Whether the event was cancelled by a handler.
    pub cancelled: bool,
}

// ── Directionality markers ──────────────────────────────────────────

/// Marker trait: plugins can register handlers for this event.
pub trait Receivable {}

/// Marker trait: plugins can fire this event.
pub trait Sendable {}

/// Marker trait for events that can be cancelled by handlers.
pub trait Cancellable {}

// ── Event trait ─────────────────────────────────────────────────────

/// Associates an event struct with its string name and cancellability.
///
/// All event types must be `#[stabby::stabby]` (which gives `IStable`,
/// which gives `stabby::Any` — the backing for cross-plugin type identity).
pub trait Event: stabby::abi::IStable + Sized {
    /// Namespaced event name, e.g. `"steel:server_starting"`.
    const NAME: &'static str;
    /// Whether this event supports cancellation.
    const IS_CANCELLABLE: bool = false;
}

/// Convenience macro for declaring events.
///
/// ```ignore
/// event!(MyEvent, "namespace:name");                           // send + receive
/// event!(MyEvent, "namespace:name", cancellable);              // send + receive, cancellable
/// event!(MyEvent, "namespace:name", receive);                  // receive only
/// event!(MyEvent, "namespace:name", send);                     // send only
/// event!(MyEvent, "namespace:name", receive, cancellable);     // receive only, cancellable
/// event!(MyEvent, "namespace:name", send, cancellable);        // send only, cancellable
/// ```
#[macro_export]
macro_rules! event {
    // Default: send + receive
    ($ty:ty, $name:literal) => {
        impl $crate::Sendable for $ty {}
        impl $crate::Receivable for $ty {}
        impl $crate::Event for $ty {
            const NAME: &'static str = $name;
        }
    };
    // send + receive, cancellable
    ($ty:ty, $name:literal, cancellable) => {
        impl $crate::Sendable for $ty {}
        impl $crate::Receivable for $ty {}
        impl $crate::Cancellable for $ty {}
        impl $crate::Event for $ty {
            const NAME: &'static str = $name;
            const IS_CANCELLABLE: bool = true;
        }
    };
    // receive only
    ($ty:ty, $name:literal, receive) => {
        impl $crate::Receivable for $ty {}
        impl $crate::Event for $ty {
            const NAME: &'static str = $name;
        }
    };
    // send only
    ($ty:ty, $name:literal, send) => {
        impl $crate::Sendable for $ty {}
        impl $crate::Event for $ty {
            const NAME: &'static str = $name;
        }
    };
    // receive only, cancellable
    ($ty:ty, $name:literal, receive, cancellable) => {
        impl $crate::Receivable for $ty {}
        impl $crate::Cancellable for $ty {}
        impl $crate::Event for $ty {
            const NAME: &'static str = $name;
            const IS_CANCELLABLE: bool = true;
        }
    };
    // send only, cancellable
    ($ty:ty, $name:literal, send, cancellable) => {
        impl $crate::Sendable for $ty {}
        impl $crate::Cancellable for $ty {}
        impl $crate::Event for $ty {
            const NAME: &'static str = $name;
            const IS_CANCELLABLE: bool = true;
        }
    };
}

// ── Built-in events ─────────────────────────────────────────────────

#[stabby::stabby]
pub struct ServerStartingEvent {
    pub block_count: u32,
}

event!(ServerStartingEvent, "steel:server_starting", receive);

// ── Ordering constraints ────────────────────────────────────────────

#[stabby::stabby]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderingConstraint {
    /// This handler must run before the named plugin's handler.
    Before,
    /// This handler must run after the named plugin's handler.
    After,
}

/// A single ordering constraint: (kind, target_plugin_id).
#[stabby::stabby]
#[derive(Debug, Clone)]
pub struct HandlerOrdering {
    pub constraint: OrderingConstraint,
    pub plugin_id: AbiString,
}

// ── Raw handler type ────────────────────────────────────────────────

/// Type-erased event handler used internally for dispatch.
///
/// - `event_data`: pointer to the event struct
/// - `cancelled`: mutable cancellation flag — handlers can set or clear it
pub type RawEventHandler = extern "C" fn(event_data: *mut u8, cancelled: *mut bool) -> EventResult;

// ── Vtable arg bundles ──────────────────────────────────────────────
// Stabby's IStable computation is exponential in fn-pointer arity,
// so we bundle parameters into structs and keep vtable fn pointers
// to a single argument each.

#[stabby::stabby(no_opt)]
pub struct RegisterArgs {
    pub plugin_id: AbiString,
    pub event_name: AbiString,
    pub handler: RawEventHandler,
    pub receive_cancelled: bool,
    pub orderings_ptr: *const HandlerOrdering,
    pub orderings_len: u32,
}

#[stabby::stabby(no_opt)]
pub struct FireArgs {
    pub event_name: AbiString,
    pub event_data: *mut u8,
    pub is_cancellable: bool,
}

// ── Event API Vtable ────────────────────────────────────────────────

#[stabby::stabby]
pub struct EventApiVtable {
    pub register: extern "C" fn(args: RegisterArgs),
    pub fire: extern "C" fn(args: FireArgs) -> FireResult,
}

// ── EventApi — plugin-side wrapper ──────────────────────────────────

/// Scoped API for event registration and custom event firing.
/// Carries the plugin_id so registrations are automatically tagged.
pub struct EventApi<'a> {
    vtable: &'a EventApiVtable,
    plugin_id: &'a str,
}

impl<'a> EventApi<'a> {
    pub fn new(vtable: &'a EventApiVtable, plugin_id: &'a str) -> Self {
        Self { vtable, plugin_id }
    }

    /// Begin building a handler registration for event `E`.
    pub fn on<E: Event + Receivable>(
        &self,
        handler: extern "C" fn(&mut E, &mut bool) -> EventResult,
    ) -> HandlerBuilder<'a> {
        // SAFETY: `&mut E` and `*mut u8` have identical ABI representation,
        // and `&mut bool` is `&mut bool`. The transmute just erases the concrete event type.
        let raw: RawEventHandler = unsafe { core::mem::transmute(handler) };
        HandlerBuilder {
            vtable: self.vtable,
            plugin_id: self.plugin_id,
            handler: raw,
            event_name: E::NAME,
            receive_cancelled: false,
            orderings: Vec::new(),
        }
    }

    /// Shorthand: register a handler with default options.
    pub fn register<E: Event + Receivable>(
        &self,
        handler: extern "C" fn(&mut E, &mut bool) -> EventResult,
    ) {
        self.on::<E>(handler).register();
    }

    /// Fire an event so that all registered handlers are called.
    pub fn fire<E: Event + Sendable>(&self, event: &mut E) -> FireResult {
        self.emit(event)
    }

    /// Fire an event without requiring `Sendable`.
    ///
    /// Intended for the plugin/crate that defines the event — use `fire`
    /// everywhere else so the `Sendable` bound catches misuse.
    pub fn emit<E: Event>(&self, event: &mut E) -> FireResult {
        (self.vtable.fire)(FireArgs {
            event_name: AbiString::from(E::NAME),
            event_data: (event as *mut E).cast::<u8>(),
            is_cancellable: E::IS_CANCELLABLE,
        })
    }

    /// Get the raw vtable for later use (e.g. storing for firing events
    /// outside of `init`). The returned reference is `'static`.
    pub fn vtable(&self) -> &'a EventApiVtable {
        self.vtable
    }
}

// ── HandlerBuilder ──────────────────────────────────────────────────

/// Builder for configuring and registering an event handler.
pub struct HandlerBuilder<'a> {
    vtable: &'a EventApiVtable,
    plugin_id: &'a str,
    handler: RawEventHandler,
    event_name: &'static str,
    receive_cancelled: bool,
    orderings: Vec<HandlerOrdering>,
}

impl<'a> HandlerBuilder<'a> {
    /// Opt in to receiving events that have already been cancelled.
    pub fn receive_cancelled(mut self) -> Self {
        self.receive_cancelled = true;
        self
    }

    /// Declare that this handler must run before the given plugin's handler.
    pub fn before(mut self, plugin_id: &str) -> Self {
        self.orderings.push(HandlerOrdering {
            constraint: OrderingConstraint::Before,
            plugin_id: AbiString::from(plugin_id),
        });
        self
    }

    /// Declare that this handler must run after the given plugin's handler.
    pub fn after(mut self, plugin_id: &str) -> Self {
        self.orderings.push(HandlerOrdering {
            constraint: OrderingConstraint::After,
            plugin_id: AbiString::from(plugin_id),
        });
        self
    }

    /// Finalize the registration.
    pub fn register(self) {
        let ptr = if self.orderings.is_empty() {
            core::ptr::null()
        } else {
            self.orderings.as_ptr()
        };
        let len = self.orderings.len() as u32;

        (self.vtable.register)(RegisterArgs {
            plugin_id: AbiString::from(self.plugin_id),
            event_name: AbiString::from(self.event_name),
            handler: self.handler,
            receive_cancelled: self.receive_cancelled,
            orderings_ptr: ptr,
            orderings_len: len,
        });
    }
}
