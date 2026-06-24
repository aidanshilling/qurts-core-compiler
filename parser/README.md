# parser

PEG parser for qurts source files, built with [pest](https://pest.rs).

## Grammar

The grammar (`src/qurts.pest`) covers:

- **Functions** — `fn name<lifetimes>(params) -> ty { body }`
- **Lifetime preorders** — `<'a, 'b, 'a <= 'b, 'a != 'c>`
- **Types** — `bool`, `qbit`, `()`, `(T, T)`, `&'a T`, `#'a T`
- **Expressions** — boolean operators, equality, unary `!`/`*`, borrows, function calls, `if`/`else`, blocks
- **Statements** — `let x: T = expr;` and expression statements

## Usage

```rust
use parser::{QurtsParser, Rule};
use pest::Parser;

let tree = QurtsParser::parse(Rule::program, source)?;
```
