# mlrd C++ dialect

C++ MLIR dialect implementation for mlrd, generated from `../td/mlrd.td`.

## Dependencies

```bash
brew install llvm cmake ninja
```

## Build

```bash
cmake -S mlrd/cpp -B mlrd/cpp/build \
  -DMLIR_DIR=$(brew --prefix llvm)/lib/cmake/mlir \
  -G Ninja

cmake --build mlrd/cpp/build
```

Run from the repo root. `mlrd.td` includes `qauc/td/qauc.td` (which itself includes
`qduc/td/qduc.td`) and reuses `qauc::RefType`/`qauc::QubitType`, so this build pulls in
`../../qauc/cpp` as a CMake subdirectory (built into `mlrd/cpp/build/qauc-build`, which itself
pulls in `qduc/cpp` into `mlrd/cpp/build/qauc-build/qduc-build`) rather than depending on
separately-built `qauc/cpp/build`/`qduc/cpp/build` trees.

The build step symlinks `compile_commands.json` into `mlrd/cpp/` (and `qauc/cpp/`, `qduc/cpp/`)
so clangd picks them up automatically.
