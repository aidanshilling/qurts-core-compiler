# qurts-core-compiler

Compiler for the qurts quantum programming language. qurts is a statically-typed language with
a lifetime and ownership system designed to enforce correct quantum uncomputation — the requirement
that ancilla qubits are returned to their original state before being released.

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

| Crate    | Description |
|----------|-------------|
| `parser` | PEG grammar and parse tree via pest |
| `qlt`    | MLIR dialect for the lifetime/region constraint system |
| `qss`    | MLIR dialect for quantum sub-structural ownership types |

## Compilation pipeline

```
qurts source
    │
    ▼  parser
parse tree (pest)
    │
    ▼  (AST lowering — TODO)
qlt + qss MLIR dialects
    │
    ▼  (uncomputation lowering pass — TODO)
target quantum circuit dialect (e.g. Catalyst, QIR)
```

## Building

The `parser` crate has no system dependencies. The `qlt` and `qss` crates depend on
[melior](https://github.com/raviqqe/melior) (Rust MLIR bindings) and require MLIR to be
installed with `MLIR_SYS_*` environment variables set appropriately.
