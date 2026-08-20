# Quantitas

Quantitas is the shared dimensional-semantics foundation used by Resolvent and Lean Atlas.
It owns exact rational exponents, SI dimensions, generic dimension vectors, canonical
quantities, quantity-kind identity, exact unit conversions, affine point/interval handling,
and registry provenance.

Quantitas has no knowledge of fields, forms, solvers, meshes, or Lean declarations.
Consumers provide their own scientific quantity-kind catalogs and inference policies.

The crate replaces both `sinbad-league` and `resolvent-quantities`; there is deliberately
no compatibility module between their former representations.

