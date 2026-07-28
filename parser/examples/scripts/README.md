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
