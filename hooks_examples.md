# Codex Hooks System

A simple, extensible hook system for adding functionality to Codex without modifying the base code.

## Overview

The hooks system allows you to register callbacks that are triggered at specific points in the Codex lifecycle. This enables you to:

- Log events and metrics
- Integrate with external systems
- Add custom notifications
- Track usage patterns
- Extend Codex behavior

## Quick Start

```rust
use codex_core::hooks::{Hooks, HookEvent, events};

// Create a hook registry
let hooks = Hooks::new();

// Register a handler for when a turn completes
hooks.register(events::TURN_COMPLETE, |event| {
    println!("Turn completed!");
    if let Some(data) = event.data {
        println!("Data: {}", data);
    }
});

// Trigger the hook
hooks.trigger(events::TURN_COMPLETE, HookEvent::new("turn_complete"));
```

## Available Hook Points

### Core Events

- **`TURN_COMPLETE`** - Triggered when a turn completes successfully
- **`ERROR`** - Triggered when an error occurs during execution
- **`USER_INPUT`** - Triggered when user provides input
- **`RESPONSE_START`** - Triggered when Codex is about to respond
- **`RESPONSE_COMPLETE`** - Triggered when a response is complete

### Tool Events

- **`TOOL_BEFORE`** - Triggered before a tool is invoked
- **`TOOL_AFTER`** - Triggered after a tool completes execution

### Conversation Events

- **`CONVERSATION_START`** - Triggered when a conversation is created
- **`CONVERSATION_END`** - Triggered when a conversation ends

## Usage Patterns

### 1. Simple Logging

```rust
use codex_core::hooks::{Hooks, events};

let hooks = Hooks::new();

hooks.register(events::TURN_COMPLETE, |event| {
    eprintln!("[HOOK] Turn completed");
});

hooks.register(events::ERROR, |event| {
    if let Some(data) = event.data {
        eprintln!("[HOOK] Error occurred: {}", data);
    }
});
```

### 2. Passing Data Through Hooks

```rust
use serde_json::json;
use codex_core::hooks::{Hooks, HookEvent, events};

let hooks = Hooks::new();

hooks.register(events::TOOL_AFTER, |event| {
    if let Some(data) = event.data {
        if let Some(tool_name) = data.get("tool_name") {
            println!("Tool executed: {}", tool_name);
        }
    }
});

// Trigger with data
let event = HookEvent::new(events::TOOL_AFTER)
    .with_data(json!({
        "tool_name": "git_diff",
        "duration_ms": 150,
        "status": "success"
    }));

hooks.trigger(events::TOOL_AFTER, event);
```

### 3. Multiple Handlers

```rust
use codex_core::hooks::{Hooks, events};

let hooks = Hooks::new();

// Register first handler
hooks.register(events::TURN_COMPLETE, |_| {
    println!("Handler 1: Turn complete");
});

// Register second handler - both will be called
hooks.register(events::TURN_COMPLETE, |_| {
    println!("Handler 2: Turn complete");
});

// Output:
// Handler 1: Turn complete
// Handler 2: Turn complete
hooks.trigger(events::TURN_COMPLETE, HookEvent::new(events::TURN_COMPLETE));
```

### 4. Stateful Hooks

```rust
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use codex_core::hooks::{Hooks, events};

let hooks = Hooks::new();
let turn_count = Arc::new(AtomicUsize::new(0));

let counter = turn_count.clone();
hooks.register(events::TURN_COMPLETE, move |_| {
    let count = counter.fetch_add(1, Ordering::Relaxed) + 1;
    println!("Turn #{}", count);
});

hooks.trigger(events::TURN_COMPLETE, HookEvent::new(events::TURN_COMPLETE));
hooks.trigger(events::TURN_COMPLETE, HookEvent::new(events::TURN_COMPLETE));

println!("Total turns: {}", turn_count.load(Ordering::Relaxed)); // 2
```

### 5. Custom Hook Points

