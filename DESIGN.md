# Design notes

Decisions that aren't obvious from the code, and the alternatives we tried first. The language
follows Hirata & Heunen, *Qurts: Automatic Quantum Uncomputation by Affine Types with Lifetime*
([arXiv:2411.10835](https://arxiv.org/abs/2411.10835)). When the code leaves a grammar or typing
question open, Figs. 3–8 of the paper decide it.

## Language

`let` always takes a type: `let x: T = e;` or the borrow form `let y = &'a x;`. The paper has an
untyped `let y = e`; we don't, on purpose, and `parser`'s `reject_let_no_type` test pins that down.
Tuples are bound with `let (a, b) : (T, U) = e;`.

Gate names start with a capital letter. `H(x)` applies a gate, `h(x)` calls a function. Nothing
else in the grammar separates a one-argument call from a gate, and a lexical rule is simpler than
a symbol lookup during lowering.

Which qubits can be dropped comes straight from Fig. 5. Results of `[c](x, ...)` keep their
operands' lifetime, a `qif` result takes its control's lifetime, `[0]()` and `[1]()` are static.
Results of `U(x)` are `#'0`: linear, never droppable. The compiler enforces that split. If an `H`
result is never consumed you get an error, not an invented inverse.

## MLIR

### Lifetimes are SSA tokens, not regions

`qduc.newlft` returns a `!qduc.lt`, `qduc.end` consumes it, and neither is a terminator. We tried
regions twice: once nesting by textual position, once nesting by dependency with `qduc.end`
yielding the values that escape. Both fail for the same reason. TYP END LFT only asks that a
lifetime be minimal in the preorder, so two unrelated lifetimes can have crossing spans
(`newlft 'a; newlft 'b; endlft 'a; endlft 'b;`), and a tree of regions can't express that.

Well-formedness is checked by verifiers instead. `NewLftOp` checks for exactly one end and no use
after it. `qauc.borrow` and `qauc.unique_borrow` check that values derived from the token don't
outlive the end either, since those uses never show up in the token's own use list. A use nested
inside a region counts by where its enclosing op sits in the end's block.

The memory effects are the other half of this. `qduc.end` frees, and the borrows read, the
specific token value rather than the lifetime resource as a whole. Rewriters that look at effects
see the conflict before they reorder anything, and unrelated lifetimes don't block each other.
Passes that ignore effects still hit the verifier.

### Three dialects, split by role

`qduc` is lifetimes. `qauc` is ownership bookkeeping: `borrow`, `release`, `uncompute` are
obligations, not things the program does. `mlrd` is what the program does: `lifted`, `unitary`,
`meas`, `qif`. `mlrd.qif` is not `scf.if`; both branches run, in superposition. It has its own
`mlrd.yield` because `scf.yield` hard-codes the parents it accepts.

Tuples have no MLIR type. `lower` flattens them, since `func.func` and `func.call` already take
several values. This only works for owned tuples; `&'a (T, U)` isn't handled.

`lower` is one recursive walk over the parse tree. `cst_to_qduc`, `cst_to_qauc` and `cst_to_mlrd`
are modules the shared dispatcher calls, and they call back into it for nested blocks. Early on we
talked about it as "pass 1, pass 2, pass 3" and that caused real confusion: because lifetime
tokens are flat, lowering a `newlft` has to hand the statements after it to the same machinery,
so there is no pass boundary to run anything "after".

Values are stored as raw `MlirValue` handles rather than melior's `Value<'c, 'a>`. melior ties a
value's lifetime to the block borrow that produced it, which makes it impossible to carry a value
out of a temporary branch block through a shared environment. The raw handle is a pointer into
MLIR's own heap, so rebuilding a `Value` from it is fine.

## Target and passes

### Why mqt-core

We wanted to share gate-level optimization, export and simulation rather than maintain them, so
the target is mqt-core's `qco` dialect, pulled in as the submodule `third_party/mqt-core`. It fits
well: qubits are SSA values like `!qauc.qbit`, and `qco.ctrl` and `qco.inv` are region-based
modifiers that `qif` and uncomputation map onto directly. Catalyst pins its own LLVM, which would
fight the Homebrew one; QIR as a direct target gives opaque calls, which are a poor surface for
optimization. What stays here is the part that needs Qurts' lifetime structure: drop inference,
uncomputation, lowering `qif`, and optimizations built on those.

Everything is on LLVM/MLIR 23, built with Homebrew clang and an explicit `CMAKE_OSX_SYSROOT`.
mqt-core pins MLIR 23.1 exactly and refuses AppleClang below 17. Without the sysroot flag,
Homebrew clang under CMake searches the SDK's `usr/include` before libc++'s and `<cstring>` won't
compile. mqt-core's thin LTO is off because rustc's linker can't read Homebrew-LLVM bitcode.

### Drops, uncomputation, qif

Drops are inferred as late as the type allows: `qurts-infer-drops` puts the uncompute just before
the `qduc.end` of the value's lifetime. For a `qif` result that's what keeps the control around to
invert it. Dropping early, right after the last use, is the space/time experiment still to run.

Uncomputation is reverse pebbling. `convert-mlrd-to-qco` emits a `qco.inv` whose body is a copy of
the wire's forward history, with controls at their current values. That's sound under the paper's
borrow discipline, which the lowering doesn't check yet. `qurts-controlled-conjugation` relies on
that body being a literal copy: it matches it against the ops at the start of the controlled
region. Change how inverses are emitted and the matcher has to change with it.

Each `qif` branch becomes a `qco.ctrl`, with the else branch wrapped in `qco.x` on the control.
MQT only allows unitaries inside a modifier body, so allocations inside a branch are hoisted out
in front of the control, the two branches' yielded fresh qubits become a single wire, sinks for
qubits dropped in a branch go after the control, and measuring under `qif` is an error. If the
branches yield different outer wires in the same position we reject the program instead of
building a controlled swap. Only qubits allocated inside a branch can be dropped there; dropping
an outer one would mean a controlled uncomputation of history that lives outside the branch.

Equivalence is tested by running the circuits. mqt-core's DD simulator gives a statevector, and
the naive lowering, the conjugated one, and MQT's optimized output have to agree to fidelity 1.
Nested modifiers are unrolled on a copy first because the simulator wants their wires in body
order, and qubits MQT's cleanup deleted are dropped before comparing.

## Known gaps

- `mlrd.qif`'s verifier doesn't check that both branches discharge the same obligations, which
  the paper's QIF rule requires.
- Helpers that allocate qubits break MQT's entry-point contract. Calls that pass qubits aren't
  converted. `&'a (T, U)` and controlled wire permutations aren't supported.
- The gate counts in the playground and the bench count ops, not cost, so conjugation only looks
  like a win on gates that are expensive to control.
