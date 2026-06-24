# qlt

MLIR dialect for the qurts lifetime and region constraint system.
The main purpose of this dialect to provide explicit markers for the verifier when checking `qss.uncompute`'s. It also provides lifetime types for `qss`.

`qlt` is target-agnostic - it knows nothing about quantum gates or classical computation,
only about lifetime variables, their ordering constraints, and the scopes they bound.

## Planned contents

**Types**
- `LifetimeType` - an SSA lifetime token; lifetimes are values in the IR so that
  the point at which a lifetime ends is a concrete, schedulable node in the dataflow graph

**Ops**
- `qlt.region` - opens a lifetime scope; yields a lifetime token and a scoped block
- `qlt.end` - closes a lifetime scope, firing any cleanup obligations attached to it

**Attributes**
- `OrderingConstraintAttr` - encodes `'a <= 'b` and `'a != 'b` constraints on function signatures

## Dependency

No intra-workspace dependencies. Depends on [melior](https://github.com/raviqqe/melior).
