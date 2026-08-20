# Agent instructions

Quantitas is a consumer-neutral foundation. It must not depend on Sinbad, Resolvent,
Lean Atlas, or any scientific-stack runtime.

Keep one canonical representation for dimensions, quantities, kinds, and units. Do not
add compatibility bridges for consumer-owned types. Consumer-specific physical catalogs
belong in the consumer.

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
before handoff. Keep `STATUS.md` current and compact.

