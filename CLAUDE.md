# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Compiler for qurts, a statically-typed quantum programming language with a lifetime/ownership
system (`&'a T` classical borrow, `#'a T` unique/linear ownership) whose purpose is to make
correct quantum *uncomputation* statically verifiable — an owned `#'a qbit` must be returned to
`|0>` before its lifetime ends, and the compiler enforces this rather than trusting the
programmer.

The language design follows a specific paper: **Hirata & Heunen, "Qurts: Automatic Quantum
Uncomputation by Affine Types with Lifetime"** (arXiv:2411.10835). Figs. 3-8 (pages 13-16) give
the SSA grammar, type system, and typing judgements. When working on the parser grammar, the type
system, or dialect semantics, treat that paper as the source of truth over guessing — if a
grammar/typing question comes up that isn't settled by the code, check the paper before inventing
an answer.

## Workspace layout

Seven crates, `parser` -> `lower` -> qduc/qauc/mlrd MLIR -> `transforms` -> MQT `qco` dialect,
plus `playground` (a UI on the side, not part of the compiler pipeline itself).

```
qurts source
    |
    v  parser (pest grammar)
parse tree (CST)
    |
    v  lower: ONE CST-driven dispatcher, extended over time (NOT sequential MLIR->MLIR passes)
qduc + qauc + mlrd MLIR dialects
    |
    v  transforms: qurts-infer-drops, convert-mlrd-to-qco      (real MLIR passes, C++)
qco (mqt-core's value-semantics gate dialect, vendored as a git submodule)
    |
    v  transforms: qurts-controlled-conjugation; then anything MQT ships (mqt-qco-default,
    |              unroll-modifiers, OpenQASM/QIR export, DD simulation, ...)
```

The split is deliberate: everything that needs Qurts' lifetime/ownership structure (drop
inference, uncomputation, lowering `qif`) lives here; everything below the gate level (inverse
cancellation, rotation merging, routing, export, simulation) is MQT's and is not re-implemented.

**`lower`'s architecture is one recursive-descent CST walk, not three chained passes.** Early
framing here (and the earlier planning conversation) described "pass 1 -> pass 2 -> pass 3" as if
pass 2 transforms pass 1's *output* — that's wrong and was a real point of confusion: `newlft_stmt`'s
lowering still needs to hand off to the *same* statement/expression lowering machinery for whatever
follows it in the flat statement sequence (qduc lifetimes are flat tokens, not nested regions — see
the qduc design note below), so "run pass 2 after pass 1 finishes" doesn't work — pass 2's job is
*interleaved with* what pass 1 does, not sequential after it. The actual shape: `block::lower_stmt` /
`expr::lower_expr` (top-level modules in `lower/src/`) are the shared dispatcher; `cst_to_qduc`/
`cst_to_qauc`/`cst_to_mlrd` (modules, not passes) are *called into* from that dispatcher when it
hits a construct they cover (`newlft_stmt`/`endlft_stmt` -> `cst_to_qduc`; `borrow_stmt`/
`drop_stmt` -> `cst_to_qauc`; `lifted_expr`/`unitary_expr`/`meas_expr`/`qif_expr` ->
`cst_to_mlrd`), and they recurse back into the same shared
`lower_block_body`/`lower_expr` for whatever's nested inside (e.g. `lower_qif`'s then/else regions
call back into `lower_block_body`, same shape as `lower_if`'s classical counterpart).
`LowerError::Unsupported*` doesn't disappear in one step when a new module lands — its scope just
shrinks as more match arms get wired up. There is no runtime-visible "pass 1 vs 2 vs 3" boundary;
only currently-covered vs. not-yet-covered grammar.

- **`parser`** - pest PEG grammar (`parser/src/qurts.pest`) + parse-tree tests. No system deps.
  `parser/examples/scripts/*.qurts` are hand-written example programs covering the currently-
  parseable grammar; `format_parse_tree()` + `cargo run -p parser --example print_ast -- <file>`
  visualize a file's CST.
- **`qduc`** - MLIR dialect for the lifetime/region system. Target-agnostic: knows nothing about
  quantum gates. Provides `!qduc.lt` (SSA lifetime token) and the ops that open/close scopes.
