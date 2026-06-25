# qss

MLIR dialect for qurts quantum sub-structural ownership types.

`qss` bridges the `qlt` lifetime system with quantum resource tracking. It enforces the
sub-structural rules that make correct quantum uncomputation statically verifiable: a
`!qss.unique<lt, qbit>` value must be consumed exactly once, and its lifetime token must
have been ended by the time the value is consumed.

## Planned contents

**Types**
- `!qss.qbit` - quantum bit; a linear type that cannot be copied (no-cloning theorem)
- `!qss.unique<lt, T>` - unique reference `#'a T`; consumed exactly once (for qubits you own)
- `!qss.ref<lt, T>` - shared reference `&'a T`; classically borrowed. When `T` is `!qss.qbit`,
  the reference is only valid as the control qubit in a `qif` (for qubits you dont own but need to be alive) — it cannot be used for arbitrary
  gate application. Its primary role is to express that a qubit must remain alive for lifetime `lt`
  so that other qubits entangled with it can be uncomputed. This is something we probably want the verifier to handle. 
  We can use to statically confirm uncomputation ordering.

**Ops**
- `qss.borrow` - creates a `!qss.ref` from a value and a lifetime token
- `qss.unique_borrow` - creates a `!qss.unique` from a value and a lifetime token
- `qss.release` - consumes a `!qss.unique`, discharging the ownership obligation
- `qss.uncompute` - marks the point at which a qbit must have been returned to |0>;
  lowered to an inverse gate sequence by the uncomputation pass

## Uncomputation lowering

`qss.uncompute` is an opaque obligation op. A dedicated lowering pass is responsible for
tracing which gate ops were applied to a given qbit, generating their inverses, and
emitting them into the target quantum circuit dialect (e.g. Catalyst, QIR).

## Dependencies

- `qlt` (path dependency) - lifetime tokens used in `!qss.unique` and `!qss.ref` type parameters
- [melior](https://github.com/raviqqe/melior)
