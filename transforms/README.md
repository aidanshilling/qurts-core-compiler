# transforms

The passes that take `lower`'s output (mlrd + qauc + qduc MLIR) to a real gate-level target, and
the first optimization experiment built on top. The target is the `qco` dialect from
[mqt-core](https://github.com/munich-quantum-toolkit/core) (vendored as the git submodule
`third_party/mqt-core`), so everything below the gate level — inverse cancellation, rotation
merging, routing, QIR/OpenQASM export, DD simulation — is MQT's, not ours. What stays here is
exactly what needs Qurts' lifetime/ownership structure: drop inference, uncomputation, and the
`qif` lowering.

## Passes (`td/passes.td`, `cpp/lib/Qurts/`)

| pass | on | what |
| --- | --- | --- |
| `qurts-infer-drops` | `func.func` | Inserts `qauc.uncompute` for every owned `!qauc.qbit` with no consuming use, as late as the paper allows: before the `qduc.end` of the lifetime the value's type carries (a `qif` result inherits its control's lifetime, a lifted result its operands'), else before the block terminator. An unconsumed `mlrd.unitary` result is an error — `U(x)` results are linear (`#'0 qbit`, Fig. 5 TYP UNITARY) and can never be dropped. |
| `convert-mlrd-to-qco` | `builtin.module` | Value-semantics lowering. Lifetime tokens and borrows vanish (a borrowed qubit is the qubit); `mlrd.lifted`/`mlrd.unitary` map to `qco` gates by name (`X`/`NOT`, `H`, `CX`/`CNOT`, `CCX`/`TOFFOLI`, `SWAP`, ...); `mlrd.qif` becomes one `qco.ctrl` region per branch, the else branch sandwiched in `qco.x` on the control, with fresh allocations inside branches hoisted out and the two branches' yielded fresh qubits unified into one; `qauc.uncompute` becomes `qco.inv` over the qubit's gate history (as the paper's reverse-pebbling: invert what was computed, controls at their current values) followed by `qco.sink`. The function with no quantum signature named `main` (or the only such function) gets `mqt.entry_point`. |
| `qurts-controlled-conjugation` | `builtin.module` | The experiment: `ctrl(U V U⁻¹)` → `U ctrl(V) U⁻¹`. When a `qco.ctrl` body ends in a `qco.inv` whose body is a copy of the ops that begin the same wires, both move outside the control. Drop inference makes this pattern structural (the `inv` is a literal clone of the forward chain), so matching is exact, not heuristic. |

Named pipelines usable from Rust (`transforms::{INFER_DROPS, TO_QCO, CONTROLLED_CONJUGATION, MQT_OPTIMIZE}`)
or `qurts-opt`:

```
builtin.module(func.func(qurts-infer-drops))
builtin.module(convert-mlrd-to-qco)
builtin.module(qurts-controlled-conjugation)
builtin.module(mqt-qco-default)          # MQT's own: merge rotations, combine controls, canonicalize
```

## Limits worth knowing

- MQT's contract puts every `qco.alloc` in the entry function's entry block, so helper functions
  that allocate qubits (the paper's `and` returning a fresh `#'a qbit`) are rejected by
  `convert-mlrd-to-qco`; write such code inline in `main` for now. Calls passing qubits are not
  converted yet either.
- Inside a `qif` branch only qubits allocated in that branch (or a nested one) can be dropped;
  dropping an outer owned qubit there would mean a controlled uncomputation of history that lives
  outside the branch.
- Both branches must yield the same outer wire (or both a fresh one) at each position — a
  controlled permutation of wires is not supported.
- Uncomputation inverts the wire's history with its controls at their *current* values. That is
  sound under the paper's typing (controls are borrowed for the whole lifetime), which the
  lowering does not itself enforce yet; a multi-qubit `U(a, b)` on two owned qubits followed by a
  drop of one of them is the way to violate it.

## Build

Needs Homebrew LLVM 23 and Homebrew clang (mqt-core refuses AppleClang < 17), plus the submodule:

```bash
git submodule update --init
export SDKROOT=$(xcrun --show-sdk-path)
cmake -S transforms/cpp -B transforms/cpp/build -G Ninja -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_C_COMPILER=$(brew --prefix llvm)/bin/clang \
  -DCMAKE_CXX_COMPILER=$(brew --prefix llvm)/bin/clang++ \
  -DCMAKE_OSX_SYSROOT=$SDKROOT \
  -DMLIR_DIR=$(brew --prefix llvm)/lib/cmake/mlir
cmake --build transforms/cpp/build                      # builds mqt-core's qco/mqt libs + our passes + qurts-opt
cmake --build transforms/cpp/build --target check-transforms   # lit tests in test/
cargo test -p transforms                                # pipeline + DD-simulation equivalence tests
cargo run -p transforms --example bench                 # naive vs conjugated gate counts per example script
```

The build pulls `mlrd/cpp` in as a subdirectory (so `qduc`/`qauc`/`mlrd` are compiled again
here, same pattern as `mlrd/cpp` does for `qauc`) and `third_party/mqt-core` with
`EXCLUDE_FROM_ALL`, so only the mqt-core libraries we link get built. `build.rs` links every
`.a` under `cpp/build/mqt-core-build`.

## Equivalence checking

`transforms::statevector(module, "main")` runs mqt-core's decision-diagram simulator on the
entry function and `fidelity(a, b)` compares two results; `tests/pipeline.rs` uses this to check
that the naive lowering, the conjugated lowering, and MQT's optimized output all agree.
