# qduc

MLIR dialect for the qurts lifetime and region constraint system.
The main purpose of this dialect to provide explicit markers for the verifier when checking `qauc.uncompute`'s. It also provides lifetime types for `qauc`.

`qduc` is target-agnostic - it knows nothing about quantum gates or classical computation,
only about lifetime variables, their ordering constraints, and the scopes they bound.

## Planned contents

**Types**
- `LifetimeType` - an SSA lifetime token; lifetimes are values in the IR so that
  the point at which a lifetime ends is a concrete, schedulable node in the dataflow graph

**Ops**
- `qduc.region` - opens a lifetime scope; yields a lifetime token and a scoped block
- `qduc.end` - closes a lifetime scope, firing any cleanup obligations attached to it

**Attributes**
- `OrderingConstraintAttr` - encodes `'a <= 'b` and `'a != 'b` constraints on function signatures

## Dependency

No intra-workspace dependencies. Depends on [melior](https://github.com/raviqqe/melior).
