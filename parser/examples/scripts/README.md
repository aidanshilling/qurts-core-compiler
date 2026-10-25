# Example scripts

Each file uses only grammar currently supported by `parser/src/qurts.pest`. Parsed and
printed by `parser/tests/example_scripts.rs` (`cargo test -p parser --test example_scripts
-- --nocapture`) and individually via `cargo run -p parser --example print_ast -- <file>`.

- `basic.qurts` - functions, params, `if`/`else`
- `lifetimes.qurts` - lifetime preorders, borrow, `newlft`/`endlft`
- `quantum_gates.qurts` - lifted state prep (`[0]()`), unitary application (`H(x)`), `meas`
- `qif.qurts` - quantum conditional over a borrowed qubit
- `calls_and_tuples.qurts` - zero/multi-arg calls, tuples
- `uncompute_walkthrough.qurts` - the borrow/qif/lifetime example from the design discussion
- `toy_example.qurts` - the paper's Section 3.1 toy example: a `qif` result dropped before its
  control lifetime ends (uncomputed via the control), then the control measured
- `qif_ancilla.qurts` - an ancilla computed, used as a nested `qif` control, and dropped inside
  a `qif` branch: the controlled-conjugation case (`ctrl(U V U†)` -> `U ctrl(V) U†`)
- `nested_qif.qurts` - two-level `qif` nesting with a non-trivial `else` branch
- `lifted_and.qurts` - the paper's `and` (nested `qif`s computing x AND y into a fresh qubit)
  inlined into `main`, used as a control, then explicitly `drop`ped

Gate names in `U(x)` start with an uppercase letter (`H`, `X`, `CX`), which is how `H(x)` is
told apart from a call `h(x)`. Lifted gates `[c](x, ...)` take any number of qubits and return
one result per operand (`let (a, b) : (qbit, qbit) = [cnot](a, b);`); `[0]()`/`[1]()` allocate.

Per the paper's typing (Fig. 5), results of `U(x)` are linear and must be consumed (measured,
returned, or passed on); only lifted results and `qif` results are affine and may be dropped,
implicitly or with `drop x;`.