- **`qauc`** - MLIR dialect for quantum sub-structural ownership. Bridges `qduc` lifetimes with
  qubit resource tracking (`!qauc.qbit`, `!qauc.unique<lt,T>`, `!qauc.ref<lt,T>`, and the
  borrow/release/uncompute ops — pure obligation bookkeeping, not program operations). Depends on
  `qduc` (path dependency) both in Rust and in its `.td` file (`qauc.td` does
  `include "qduc/td/qduc.td"` and reuses `qduc::LifetimeType`).
- **`mlrd`** - MLIR dialect for the operations a qurts program actually performs (as opposed to
  qduc's lifetime tokens or qauc's ownership bookkeeping): `mlrd.lifted` (opaque lifted-gate
  application, moved here from `qauc` since it's a program op, not an obligation) and `mlrd.qif`
  (coherent quantum conditional — NOT `scf.if`; both regions unitarily contribute, neither is
  "not executed", so `scf.if`'s branch semantics would misrepresent it to later passes). Depends
  on `qauc` (path dependency, transitively `qduc`) for `!qauc.ref`/`!qauc.qbit` in `mlrd.qif`'s
  condition type. Its branch regions terminate in `mlrd.yield`, not `scf.yield` — `scf.yield`'s
  own verifier hardcodes an allowed-parent list (`scf.if`/`scf.for`/...) that doesn't include
  `mlrd.qif`, so reusing it doesn't work (found via the Rust dialect tests, not anticipated up
  front — don't re-attempt reusing an upstream dialect's terminator op without checking its
  verifier's parent allowlist first). `mlrd.qif`'s verifier checks the condition is qubit-valued
  and both regions are a single block ending in `mlrd.yield` with matching result types — it does
  **not** yet check that both regions discharge the same lifetime/uniqueness obligations (the
  paper's QIF typing rule requires this; it's a real gap, see `mlrd/README.md`, not silently
  assumed safe).
- **`lower`** - CST -> MLIR lowering (melior-based, no new FFI). `lib.rs` (crate root) holds the
  shared `Lowerer`/`LoweredProgram`/`lower_program`/`default_context` — the entry point and the
  types every dialect-domain module needs, so it can't live inside just one of them. The classical
  dispatcher (`block`/`env`/`error`/`expr`/`function`/`signature`/`ty`/`value`) sits as flat
  top-level modules right next to `lib.rs`, not nested under their own module — "top level" was a
  deliberate choice so the resolver functions (`lower_stmt`, `lower_expr`, `lower_function`,
  `lower_ty`, ...) read as the crate's core, with per-dialect construction logic split out
  separately. `cst_to_qauc` holds qauc-specific construction the dispatcher calls into:
  `ty::lower_ty`'s `qbit_ty`/`ref_ty`/`unique_ty` cases delegate to `cst_to_qauc::{qbit_type,
  ref_type, unique_type}` (which `Type::parse` `!qauc.qbit`/`!qauc.ref<...>`/`!qauc.unique<...>`),
  and `cst_to_qauc::borrow` builds the actual `qauc.borrow` op, called from `block::lower_stmt`'s
  `borrow_stmt` arm — looks up the lifetime token (`Env::lifetime`, a non-consuming read, unlike
  `endlft`'s `close_lifetime`) and the source value (`Env::lookup`, must be `.as_single()`; a tuple
  source is `LowerError::UnsupportedStmt` — borrowing a tuple isn't handled, same flattening gap
  noted in the `StoredValue`/tuple design note below). `cst_to_qduc` holds `qduc.newlft`/`qduc.end`
  construction (`lifetime_type`/`newlft`/`end`), called into from `block::lower_stmt`'s
  `newlft_stmt`/`endlft_stmt` arms. `cst_to_mlrd` holds `mlrd.lifted`/`mlrd.qif`/`mlrd.yield`
  construction, called from `expr::lower_expr`'s `lifted_expr`/`qif_expr` arms (`lower_lifted`/
  `lower_qif`): `lower_qif` builds `mlrd.qif`'s two regions exactly like `lower_if` builds
  `scf.if`'s (push a scope, recurse into `lower_block_body`, flatten the result, terminate — just
  with `cst_to_mlrd::r#yield` instead of `scf::r#yield`, and the op's result types are still
  derived from the *then* branch only, so a mismatched *else* branch surfaces as a verifier
  failure on that branch's `mlrd.yield`, not a Rust-level check — same "let the verifier catch it"
  choice `lower_if` already makes for `scf.if`). `Env` (`env.rs`) grew a second, *unscoped* table alongside its `{}`-scoped
  value `scopes`: `lifetimes: HashMap<String, StoredValue>`, name -> the token its `qduc.newlft`
  produced. It's flat/unscoped on purpose, matching qduc's own flat-token design (see the design
  note below) — a lifetime's visibility is governed by `newlft_stmt`/`endlft_stmt` alone, not by
  surrounding `{}` block structure, so it isn't pushed/popped alongside `scopes`.
  `lower_program` skips any function hitting a not-yet-covered construct and records why in
  `LoweredProgram::errors`, rather than failing the whole program — partial-program lowering by
  design. See `lower/tests/example_scripts.rs` for the exact per-function expected outcome against
  each bundled example script.
  `default_context()` is the one shared place that registers `func`/`arith`/`scf` +
  `qduc`/`qauc`/`mlrd` in a `Context` — reused by `lower`'s own tests and by `playground`; add a
  new consumer to this, don't re-derive the dialect-registration dance again.