```rust
use codex_core::hooks::{Hooks, HookEvent};

let hooks = Hooks::new();

// You can create custom hook points beyond the predefined ones
hooks.register("custom_event", |event| {
    println!("Custom event triggered: {}", event.event_type);
});

hooks.trigger("custom_event", HookEvent::new("custom_event"));
```

## Checking Hook Status

```rust
use codex_core::hooks::Hooks;

let hooks = Hooks::new();

// Check if a hook has handlers
if hooks.has_handlers("turn_complete") {
    println!("Turn complete hook is registered");
}

// Count handlers
println!("Handlers: {}", hooks.handler_count("turn_complete"));

// Clear specific hook
hooks.clear("turn_complete");

// Clear all hooks
hooks.clear_all();
```

## Integration Example

Here's how you might integrate hooks into the Codex conversation flow:

```rust
use codex_core::{
    hooks::{Hooks, HookEvent, events},
    CodexConversation,
};
use serde_json::json;

pub struct CodexWithHooks {
    conversation: CodexConversation,
    hooks: Hooks,
}

impl CodexWithHooks {
    pub fn new(conversation: CodexConversation) -> Self {
        Self {
            conversation,
            hooks: Hooks::new(),
        }
    }

    pub fn register_hook<F>(&self, hook_name: &str, handler: F)
    where
        F: Fn(HookEvent) + Send + Sync + 'static,
    {
        self.hooks.register(hook_name, handler);
    }

    pub async fn send_turn(&self, prompt: &str) {
        // Trigger user input hook
        self.hooks.trigger(
            events::USER_INPUT,
            HookEvent::new(events::USER_INPUT)
                .with_data(json!({ "prompt_length": prompt.len() })),
        );

        // Trigger response start
        self.hooks.trigger(
            events::RESPONSE_START,
            HookEvent::new(events::RESPONSE_START),
        );

        // Do actual work...
        // let response = self.conversation.send_turn(prompt).await;

        // Trigger response complete
        self.hooks.trigger(
            events::RESPONSE_COMPLETE,
            HookEvent::new(events::RESPONSE_COMPLETE),
        );
    }
}
```

## Best Practices

1. **Keep handlers fast** - Hooks execute synchronously, so long operations will block
2. **Handle errors gracefully** - Panicking in a hook won't crash Codex, but will be logged
3. **Use meaningful names** - Use the predefined event names or follow a clear naming convention
4. **Pass structured data** - Use JSON for event data to keep things flexible
5. **Document custom hooks** - If you create custom hook points, document when they're triggered
6. **Clone shared state efficiently** - Use `Arc` for shared data rather than cloning

## Testing Hooks

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_hook_integration() {
        let hooks = Hooks::new();
        let events_fired = Arc::new(Mutex::new(Vec::new()));

        let events_clone = events_fired.clone();
        hooks.register("test_hook", move |event| {
            events_clone.lock().unwrap().push(event.event_type);
        });

        hooks.trigger("test_hook", HookEvent::new("test_hook"));

        let fired = events_fired.lock().unwrap();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0], "test_hook");
    }
}
```

## API Reference

### `Hooks`

- `new()` - Create a new hook registry
- `register(hook_name, handler)` - Register a handler for a hook
- `trigger(hook_name, event)` - Trigger all handlers for a hook
- `handler_count(hook_name)` - Get the number of registered handlers
- `has_handlers(hook_name)` - Check if a hook has registered handlers
- `clear(hook_name)` - Clear all handlers for a specific hook
- `clear_all()` - Clear all hooks

### `HookEvent`

- `new(event_type)` - Create a new event
- `with_data(value)` - Add JSON data to the event

## Future Extensions

The hook system can be easily extended with:

- **Async hooks** - For I/O operations
- **Hook priorities** - Control execution order
- **Hook filters** - Conditional execution
- **Event history** - Record all triggered events
- **Hook middleware** - Pre/post processing

## License

The hooks system is part of Codex and follows the same license terms.
