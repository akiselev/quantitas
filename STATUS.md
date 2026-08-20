# Quantitas status

Updated: 2026-08-20
Branch: `master`
Milestone: standalone foundation invariant pass

## Current role

Quantitas owns the shared canonical representation of SI dimensions, quantities,
quantity-kind identities, exact unit scales, unit definitions, and registry
provenance. It depends on neither Resolvent nor Lean Atlas.

## Implemented

- Reduced rational exponents with checked algebra and validating
  deserialization.
- Seven-base SI dimensions with checked product, quotient, integer-power, and
  root operations.
- Finite canonical SI quantities with non-empty kind validation.
- One canonical `ExactScale` representation, including signed-minimum and
  exponent-overflow handling plus validating deserialization.
- Conflict-checked registry construction followed by an immutable, provenance-
  bearing frozen state.
- Frozen-registry serialization/deserialization that rebuilds indices and
  revalidates identities, provenance, and affine point/interval contracts.
- SI bootstrap units and parsing that retains the authored display unit.

Generic quantity arithmetic and the unused sparse dimension-vector seam were
removed. Affine arithmetic belongs in consumer code that knows the quantity-kind
semantics.

## Validation

Passed locally on 2026-08-20 with Rust 1.97.0:

- `cargo fmt --all -- --check`
- `cargo check --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets` — 10 unit tests passed
- `git diff --check`
- Resolvent consumer: `cargo check --all-targets`

## Next

Replace the bootstrap metadata and definitions with an admitted, generated
standards snapshot. Add further consumer-neutral algebra only after both initial
consumers require the same semantics.
