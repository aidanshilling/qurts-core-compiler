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
- `qduc.newlft` - opens a new lifetime scope, producing its `!qduc.lt` token as an ordinary op
  result (not region-scoped — lifetimes aren't structurally nested, since source-level lifetimes
  with no ordering constraint between them may have crossing `newlft`/`endlft` spans, which MLIR
  regions can't represent). Well-formedness (exactly one `qduc.end` per token, no use of it after
  that `qduc.end`) is enforced by this op's verifier instead of by region/dominance structure.
- `qduc.end` - closes a lifetime scope, consuming its token. `qauc`'s own ops (`borrow`/
  `unique_borrow`) layer a second verifier on top, checking that values *derived* from a token
  (e.g. `qauc.unique_borrow`'s result) don't outlive it either — something a `qduc`-only check
  can't see, since that derived value's uses never appear in the token's own use-list.
  `qduc.end` also declares a `Free` memory effect scoped to the specific `%lt` value (not just the
  `LifetimeResource` kind); `qauc.borrow`/`qauc.unique_borrow` declare a matching value-scoped
  `Read`. This is advisory information for effect-aware optimizations (CSE, LICM, the greedy
  rewriter) so they see the conflict and don't reorder past `qduc.end` in the first place, instead
  of only finding out from the verifier after the fact. It doesn't replace the verifier — a pass
  that ignores effects can still produce violating IR — but it means well-behaved passes usually
  never attempt the illegal move.

**Attributes**
- `OrderingConstraintAttr` - encodes `'a <= 'b` and `'a != 'b` constraints on function signatures

## Build

See [`cpp/README.md`](cpp/README.md) for the C++ dialect build (tablegen + CMake), which the
Rust crate's `build.rs` links against. Then `cargo build` / `cargo test` as usual.

## Dependency

No intra-workspace dependencies. Depends on [melior](https://github.com/raviqqe/melior).
