# Design decisions

The "why" that is not recoverable from the code or the git history. Each entry names the
alternative that was rejected, so it doesn't get re-proposed. The language itself follows
Hirata & Heunen, *Qurts: Automatic Quantum Uncomputation by Affine Types with Lifetime*
([arXiv:2411.10835](https://arxiv.org/abs/2411.10835)); Figs. 3–8 are the source of truth for
grammar and typing whenever the code leaves a question open.

## Language surface

- **Typed `let` only.** `let x: T = e;` and `let y = &'a x;` exist; `let y = e;` does not. A
  deliberate divergence from the paper's grammar for this version, locked in by
  `parser`'s `reject_let_no_type`. `let (a, b) : (T, U) = e;` is the tuple form.
- **Gate names start uppercase.** `H(x)` is a unitary application, `h(x)` is a call. The grammar
  otherwise can't tell a one-argument call from a gate; a lexical rule beats a symbol lookup in
  the lowering.
- **`[c](x, ...)` is the only droppable gate form.** Per Fig. 5, lifted results keep their
  operands' lifetime (affine), `qif` results take the *control's* lifetime, allocations are
  static, and `U(x)` results are `#'0` — linear, never droppable. The compiler enforces that
  split (an unconsumed `H` result is an error, not an uncomputation) rather than inventing an
  inverse for arbitrary unitaries.

## MLIR representation

- **Lifetimes are flat `!qduc.lt` SSA tokens, not regions.** `qduc.newlft` produces a token,
  `qduc.end` consumes it, neither is a terminator. Regions were tried twice (textual nesting;
  nesting by dependency with `qduc.end` yielding escaping values) and rejected: TYP END LFT only
  requires a lifetime to be *minimal* in the preorder, so two unrelated lifetimes may have
  crossing spans, which a tree of regions cannot express. Well-formedness is verifier-enforced
  instead (`NewLftOp`: exactly one end, no later use; `qauc.borrow`/`unique_borrow`: derived
  values don't outlive the end either). Uses nested in regions count by their ancestor op's
  position in the end's block.
- **Value-scoped memory effects are the "before" half of that enforcement.** `qduc.end` frees
  and borrows read the *specific* token value, not the resource kind, so effect-aware
  rewriters see the conflict before reordering without making every lifetime conflict with
  every other. The verifier remains the backstop for passes that don't consult effects.
- **Three dialects by role, not by phase.** `qduc` = lifetimes, `qauc` = ownership obligations
  (`borrow`, `release`, `uncompute` — bookkeeping, not program ops), `mlrd` = what the program
  does (`lifted`, `unitary`, `meas`, `qif`). `mlrd.qif` is *not* `scf.if`: both branches
  contribute unitarily. It has its own `mlrd.yield` because `scf.yield` hard-codes its allowed
  parents.
- **Tuples have no MLIR type.** `lower` flattens them; `func.func`/`func.call` already carry
  multiple values. Owned tuples only — `&'a (T, U)` is unhandled.
- **`lower` is one CST walk, not chained MLIR passes.** `cst_to_qduc`/`cst_to_qauc`/
  `cst_to_mlrd` are modules the shared dispatcher calls into and that call back into it.
  "Pass 1 → pass 2 → pass 3" framing was a real source of confusion early on: qduc tokens are
  flat, so a `newlft` has to hand the *following* statements to the same machinery.
- **Values are stored by raw `MlirValue`, not melior's borrow-checked `Value<'c,'a>`.** melior
  ties a value to the block borrow that produced it, which makes carrying values out of a
  temporary branch block impossible; the raw handle is a non-owning pointer into MLIR's own
  heap, so reconstructing it is sound.

## Target and transforms

- **Target is mqt-core's `qco` dialect, as a submodule, not a home-grown gate dialect.** The
  aim is to share gate-level optimization, export and simulation rather than maintain them.
  `qco` was the fit: SSA qubits like `!qauc.qbit`, and region-based `qco.ctrl`/`qco.inv`
  modifiers that `qif` and uncomputation map onto directly. Catalyst was rejected for pinning
  its own LLVM; QIR as a direct target because opaque calls are a poor surface for
  optimization. What stays in this repo is exactly what needs Qurts' lifetime structure: drop
  inference, uncomputation, `qif` lowering, and optimizations that exploit them.
- **Everything is on LLVM/MLIR 23, built with Homebrew clang and an explicit sysroot.** mqt-core
  pins MLIR 23.1 exactly and refuses AppleClang < 17; without `CMAKE_OSX_SYSROOT`, Homebrew
  clang under CMake puts the SDK's `usr/include` ahead of libc++'s. mqt-core's thin-LTO is
  turned off because rustc's system linker can't read Homebrew-LLVM bitcode.
- **Drops are inferred as late as the type allows.** `qurts-infer-drops` places an uncompute
  before the `qduc.end` of the value's lifetime, which is what keeps a `qif` result's control
  available to invert it. Early placement (right after last use) is the open space/time
  experiment, not the default.
- **Uncomputation is reverse pebbling, emitted as a literal `qco.inv` of the wire's history**
  with controls at their *current* values — sound under the paper's borrow discipline, which the
  lowering does not itself verify yet. `qurts-controlled-conjugation` matches that `inv` body
  structurally against the start of the controlled region; changing how inverses are emitted
  means changing the matcher.
- **`qif` becomes one `qco.ctrl` per branch, shaped by MQT's modifier rules.** Bodies may hold
  only unitaries, so allocations inside a branch are hoisted before the control (both branches'
  yielded fresh qubits become *one* wire), sinks of branch-dropped qubits follow it, and
  measurement under `qif` is an error. Branches yielding different outer wires at the same
  position are rejected rather than turned into a controlled permutation. Only qubits allocated
  within a branch may be dropped there; dropping an outer qubit would mean controlled
  uncomputation of history outside the branch.
- **Equivalence is checked by simulation, not by inspection.** mqt-core's DD simulator gives a
  statevector; naive, conjugated and MQT-optimized lowerings must agree to fidelity 1. Nested
  modifiers are unrolled on a copy first (the simulator needs their wires in body order), and
  qubits MQT's cleanup deleted are discarded before comparing.

## Open gaps worth knowing

- `mlrd.qif`'s verifier doesn't check that both branches discharge the same obligations (the
  paper's QIF rule requires it).
- Helpers that allocate qubits violate MQT's entry-point contract; calls passing qubits aren't
  converted; `&'a (T, U)` and controlled wire permutations are unsupported.
- Gate counts in the playground/bench count ops, not cost, so the conjugation win only shows
  on gates that are expensive under control.
