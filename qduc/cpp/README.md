# qduc C++ dialect

C++ MLIR dialect implementation for qduc, generated from `../td/qduc.td`.

## Dependencies

```bash
brew install llvm cmake ninja
```

## Build

```bash
cmake -S qduc/cpp -B qduc/cpp/build \
  -DMLIR_DIR=$(brew --prefix llvm)/lib/cmake/mlir \
  -G Ninja

cmake --build qduc/cpp/build
```

Run from the repo root. The build step symlinks `compile_commands.json` into
`qduc/cpp/` so clangd picks it up automatically.

## lit/FileCheck verifier tests

`tools/qduc-opt` is an `mlir-opt`-alike registering just `func`/`arith`/`scf`/`qduc`, used to
verify `../test/verifier/*.mlir` against `NewLftOp::verify()` with `FileCheck` and
`--verify-diagnostics`. Requires `FileCheck` (ships with `brew install llvm`, at
`$(brew --prefix llvm)/bin`) and `lit` (`pipx install lit`) on `PATH`; the CMake configure step
silently skips the `check-qduc` target if either isn't found.

```bash
cmake --build qduc/cpp/build --target check-qduc
```
