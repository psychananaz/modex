//! Simple hook system for extending Codex functionality without modifying base code.
//!
//! The hook system provides a way to register callbacks that are triggered at specific
//! points in the Codex lifecycle. Hooks are identified by name and support multiple
//! handlers per hook point.
//!
//! # Examples
//!
//! ```ignore
//! use codex_core::hooks::{Hooks, HookEvent};
//!
//! let hooks = Hooks::new();
//!
//! // Register a hook
//! hooks.register("turn_complete", |event| {
//!     println!("Turn completed: {:?}", event);
//! });
//!
//! // Trigger the hook
//! hooks.trigger("turn_complete", HookEvent::default());
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Hook event data passed to hook handlers
#[derive(Debug, Clone, Default)]
pub struct HookEvent {
    /// Event type identifier
    pub event_type: String,
    /// Contextual data as JSON
    pub data: Option<serde_json::Value>,
}

impl HookEvent {
    /// Create a new hook event
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            data: None,
        }
    }

    /// Add data to the event
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Type alias for hook handler functions
type HookHandler = Box<dyn Fn(HookEvent) + Send + Sync>;

/// Hook system for registering and triggering event handlers
///
/// Hooks allow extending Codex behavior without modifying base code.
/// Multiple handlers can be registered for the same hook point.
#[derive(Clone)]
pub struct Hooks {
    handlers: Arc<Mutex<HashMap<String, Vec<Arc<HookHandler>>>>>,
}

impl Hooks {
    /// Create a new empty hook registry
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a handler for a hook point
    ///
    /// Multiple handlers can be registered for the same hook. They will be
    /// called in registration order.
    ///
    /// # Arguments
    ///
    /// * `hook_name` - The name of the hook point (e.g., "turn_complete")
    /// * `handler` - The callback function to execute
    pub fn register<F>(&self, hook_name: impl Into<String>, handler: F)
    where
        F: Fn(HookEvent) + Send + Sync + 'static,
    {
        let hook_name = hook_name.into();
        let handler = Arc::new(Box::new(handler) as HookHandler);

        let mut handlers = self.handlers.lock().unwrap();
        handlers
            .entry(hook_name)
            .or_insert_with(Vec::new)
            .push(handler);
    }

    /// Trigger all handlers registered for a hook point
    ///
    /// Executes all registered handlers in order. If a handler panics,
    /// the panic is caught and logged, but other handlers continue to execute.
    pub fn trigger(&self, hook_name: &str, event: HookEvent) {
        let handlers = self.handlers.lock().unwrap();

        if let Some(hook_handlers) = handlers.get(hook_name) {
            for handler in hook_handlers {
                // Wrap in catch_unwind to prevent one handler panicking from stopping others
                let event = event.clone();
                let handler = Arc::clone(handler);
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handler(event);
                }))
                .unwrap_or_else(|_| {
                    eprintln!("Hook handler for '{}' panicked", hook_name);
                });
            }
        }
    }

    /// Get the count of handlers registered for a hook
    pub fn handler_count(&self, hook_name: &str) -> usize {
        let handlers = self.handlers.lock().unwrap();
        handlers.get(hook_name).map(|h| h.len()).unwrap_or(0)
    }

    /// Check if a hook has any registered handlers
    pub fn has_handlers(&self, hook_name: &str) -> bool {
        let handlers = self.handlers.lock().unwrap();
        handlers.contains_key(hook_name)
    }

    /// Clear all handlers for a specific hook
    pub fn clear(&self, hook_name: &str) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.remove(hook_name);
    }

    /// Clear all hooks
    pub fn clear_all(&self) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.clear();
    }
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard hook point names for common Codex events
pub mod events {
    /// Triggered when a turn completes successfully
    pub const TURN_COMPLETE: &str = "turn_complete";

    /// Triggered when an error occurs during execution
    pub const ERROR: &str = "error";

    /// Triggered before a tool is invoked
    pub const TOOL_BEFORE: &str = "tool_before";

    /// Triggered after a tool completes execution
    pub const TOOL_AFTER: &str = "tool_after";

    /// Triggered when user provides input
    pub const USER_INPUT: &str = "user_input";

    /// Triggered when Codex is about to respond
    pub const RESPONSE_START: &str = "response_start";

    /// Triggered when a response is complete
    pub const RESPONSE_COMPLETE: &str = "response_complete";

    /// Triggered when a conversation is created
    pub const CONVERSATION_START: &str = "conversation_start";

    /// Triggered when a conversation ends
    pub const CONVERSATION_END: &str = "conversation_end";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc as StdArc;

    #[test]
    fn test_register_and_trigger() {
        let hooks = Hooks::new();
        let counter = StdArc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        hooks.register("test_hook", move |_| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        hooks.trigger("test_hook", HookEvent::new("test"));
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        hooks.trigger("test_hook", HookEvent::new("test"));
        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_multiple_handlers() {
        let hooks = Hooks::new();
        let counter1 = StdArc::new(AtomicUsize::new(0));
        let counter2 = StdArc::new(AtomicUsize::new(0));

        let c1 = counter1.clone();
        hooks.register("test_hook", move |_| {
            c1.fetch_add(1, Ordering::Relaxed);
        });

        let c2 = counter2.clone();
        hooks.register("test_hook", move |_| {
            c2.fetch_add(1, Ordering::Relaxed);
        });

        hooks.trigger("test_hook", HookEvent::new("test"));

        assert_eq!(counter1.load(Ordering::Relaxed), 1);
        assert_eq!(counter2.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_handler_count() {
        let hooks = Hooks::new();
        assert_eq!(hooks.handler_count("test_hook"), 0);

        hooks.register("test_hook", |_| {});
        assert_eq!(hooks.handler_count("test_hook"), 1);

        hooks.register("test_hook", |_| {});
        assert_eq!(hooks.handler_count("test_hook"), 2);
    }

    #[test]
    fn test_clear() {
        let hooks = Hooks::new();
        hooks.register("test_hook", |_| {});
        hooks.register("test_hook", |_| {});
        assert_eq!(hooks.handler_count("test_hook"), 2);

        hooks.clear("test_hook");
        assert_eq!(hooks.handler_count("test_hook"), 0);
    }

    #[test]
    fn test_hook_event_data() {
        let hooks = Hooks::new();
        let received = StdArc::new(Mutex::new(None));
        let received_clone = received.clone();

        hooks.register("test_hook", move |event| {
            *received_clone.lock().unwrap() = Some(event);
        });

        let data = serde_json::json!({ "key": "value" });
        hooks.trigger("test_hook", HookEvent::new("test").with_data(data.clone()));

        let received_event = received.lock().unwrap();
        assert!(received_event.is_some());
        assert_eq!(received_event.as_ref().unwrap().data, Some(data));
    }
}
