---
name: async-rust
description: Design, implement, debug, or review asynchronous Rust according to this project's Tokio, actor, synchronization, race, and stream conventions.
---

# Async Rust

Use Tokio for asynchronous Rust. Prefer designs that avoid shared mutable state, `tokio::select!`, cancellation hazards, and futurelock.

## Build on Tokio

- Use Tokio as the async runtime and ecosystem.
- Prefer Tokio's task, channel, time, networking, and I/O facilities.
- Do not introduce another async runtime or runtime-neutral abstraction unless the request specifically requires it.

## Prefer actors over shared mutable state

When a component has mutable state, prefer a Tokio actor based on Alice Ryhl's [Actors with Tokio](https://ryhl.io/blog/actors-with-tokio/) pattern.

Split the actor into:

- a private actor struct that exclusively owns the mutable state and the receiving half of its mailbox;
- a spawned Tokio task that owns the actor and runs its message loop;
- a cloneable public handle containing the sending half of the mailbox;
- a private message enum describing the operations clients may request.

Follow these conventions:

- Create a bounded `tokio::sync::mpsc` channel for the mailbox.
- Construct the actor and spawn its task when constructing the handle.
- Move the actor into `tokio::spawn`; do not spawn a future that borrows the actor.
- Keep mutable state inside the actor. Handles must not expose or share access to it.
- Implement the actor loop as receiving and handling one message at a time. Message handling may await because the actor task retains exclusive ownership of its state.
- Put arguments directly in message variants.
- For request-response operations, include a `tokio::sync::oneshot::Sender` in the message and await its receiver in the handle method.
- Let handle methods present an ordinary domain API. Channel and message details remain private.
- Ignore a failed oneshot response when it only means the caller is no longer interested.
- Use mailbox closure for graceful shutdown where possible: when all handles are dropped, `recv()` returns `None` and the actor exits.
- Choose mailbox capacity deliberately. Do not use an unbounded channel.
- Avoid cycles in which actors retain one another's handles. Such cycles can prevent shutdown, and cycles of bounded-channel sends can deadlock.
- Use separate handle types when different clients should be allowed to send different subsets of messages.

The essential shape is:

```rust
struct Actor {
    receiver: tokio::sync::mpsc::Receiver<Message>,
    state: State,
}

enum Message {
    Get {
        respond_to: tokio::sync::oneshot::Sender<Value>,
    },
}

#[derive(Clone)]
pub struct ActorHandle {
    sender: tokio::sync::mpsc::Sender<Message>,
}
```

## Encapsulate unavoidable shared mutable state

Some state, such as a widely shared logging context, may not fit the actor pattern. In that case:

- Prefer `parking_lot::Mutex` or `std::sync::Mutex` over `tokio::sync::Mutex`.
- Put the mutex and protected state in a dedicated struct.
- Make the fields private so callers cannot lock or mutate the state directly.
- Provide narrow methods that acquire the lock, perform the complete read or edit, and return an owned result.
- Do not return guards or references into the protected state.
- Keep the critical section synchronous and release the guard before any `.await`.

Use an async mutex only when it is genuinely difficult to structure the operation so that the guard is released before awaiting. Try an actor or a synchronous accessor first.

### Never acquire a mutex in a `match`

Do not put a lock acquisition in the scrutinee of a `match`. Temporaries in a `match` scrutinee can live for the entire match, keeping the guard held while an arm executes.

```rust
// Avoid.
match state.lock().current_mode() {
    Mode::Ready => act().await,
    Mode::Stopped => {}
}

// Prefer.
let mode = state.current_mode();
match mode {
    Mode::Ready => act().await,
    Mode::Stopped => {}
}
```

The accessor should acquire the lock internally and return an owned snapshot.

## Avoid `select!`

Do not reach for `tokio::select!` as a general control-flow tool. It makes cancellation and polling behavior difficult to reason about.

Prefer:

- an owned race combinator when exactly one of several operations should finish first;
- merged streams when several sources produce an ongoing sequence of events;
- actors and message passing for stateful event loops.

### Race operations

A race combinator should own the competing futures and promptly drop the losers. Before racing an operation, verify that cancelling it at any `.await` point is safe:

- partial reads, writes, sends, or state changes must not be lost or duplicated;
- dropping the future must release any resources it owns;
- background work must not continue unnoticed;
- retrying the operation must have defined semantics.

Also verify that the future returned by the race combinator is itself cancellation-safe: dropping it must cancel and release every competitor without leaving background work or retained resources behind.

Do not keep a losing future alive for later unless its continued polling is independently guaranteed.

### Merge event streams

When several event sources must be handled continuously, convert them into streams of events and merge those streams. Consume the merged stream in one loop and dispatch each event.

- Merge events, not futures representing partially completed operations.
- Keep mutable state in the consuming actor.
- Do not use buffered streams of futures as an event multiplexer.
- Ensure the merge abstraction continues polling every pending source correctly.

## Avoid futurelock

Futurelock is a deadlock in which one future owns or is first in line for a resource needed by another future, but the task responsible for both has stopped polling the first future.

Prevent it by construction:

- do not poll a future and then leave it alive but unpolled;
- do not race borrowed futures that survive after another branch wins;
- do not await unrelated work while pending futures in the same task are temporarily no longer being polled;
- prefer actors, independently spawned tasks, owned race combinators, and merged event streams;
- be suspicious of pinned futures retained across iterations and streams that internally buffer in-flight futures.

If asynchronous code hangs while a mutex, semaphore, or bounded channel appears available, investigate whether an earlier future is still queued for that resource but is no longer being polled.

## Review checklist

Before finishing async Rust work, verify:

- the implementation uses Tokio;
- mutable state is owned by an actor where practical;
- actor state and message types are private behind a handle API;
- unavoidable shared state is encapsulated behind synchronous mutex accessors;
- no mutex is acquired in a `match` scrutinee;
- no mutex guard crosses an `.await`;
- `select!` has been replaced with an owned race, merged stream, or actor design;
- every raced future is cancellation-safe;
- no future can remain alive after its task stops polling it;
- bounded channels cannot form a send cycle.