- **`transforms`** - the MLIR passes after `lower`, in C++ (`transforms/cpp/lib/Qurts/`,
  tablegen'd in `transforms/td/passes.td`) with a thin Rust wrapper (`transforms/src/lib.rs`:
  `context()`, `run_pipeline()`, `simulate()`, `gate_summary()`). Three passes:
  `qurts-infer-drops` (the paper's drop inference: `qauc.uncompute` for every owned qubit never
  consumed, placed before the `qduc.end` of the lifetime its type carries; an unconsumed
  `mlrd.unitary` result is an *error* because `U(x)` results are linear `#'0 qbit`),
  `convert-mlrd-to-qco` (value-semantics lowering to mqt-core's `qco` dialect: `mlrd.qif` ->
  one `qco.ctrl` region per branch, else branch sandwiched in `qco.x` on the control, fresh
  allocs inside branches hoisted and the two branches' yielded fresh qubits unified;
  `qauc.uncompute` -> `qco.inv` over the wire's gate history + `qco.sink`), and
  `qurts-controlled-conjugation` (the first optimization experiment: `ctrl(U V U⁻¹)` ->
  `U ctrl(V) U⁻¹`, matched structurally because drop inference makes the `inv` a literal clone of
  the forward chain). mqt-core is the git submodule `third_party/mqt-core`, built *inside*
  `transforms/cpp/build` with `EXCLUDE_FROM_ALL`; `build.rs` links every `.a` it produces. Also
  `tools/qurts-opt` + `test/*.mlir` lit tests (`check-transforms`) and `tests/pipeline.rs`, which
  checks naive vs optimized lowerings agree via mqt-core's DD simulator (`statevector`/`simulate`,
  `fidelity` discards qubits MQT's cleanup removed). See `transforms/README.md` for the limits
  (allocs only in `main` per MQT's contract, no qubit-passing calls, only branch-local qubits may
  be dropped inside a `qif` branch, no controlled wire permutations).
- **`playground`** - local web UI (axum backend in `src/`, Vite+React+TS frontend in
  `frontend/`) visualizing qurts source across every stage: CST, plain MLIR, drop inference, qco
  (naive), MQT-optimized naive, controlled-conjugation, MQT-optimized conjugated, and a gate-count
  summary. Backend returns a JSON array of named stages (`POST /api/compile`); the frontend
  renders whatever comes back generically, so a new stage is one more entry in
  `playground/src/compile.rs`, no frontend change. `npm run build` in `frontend/` before
  `cargo run -p playground` (serves `frontend/dist/` as static files) — see
  `playground/README.md` for the dev-mode (hot reload) workflow.

Gate names in source: `U(x, ...)` with an uppercase-initial gate name (`H`, `X`, `CX`) — that is
how `H(x)` is told apart from a call `h(x)`; `[c](x, ...)` lifted gates take any number of qubits
and return one result per operand (`[0]()`/`[1]()` allocate); both are resolved by name in
`convert-mlrd-to-qco` (`X`/`NOT`, `CX`/`CNOT`, `CCX`/`TOFFOLI`, `SWAP`, `CSWAP`, `H`, `S`,
`T`, ...). `let (a, b) : (qbit, qbit) = [cnot](a, b);` binds tuple results; `drop x;` is an
explicit drop (`qauc.uncompute` for owned qubits, nothing for refs/bools).

## Each dialect crate's structure

`qduc/`, `qauc/`, and `mlrd/` all follow the same three-layer pattern. `mlrd` doesn't yet have the
`tools/<dialect>-opt` + `test/` lit/FileCheck layer `qduc`/`qauc` have (below) — its op verifier
(`mlrd.qif`) is currently only exercised by Rust tests in `mlrd/src/dialect.rs`, same style as
`qduc`/`qauc`'s tests before the lit suite was added.

```
<dialect>/td/<dialect>.td       # tablegen: dialect, types, ops, attrs
<dialect>/cpp/                  # hand-written C++ dialect impl, built via CMake + mlir-tblgen
  CMakeLists.txt                #   generates .inc files from ../td/<dialect>.td, builds a static lib,
                                 #   builds tools/<dialect>-opt, wires up the check-<dialect> lit target
  include/<Dialect>/<Dialect>Dialect.h
  lib/<Dialect>/<Dialect>.cpp   #   dialect initialize(), op interface impls, the qauc_register_dialect
                                 #   / qduc_register_dialect extern "C" FFI entry point
  tools/<dialect>-opt/<dialect>-opt.cpp  # mlir-opt-alike (MlirOptMain) registering just this
                                 #   dialect's needs — see "lit/FileCheck verifier tests" below
<dialect>/test/                 # lit/FileCheck tests exercising hasVerifier=1 ops, run against
  lit.cfg.py, lit.site.cfg.py.in #  <dialect>-opt; NOT under <dialect>/cpp — cpp/ only holds the
  verifier/*.mlir                #  tool that runs them, same split as td/ vs cpp/ vs src/
<dialect>/src/                  # Rust side (melior-based)
  dialect.rs                   #   calls the extern "C" register fn via mlir-sys
  types.rs, ops.rs              #   currently empty stubs — no ergonomic Rust wrappers yet,
                                 #   only the raw C API via melior's generic Type/Operation
  build.rs                      #   links against the CMake-built static lib (cpp/build/*.a)
```

`qauc`'s CMake build additionally pulls `qduc/cpp` in as an `add_subdirectory` (built into
`qauc/cpp/build/qduc-build`), since `qauc.td` includes `qduc.td`. `mlrd`'s CMake build does the
same one level further out: it pulls `qauc/cpp` in as an `add_subdirectory` (built into
`mlrd/cpp/build/qauc-build`), which itself pulls `qduc/cpp` in as `mlrd/cpp/build/qauc-build/qduc-build`
— three nested copies of `QducDialect` end up compiled across the three dialects' build trees
rather than any of them depending on a standalone sibling build — see `qauc/cpp/README.md` and
`mlrd/cpp/README.md`.

