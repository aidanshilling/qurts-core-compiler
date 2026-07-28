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
