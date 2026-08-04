# mlrd

MLIR dialect for the operations a qurts program actually performs — as opposed to `qduc`
(lifetime tokens) and `qauc` (sub-structural ownership bookkeeping). `mlrd` depends on `qauc`
(and transitively `qduc`) for its qubit/ref/unique types, but neither of those dialects knows
anything about `mlrd`.

## Planned contents

**Ops**
- `mlrd.lifted` - opaque application of a lifted (lifetime-preserving) classical function,
  e.g. state prep, X, CX. Moved here from `qauc` — it's a program operation, not an ownership
  obligation.
- `mlrd.qif` - coherent quantum conditional over a borrowed control qubit (`!qauc.ref<lt,
  !qauc.qbit>`). Unlike `scf.if`, this is not a branch: both regions unitarily contribute:
  neither is "not executed". Its verifier checks the condition is qubit-valued and that both
  regions are a single block ending in `mlrd.yield` with operand types matching the op's results.
- `mlrd.yield` - terminates a `mlrd.qif` branch region. Can't reuse `scf.yield` here — its
  verifier hardcodes an allowed-parent list (`scf.if`/`scf.for`/...) that doesn't include
  `mlrd.qif`, so it rejects being nested inside one (found by the Rust dialect tests, not
  anticipated up front).

  **Known gap**: the verifier does not yet check that both regions discharge the same
  lifetime/uniqueness obligations (open the same lifetimes, `qauc.release` the same unique
  values). Per the paper's QIF typing rule, that symmetry is required — there's no "which branch
  ran" once you're coherent, so an obligation satisfied in only one branch is unsound. This needs
  walking both regions' ops and diffing their `qduc.end`/`qauc.release` sets, which is a bigger
  design problem than the structural checks already in place; not implemented yet.
- (future) measurement, real gate ops (`X`, `H`, `CNOT`, ...) once pass 3 needs something to trace

## Build

See [`cpp/README.md`](cpp/README.md) for the C++ dialect build (tablegen + CMake), which the
Rust crate's `build.rs` links against. Then `cargo build` / `cargo test` as usual.

## Dependencies

- `qauc` (path dependency, transitively `qduc`) - qubit/ref/unique types used in `mlrd.qif`'s
  condition and `mlrd.lifted`'s operands/result
- [melior](https://github.com/raviqqe/melior)
