# Quantitas status

2026-09-18 bounded implementation accepted:
ThermalExpansionCoefficient is a distinct quantity kind with exact inverse-temperature dimension. The thermoelastic consumer supplies the material value; Quantitas has no material catalog. Owner gate passed: 28 tests, formatting, strict clippy, rustdoc and doctests; final consumer acceptance passed (210 tests across 35 targets, documented external-fixture retry).


Historical SC-W2 checkpoint (2026-09-17): HeatFlux / HeatFluxDensity is the standard W/m² kind,
distinct from Power. Exact integration over area and time yields Energy.
Owner gate: 27 tests, fmt, strict all-feature clippy, rustdoc and doctests pass.


Updated: 2026-09-17
Branch: `master`
Milestone: conserved boundary-flux quantity kinds over the standalone foundation

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

Final cross-repository evidence: [September 18 acceptance](../sinbad/docs/validation/2026-09-18-assembly/README.md).
