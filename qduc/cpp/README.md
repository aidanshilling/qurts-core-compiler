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