Gotcha specific to this tablegen setup: `-gen-op-decls`/`-gen-op-defs` have **no dialect filter**
flag (unlike `-typedefs-dialect=...`), so generating `qauc`'s ops without
`-op-include-regex "qauc\\..*"` would also re-emit `qduc`'s ops (pulled in transitively via the
`include`), causing duplicate-definition errors when both `.inc` files are included together.
Same story for `mlrd`'s `-op-include-regex "mlrd\\..*"` — without it, generating `mlrd`'s ops
would also re-emit both `qauc`'s and `qduc`'s ops, pulled in transitively via `mlrd.td`'s
`include "qauc/td/qauc.td"`.

## Build & test commands

Everything is on **LLVM/MLIR 23.1** (`brew install llvm` = 23.1.x; Rust side `melior 0.28` /
`mlir-sys 230`, which locate it via `llvm-config` on `PATH` or `MLIR_SYS_230_PREFIX`). mqt-core
pins MLIR 23.1 exactly and refuses AppleClang < 17, so all C++ is built with Homebrew clang and an
explicit sysroot (without `CMAKE_OSX_SYSROOT`, Homebrew clang + CMake put the SDK's `usr/include`
ahead of libc++'s and `<cstring>` fails to compile):

```bash
export SDKROOT=$(xcrun --show-sdk-path)
CM="-G Ninja -DCMAKE_C_COMPILER=$(brew --prefix llvm)/bin/clang -DCMAKE_CXX_COMPILER=$(brew --prefix llvm)/bin/clang++ -DCMAKE_OSX_SYSROOT=$SDKROOT -DMLIR_DIR=$(brew --prefix llvm)/lib/cmake/mlir"
```

**C++ builds (required once, and after any `.td`/`.cpp` change, before `cargo build`/`cargo
test` will link):**

```bash
cmake -S qduc/cpp -B qduc/cpp/build $CM && cmake --build qduc/cpp/build
cmake -S qauc/cpp -B qauc/cpp/build $CM && cmake --build qauc/cpp/build
cmake -S mlrd/cpp -B mlrd/cpp/build $CM && cmake --build mlrd/cpp/build
git submodule update --init                       # third_party/mqt-core
cmake -S transforms/cpp -B transforms/cpp/build $CM -DCMAKE_BUILD_TYPE=Release
cmake --build transforms/cpp/build                # mqt-core's qco/mqt libs + our passes + qurts-opt (slow first time)
```

`cargo build` does **not** trigger any of this — each `build.rs` only links against whatever
static libs already exist in `<crate>/cpp/build/`. A `.td` change in `qduc`/`qauc` has to be
rebuilt in every tree that compiles a copy of it (`qauc/cpp/build`, `mlrd/cpp/build`,
`transforms/cpp/build/mlrd-build/...`).

**lit/FileCheck verifier tests** (`<dialect>/test/verifier/*.mlir`, run against `<dialect>-opt`
built above — a second, .mlir-text-level verifier test surface alongside the Rust ones in
`<dialect>/src/dialect.rs`'s `#[cfg(test)]`, not a replacement for them):

```bash
cmake --build qduc/cpp/build --target check-qduc
cmake --build qauc/cpp/build --target check-qauc   # also reaches check-qduc — qduc/cpp is a subdir here
cmake --build transforms/cpp/build --target check-transforms   # pass tests against qurts-opt
```

Needs `FileCheck` (ships with `brew install llvm`, at `$(brew --prefix llvm)/bin`, just not
symlinked onto `PATH` by default — add it or invoke by full path) and `lit` (`pipx install lit`)
findable at CMake-configure time; the `check-<dialect>` target is silently skipped (with a
`message(STATUS ...)`) if either is missing, so a `.td`/verifier-only workflow isn't blocked on
having them installed.

**Rust build/test (after the above):**

```bash
cargo build --workspace
cargo test --workspace
cargo test -p parser <test_name>       # e.g. cargo test -p parser fn_with_params
cargo test -p qauc <test_name>         # e.g. cargo test -p qauc dialect_registers_ops
cargo test -p lower --test example_scripts -- --nocapture   # prints lowered MLIR per script
cargo test -p transforms                                    # pipeline + DD-simulation equivalence
cargo run -p transforms --example bench                     # naive vs conjugated gate counts per script
cargo clippy --workspace
```

`parser` has no C++ dependency and builds/tests standalone with plain `cargo test -p parser`.
`lower` needs `qduc`/`qauc`/`mlrd` C++ builds all done (it registers all three dialects, see
above); `transforms` and `playground` need `transforms/cpp/build` too.

**`playground` (web UI, separate npm subproject in `playground/frontend/`):**

```bash
cd playground/frontend && npm install && npm run build && cd ../..
cargo run -p playground   # serves the built frontend + API at the printed URL
```

`cargo build --workspace` builds `playground`'s Rust side regardless, but the UI itself needs the
npm build done at least once (`frontend/dist/` is what gets served). Not part of the compiler
pipeline — see `playground/README.md`.

## Design decisions worth knowing before "fixing" something

- **Untyped `let` is intentionally rejected.** `parser/src/qurts.pest` only has `let x: T = e;`
  (typed) and `let y = &'a x;` (borrow) — there is no `let y = e;` form, and
  `parser/src/lib.rs`'s `reject_let_no_type` test locks this in deliberately. This is a real,
  acknowledged divergence from the paper's grammar (which has `let y = e` as its own form) — do
  not "fix" it without being asked; it's a deliberate scope decision for this version of the
  language.
- **Lifetimes are flat `!qduc.lt` SSA tokens (`qduc.newlft`/`qduc.end`), not a nested region.**
  An earlier design modeled a lifetime scope as a structurally-nested MLIR region (`qduc.region`,
  token as a block argument that dominance rules kept from escaping). That was rejected: the
  paper's `TYP END LFT` rule only requires a lifetime to be minimal in the ordering preorder, not
  most-recently-opened, so two lifetimes with no ordering constraint between them can have
  crossing, non-nested `newlft`/`endlft` spans (`newlft 'a; newlft 'b; endlft 'a; endlft 'b;`) —
  MLIR regions are inherently tree-structured and can't represent that. A region-based redesign
  (nesting by dependency instead of textual position, with `qduc.end` yielding escaping values)
  was also explored and rejected — don't re-litigate either regions-based approach.
  `qduc.newlft` now just produces `!qduc.lt` as an ordinary op result; `qduc.end` consumes it and
  is not a terminator. Well-formedness is enforced by verifiers instead of region/dominance
  structure, in two layers: `qduc`'s `NewLftOp::verify()` checks the raw token (exactly one
  `qduc.end`, no other use after it); `qauc`'s `BorrowOp`/`UniqueBorrowOp::verify()` layer on top,
  checking that *derived* values (e.g. `unique_borrow`'s result) don't outlive `$lifetime`'s
  `qduc.end` either — a use `qduc`'s check can't see, since it never appears in the token's own
  use-list. Shared, non-diagnosing helpers `qduc::findUniqueEnd`/`qduc::allUsesPrecede` (in
  `qduc/cpp/lib/Qduc/Qduc.cpp`) avoid duplicating the ordering-check logic and double-reporting the
  same root cause from two ops. This holds up against pass reordering specifically because MLIR's
  `PassManager` verifies after every pass by default (`enableVerifier`) — a pass that violates
  token ordering fails the pipeline immediately after that pass, not silently.
- **The verifier catches token-ordering violations after a pass runs; value-scoped memory effects
  are the complementary "before" half.** `qduc::EndOp::getEffects` declares `Free` scoped to the
  specific `%lt` SSA value (an `OpOperand*`/`OpResult`/`BlockArgument`, not a bare `Value` —
  `mlir::SideEffects::EffectInstance`'s value constructor requires one of those three; use
  `&getOperation()->getOpOperand(i)` for an operand slot). `qauc::BorrowOp`/`UniqueBorrowOp`
  declare a matching value-scoped `Read` on `$lifetime`. Scoping to the specific value (not just
  `Qduc_LifetimeResource`, the resource *kind*) matters: resource-level scoping would make every
  lifetime in a function conflict with every other lifetime for scheduling purposes, blocking
  legitimate reordering between two genuinely unrelated lifetimes — the same over-conservatism
  problem verifier-only enforcement has, just moved to a different axis. This lets effect-aware
  MLIR infrastructure (CSE, LICM, the greedy rewriter) see the conflict *before* transforming and
  simply not attempt the illegal reorder, rather than attempting it and finding out from the
  verifier afterward. It's advisory, not a replacement for the verifier — a pass that doesn't
  consult `getEffects()` can still produce violating IR, so the verifier stays as the hard
  backstop regardless.
- **Drop semantics follow Fig. 5 of the paper literally, and the passes rely on it.** Only
  `[c](x)` lifted results (`#'a`, same lifetime as the operands), `qif` results (`#'α`, the
  *control's* lifetime) and `[0]()`/`[1]()` allocations (static) are droppable; `U(x)` results
  are `#'0` linear and must be measured/returned/consumed — `qurts-infer-drops` errors on them
  instead of inventing an uncomputation. A drop is placed before the `qduc.end` of its lifetime,
  which is what makes the control still available to invert a `qif` result. Uncomputation is the
  paper's reverse pebbling: `qco.inv` over the wire's forward gate history with controls at
  their *current* values — sound under the paper's borrow discipline (controls stay borrowed),
  which the lowering does not itself verify yet. `qurts-controlled-conjugation` depends on the
  `inv` body being a literal clone of that history; don't "optimize" the uncompute emitter into
  something that produces different-looking inverses without updating the matcher.
- **`qif` branches become `qco.ctrl` regions, and MQT's modifier-body rules shape the lowering.**
  A `qco.ctrl` body may only hold unitaries and effect-free classical ops — no `qco.alloc`, no
  `qco.sink`, no `qco.measure`. So allocations inside a branch are hoisted before the outermost
  control (both branches' yielded fresh qubits are *one* hoisted qubit; `[1]()` inside a branch is
  a controlled `qco.x`), sinks of branch-dropped qubits are emitted after it, and measurement
  inside `qif` is an error. Because the paper's QIF rule hands the whole context to both branches,
  "which outer wire does a yielded value denote" is decided by `root()` (wire identity through
  gate operand/result positions, borrows, and unified qif results) — the lowering rejects
  branches that yield different outer wires at the same position rather than guessing.
- **MQT's DD simulator can't take nested modifiers whose wires aren't in body order**, so
  `transforms::simulate` runs `unroll-modifiers,canonicalize` on a copy first. Compare circuits
  with `fidelity`, which also tolerates MQT's cleanup deleting an allocated-but-unused qubit.
- **`AI assistance` note in the root README**: parts of this project are developed with Claude's
  assistance per the University of Edinburgh's generative AI policy; all changes are expected to
  be reviewed by the project author, not auto-applied blind.
- **Tuples have no MLIR type/op of their own.** `lower` flattens them entirely at lowering time
  (`value::LoweredValue::Single`/`Tuple`, `value::Shape` for un-flattening call results) — `(x0,
  x1)`/tuple destructuring emit zero ops, and crossing a real boundary (`func.func` params/return,
  `func.call`) spreads a tuple across N real SSA values, since `func.func`/`func.call` natively
  support multiple results already. Only works for *owned* tuples; `&'a (T1,T2)` can't be
  flattened this way and isn't handled yet.
- **`lower`'s `Env`/`LoweredValue` store values by raw handle (`value::StoredValue`), not as
  melior's borrow-checked `Value<'c,'a>`.** melior ties a `Value`'s second lifetime to the exact
  `Block` borrow that produced it, which makes it impossible to carry a value computed inside a
  temporary branch block (e.g. an `if`'s then/else region) back out through a shared, single-
  lifetime environment — the borrow checker can't unify the branch's short-lived block with the
  function's longer-lived one. `StoredValue` sidesteps this by keeping just the raw `MlirValue`
  pointer (`ValueLike::to_raw`) and reconstructing `Value<'c,'c>` on demand (`Value::from_raw`,
  unsafe) — sound because `MlirValue` is a non-owning handle into MLIR's own heap-allocated IR
  tree, not a real Rust borrow. Reuse this pattern rather than re-deriving it if pass 2/3 hit the
  same issue (they will, for anything crossing a nested MLIR region boundary, e.g. a future `qauc`
  construct like a `qif` lowered via `scf.if`-shaped regions — `qduc` lifetimes themselves are flat
  tokens now, not region-scoped, so this no longer applies to them specifically).

## Working conventions for Claude Code in this repo

- **Comments**: default to zero. Only add one when something is genuinely non-obvious from the
  code itself (a hidden constraint, a subtle invariant, a workaround) — never to restate what's
  already visible on the line (e.g. don't explain "else is optional" when the `?` is right there).
  When a comment is warranted, one line, as few words as possible.
- **Explaining changes**: before/alongside making a substantive edit, explain what's changing —
  concise, not exhaustive. Reference specific line numbers where useful instead of restating the
  whole diff, then close with a short bullet-point summary. Skip the ceremony for trivial one-line
  fixes where the diff speaks for itself.
