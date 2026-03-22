use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{LazyLock, Mutex};

use steel_api::{
    Event, EventApiVtable, EventResult, FireArgs, FireResult, OrderingConstraint, RegisterArgs,
    RawEventHandler,
};

// ── Handler Record ──────────────────────────────────────────────────

struct HandlerRecord {
    plugin_id: String,
    handler: RawEventHandler,
    receive_cancelled: bool,
    orderings: Vec<(OrderingConstraint, String)>,
}

// ── Registry ────────────────────────────────────────────────────────

struct EventRegistry {
    /// event_name -> registered handlers (insertion order)
    handlers: HashMap<String, Vec<HandlerRecord>>,
    /// event_name -> sorted handler indices (cached, invalidated on registration)
    sorted_order: HashMap<String, Vec<usize>>,
}

impl EventRegistry {
    fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            sorted_order: HashMap::new(),
        }
    }
}

static REGISTRY: LazyLock<Mutex<EventRegistry>> =
    LazyLock::new(|| Mutex::new(EventRegistry::new()));

// ── Hash-based ordering ─────────────────────────────────────────────

fn plugin_id_hash(plugin_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    plugin_id.hash(&mut hasher);
    hasher.finish()
}

/// Topological sort with hash-based tiebreaking (Kahn's algorithm).
/// Returns ordered indices into the handlers vec.
fn compute_order(handlers: &[HandlerRecord]) -> Vec<usize> {
    let n = handlers.len();
    if n == 0 {
        return vec![];
    }

    // Map plugin_id -> all handler indices for that plugin (for this event).
    let mut plugin_indices: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, h) in handlers.iter().enumerate() {
        plugin_indices
            .entry(h.plugin_id.as_str())
            .or_default()
            .push(i);
    }

    // Build adjacency list: edge a -> b means "a runs before b".
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    let mut in_degree: Vec<usize> = vec![0; n];

    for (i, handler) in handlers.iter().enumerate() {
        for (constraint, target_id) in &handler.orderings {
            let Some(target_indices) = plugin_indices.get(target_id.as_str()) else {
                continue; // target plugin has no handler for this event
            };
            for &j in target_indices {
                match constraint {
                    OrderingConstraint::Before => {
                        // i must run before j
                        adj[i].push(j);
                        in_degree[j] += 1;
                    }
                    OrderingConstraint::After => {
                        // i must run after j => j runs before i
                        adj[j].push(i);
                        in_degree[i] += 1;
                    }
                }
            }
        }
    }

    // Min-heap on (hash, index) for deterministic tiebreaking.
    let hashes: Vec<u64> = handlers
        .iter()
        .map(|h| plugin_id_hash(&h.plugin_id))
        .collect();

    let mut heap: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();
    for i in 0..n {
        if in_degree[i] == 0 {
            heap.push(Reverse((hashes[i], i)));
        }
    }

    let mut result = Vec::with_capacity(n);
    while let Some(Reverse((_, idx))) = heap.pop() {
        result.push(idx);
        for &next in &adj[idx] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                heap.push(Reverse((hashes[next], next)));
            }
        }
    }

    if result.len() != n {
        eprintln!(
            "[Steel] WARNING: Circular ordering constraints detected. \
             Falling back to hash order for unresolvable handlers."
        );
        let in_result: std::collections::HashSet<usize> = result.iter().copied().collect();
        let mut remaining: Vec<usize> = (0..n).filter(|i| !in_result.contains(i)).collect();
        remaining.sort_by_key(|&i| hashes[i]);
        result.extend(remaining);
    }

    result
}

// ── FFI callbacks ───────────────────────────────────────────────────

extern "C" fn host_register_event(args: RegisterArgs) {
    let plugin_id_str = args.plugin_id.as_str().to_owned();
    let event_name_str = args.event_name.as_str().to_owned();

    // Copy ordering constraints from the raw pointer.
    let orderings: Vec<(OrderingConstraint, String)> = if args.orderings_len > 0
        && !args.orderings_ptr.is_null()
    {
        let slice = unsafe {
            core::slice::from_raw_parts(args.orderings_ptr, args.orderings_len as usize)
        };
        slice
            .iter()
            .map(|o| (o.constraint, o.plugin_id.as_str().to_owned()))
            .collect()
    } else {
        vec![]
    };

    let ordering_desc = if orderings.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = orderings
            .iter()
            .map(|(c, id)| {
                let kind = match c {
                    OrderingConstraint::Before => "before",
                    OrderingConstraint::After => "after",
                };
                format!("{kind} {id}")
            })
            .collect();
        format!(" ({})", parts.join(", "))
    };

    println!(
        "[Host] Registered event handler: {event_name_str} (plugin: {plugin_id_str}){ordering_desc}"
    );

    let record = HandlerRecord {
        plugin_id: plugin_id_str,
        handler: args.handler,
        receive_cancelled: args.receive_cancelled,
        orderings,
    };

    let mut reg = REGISTRY.lock().unwrap();
    reg.handlers
        .entry(event_name_str.clone())
        .or_default()
        .push(record);
    // Invalidate cached sort order.
    reg.sorted_order.remove(&event_name_str);
}

extern "C" fn host_fire_event(args: FireArgs) -> FireResult {
    let (result, cancelled) =
        fire_event_raw(args.event_name.as_str(), args.event_data, args.is_cancellable);
    FireResult { result, cancelled }
}

// ── Public API vtable ───────────────────────────────────────────────

pub(crate) static EVENT_API: EventApiVtable = EventApiVtable {
    register: host_register_event,
    fire: host_fire_event,
};

// ── Internal dispatch ───────────────────────────────────────────────

fn fire_event_raw(event_name: &str, event_data: *mut u8, is_cancellable: bool) -> (EventResult, bool) {
    let mut reg = REGISTRY.lock().unwrap();

    if !reg.handlers.contains_key(event_name) {
        return (EventResult::Continue, false);
    }

    // Compute and cache sort order if needed.
    if !reg.sorted_order.contains_key(event_name) {
        let computed = compute_order(&reg.handlers[event_name]);
        reg.sorted_order.insert(event_name.to_owned(), computed);
    }

    let order = reg.sorted_order[event_name].clone();
    let handlers = &reg.handlers[event_name];

    let mut cancelled = false;

    for &idx in &order {
        let handler = &handlers[idx];

        // Skip cancelled events for handlers that didn't opt in.
        if cancelled && !handler.receive_cancelled {
            continue;
        }

        // Handlers can freely set/clear cancelled.
        // If the event isn't cancellable, we still pass the flag but ignore writes.
        let mut handler_cancelled = cancelled;
        match (handler.handler)(event_data, &mut handler_cancelled as *mut bool) {
            EventResult::Continue => {
                if is_cancellable {
                    cancelled = handler_cancelled;
                }
            }
            EventResult::Panic => return (EventResult::Panic, cancelled),
        }
    }

    (EventResult::Continue, cancelled)
}

/// Typed fire for host-side use.
pub fn fire_event<E: Event>(event: &mut E) -> FireResult {
    let (result, cancelled) = fire_event_raw(
        E::NAME,
        (event as *mut E).cast::<u8>(),
        E::IS_CANCELLABLE,
    );
    FireResult { result, cancelled }
}
