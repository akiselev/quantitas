# Quantitas

Quantitas is the consumer-neutral dimensional-semantics foundation shared by
Resolvent and Lean Atlas. It owns:

- reduced rational exponents over the seven SI base dimensions;
- stable quantity-kind and unit identities;
- finite canonical SI quantities;
- canonical exact rational/decimal unit scales; and
- provenance-bearing unit registries with explicit affine point/interval units.

`ExactScale` is exact metadata. Canonicalization deliberately crosses into a
finite `f64` SI magnitude when it constructs a `Quantity`; non-finite results are
rejected.

A registry starts as a mutable builder returned by `UnitRegistry::empty`.
Insertion is conflict checked, and `freeze` validates its provenance snapshot and
all affine interval references. Only a frozen registry can canonicalize values or
be serialized. Deserialization rebuilds the symbol index and repeats validation.

Quantitas does not define scientific kind catalogs, infer kinds, or perform
quantity arithmetic: affine points and intervals make generic arithmetic
semantically unsafe. Consumers own those policies and operations. Quantitas also
has no knowledge of fields, forms, Lean declarations, meshes, runtimes, or
solvers.
