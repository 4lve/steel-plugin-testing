use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use steel_api::{AbiString, Event, EventApiVtable, EventResult, RawEventHandler};

// ── Registry ─────────────────────────────────────────────────────────────────

static EVENTS: LazyLock<Mutex<HashMap<String, Vec<RawEventHandler>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

extern "C" fn host_register_event(name: AbiString, handler: RawEventHandler) {
    let name_str = name.as_str().to_owned();
    println!("[Host] Registered event handler: {name_str}");
    EVENTS
        .lock()
        .unwrap()
        .entry(name_str)
        .or_default()
        .push(handler);
}

pub(crate) static EVENT_API: EventApiVtable = EventApiVtable {
    register: host_register_event,
};

// ── Dispatch ─────────────────────────────────────────────────────────────────

pub fn fire_event<E: Event>(event: &mut E) -> EventResult {
    let lock = EVENTS.lock().unwrap();
    let Some(handlers) = lock.get(E::NAME) else {
        return EventResult::Continue;
    };
    for handler in handlers {
        match handler((event as *mut E).cast::<u8>()) {
            EventResult::Continue => {}
            other => return other,
        }
    }
    EventResult::Continue
}
