# Quantitas status

Updated: 2026-08-20
Milestone: standalone foundation extraction

## Current role

Quantitas owns the canonical shared representation of rational dimensions, quantities,
quantity kinds, units, and unit-registry provenance. Resolvent and Lean Atlas depend on it;
Quantitas depends on neither consumer.

## Implemented

- checked reduced rational exponents;
- copyable seven-base SI dimensions and generic sparse dimension vectors;
- canonical quantities and quantity-kind identifiers;
- exact rational/decimal scales and affine point-versus-interval conversion;
- unit definitions, registry snapshots, symbol lookup, and a standards-oriented bootstrap
  registry;
- parsing that preserves authored display units.

## Validation

Passed on 2026-08-20 with Rust 1.97.0:

```text
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test                         # 2 passed
```

## Next

1. Replace bootstrap registry metadata with an admitted, generated standards snapshot.
2. Add only consumer-neutral dimensional algebra proven useful by both Resolvent and Lean
   Atlas.
