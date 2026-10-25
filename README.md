# qurts-core-compiler

Compiler for the qurts quantum programming language. qurts is a statically-typed language with
a lifetime and ownership system designed to enforce correct quantum uncomputation — the requirement
that ancilla qubits are returned to their original state before being released.

Link to original Qurts paper - https://arxiv.org/pdf/2411.10835

## Language overview

qurts has two reference types built around lifetimes:

- `&'a T` — classical shared reference, borrows a value for lifetime `'a`
- `#'a T` — unique (linear) reference, exclusively owns a resource for lifetime `'a`

The unique reference type is the core mechanism for quantum resource tracking: a `#'a qbit` must
be consumed exactly once, and when lifetime `'a` ends the compiler verifies that the qbit has
been uncomputed.

Lifetimes can carry ordering constraints, expressed in function signatures:

```
fn foo<'a, 'b, 'a <= 'b>(x: #'a qbit, y: &'b bool) -> ()
```

## Workspace crates

| Crate        | Description |
|--------------|-------------|
| `parser`     | PEG grammar and parse tree via pest |
| `qduc`       | MLIR dialect for the lifetime/region constraint system |
| `qauc`       | MLIR dialect for quantum sub-structural ownership types |
| `mlrd`       | MLIR dialect for the operations a program performs: lifted/unitary gates, `qif`, `meas` |
| `lower`      | parse tree → qduc/qauc/mlrd MLIR |
| `transforms` | drop inference, lowering to [mqt-core](https://github.com/munich-quantum-toolkit/core)'s `qco` gate dialect, and the controlled-conjugation optimization; DD-simulation equivalence checks |
| `playground` | local web UI showing every stage side by side |

## Compilation pipeline

```
qurts source
    │
    ▼  parser
parse tree (pest)
    │
    ▼  lower
qduc + qauc + mlrd MLIR dialects
    │
    ▼  transforms: qurts-infer-drops, convert-mlrd-to-qco
qco (mqt-core's value-semantics gate dialect)
    │
    ▼  transforms: qurts-controlled-conjugation, then MQT's own passes/exports
optimized circuit (MQT handles OpenQASM 3 / QIR export and simulation)
```

Gate-level optimization is deliberately not re-implemented here: what stays in this repo is the
part that needs Qurts' lifetime and ownership structure (when a qubit is dropped, how it is
uncomputed, how `qif` becomes controlled gates), and everything below that is mqt-core's.

## AI assistance

Parts of this project were developed with the assistance of Claude (Anthropic). Usage is primarily restricted to brainstorming and proofreading. All generated
code and design suggestions have been reviewed by the project author. All AI usage is in line
with the University of Edinburgh's current [generative AI policy & assement regulations](https://registryservices.ed.ac.uk/academic-services/policies-regulations/regulations).

## Building

The `parser` crate has no system dependencies.

Everything else depends on [melior](https://github.com/raviqqe/melior) (Rust MLIR bindings) and
requires LLVM/MLIR 23.1 (`brew install llvm cmake ninja`; the base `mlir-sys` crate locates it via
`llvm-config` on `PATH` or an `MLIR_SYS_230_PREFIX` env var).

Beyond that, each dialect has a hand-written C++ implementation generated from its `.td` file, and
`transforms` has C++ passes plus the mqt-core submodule (`git submodule update --init`), all of
which must be built with CMake *before* `cargo build`/`cargo test` will link — see
`CLAUDE.md`'s "Build & test commands" for the exact invocations (Homebrew clang and an explicit
`CMAKE_OSX_SYSROOT` are required by mqt-core). `cargo build` does not build this C++ side itself;
it only links against the static libraries CMake produces.
