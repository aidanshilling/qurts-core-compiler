# qauc C++ dialect

C++ MLIR dialect implementation for qauc, generated from `../td/qauc.td`.

## Dependencies

```bash
brew install llvm cmake ninja
```

## Build

```bash
cmake -S qauc/cpp -B qauc/cpp/build \
  -DMLIR_DIR=$(brew --prefix llvm)/lib/cmake/mlir \
  -G Ninja

cmake --build qauc/cpp/build
```

Run from the repo root. `qauc.td` includes `qduc/td/qduc.td` and reuses
`qduc::LifetimeType`, so this build pulls in `../../qduc/cpp` as a CMake
subdirectory (built into `qauc/cpp/build/qduc-build`) rather than depending on
a separate, independently-built `qduc/cpp/build`.

The build step symlinks `compile_commands.json` into `qauc/cpp/` (and
`qduc/cpp/`) so clangd picks them up automatically.

## lit/FileCheck verifier tests

`tools/qauc-opt` is an `mlir-opt`-alike registering `func`/`arith`/`scf`/`qduc`/`qauc`, used to
verify `../test/verifier/*.mlir` against `BorrowOp`/`UniqueBorrowOp::verify()` with `FileCheck`
and `--verify-diagnostics`. Same tool requirements as `qduc/cpp`'s `check-qduc` (see
[`../../qduc/cpp/README.md`](../../qduc/cpp/README.md)); the `check-qduc` target is also reachable
from this build tree (`qduc/cpp` is pulled in as a subdirectory here).

```bash
cmake --build qauc/cpp/build --target check-qauc  # qauc's own borrow/unique_borrow verifier tests
cmake --build qauc/cpp/build --target check-qduc  # qduc's newlft/end verifier tests, reachable here too
```
