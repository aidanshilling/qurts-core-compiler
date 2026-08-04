cmake_minimum_required(VERSION 3.20)
project(mlrd-dialect C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# find_package(MLIR) also pulls in LLVMConfig.cmake
find_package(MLIR REQUIRED CONFIG)

message(STATUS "Found MLIR ${MLIR_VERSION} in ${MLIR_DIR}")

list(APPEND CMAKE_MODULE_PATH "${MLIR_CMAKE_DIR}")
list(APPEND CMAKE_MODULE_PATH "${LLVM_CMAKE_DIR}")

include(TableGen)  # tablegen() function
include(AddLLVM)   # add_llvm_library, etc.
include(AddMLIR)   # mlir_tablegen(), add_mlir_library

# Make MLIR/LLVM .td files findable by mlir-tblgen (e.g. mlir/IR/AttrTypeBase.td)
include_directories(${MLIR_INCLUDE_DIRS})
include_directories(${LLVM_INCLUDE_DIRS})

# mlrd.td does `include "qauc/td/qauc.td"` assuming the repo root is on the
# tablegen include path.
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/../..)

# ---------------------------------------------------------------------------
# qauc — mlrd.td reuses qauc::RefType/QubitType, so pull in its generated
# headers and static library. qauc's own CMake pulls in qduc the same way.
# ---------------------------------------------------------------------------

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../../qauc/cpp qauc-build)

# ---------------------------------------------------------------------------
# TableGen — generate all .inc files from mlrd.td
# ---------------------------------------------------------------------------

# Path is relative to CMAKE_CURRENT_SOURCE_DIR (this file lives in cpp/)
set(LLVM_TARGET_DEFINITIONS ../td/mlrd.td)

mlir_tablegen(MlrdDialect.h.inc   -gen-dialect-decls  -dialect=mlrd)
mlir_tablegen(MlrdDialect.cpp.inc -gen-dialect-defs   -dialect=mlrd)
# -gen-op-decls/-defs have no dialect filter flag, so without -op-include-regex
# they'd also re-emit qauc's (and transitively qduc's) ops pulled in via
# mlrd.td's `include "qauc/td/qauc.td"`, colliding with QaucOps.h.inc.
mlir_tablegen(MlrdOps.h.inc       -gen-op-decls -op-include-regex "mlrd\\..*")
mlir_tablegen(MlrdOps.cpp.inc     -gen-op-defs  -op-include-regex "mlrd\\..*")

# Rolls all the add_custom_command outputs into one CMake target
add_public_tablegen_target(MlrdIncGen)

# ---------------------------------------------------------------------------
# Library
# ---------------------------------------------------------------------------

add_library(MlrdDialect STATIC
  lib/Mlrd/Mlrd.cpp
)

# MlrdIncGen must run before we compile anything
add_dependencies(MlrdDialect MlrdIncGen)

target_include_directories(MlrdDialect
  PUBLIC
    # Hand-written public header (include/Mlrd/MlrdDialect.h)
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    # Generated .inc files land flat in the binary dir
    $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}>
)

target_link_libraries(MlrdDialect
  PUBLIC
    MLIRIR                   # Dialect, Type, Op, Attribute base classes
    MLIRSupport               # llvm::SmallVector, StringRef, etc.
    MLIRSideEffectInterfaces  # MemoryEffects, SideEffects::Resource
    QaucDialect               # qauc::RefType, qauc::QubitType, qauc::QaucDialect
)

# Match the ABI of the installed MLIR (Homebrew LLVM is built without EH/RTTI)
if(NOT LLVM_ENABLE_EH)
  target_compile_options(MlrdDialect PRIVATE -fno-exceptions)
endif()
if(NOT LLVM_ENABLE_RTTI)
  target_compile_options(MlrdDialect PRIVATE -fno-rtti)
endif()

# ---------------------------------------------------------------------------
# compile_commands.json — symlink into source tree so clangd finds it
# ---------------------------------------------------------------------------

add_custom_target(link_compile_commands_mlrd ALL
  COMMAND ${CMAKE_COMMAND} -E rm -f
    ${CMAKE_CURRENT_SOURCE_DIR}/compile_commands.json
  COMMAND ${CMAKE_COMMAND} -E create_symlink
    ${CMAKE_CURRENT_BINARY_DIR}/compile_commands.json
    ${CMAKE_CURRENT_SOURCE_DIR}/compile_commands.json
  VERBATIM
)
