# Just Right Rust

Just Right Rust is an opinionated collection of Rust recommendations packaged
as a [Symposium](https://github.com/symposium-dev/symposium) plugin.

The plugin currently provides:

- guidance for async Rust built on Tokio, actors, and cancellation-safe
  concurrency;
- Google's unsafe Rust review skill;
- a portable Symposium hook engine.

The hook engine is currently a no-op: every registered hook permits the event
without modifying it.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
