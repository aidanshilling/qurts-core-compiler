cmake_minimum_required(VERSION 3.20)
project(qauc-dialect C CXX)

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

# qauc.td does `include "qduc/td/qduc.td"` assuming the repo root is on the
# tablegen include path.
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/../..)

# ---------------------------------------------------------------------------
# qduc — qauc.td reuses qduc::LifetimeType, so pull in its generated headers
# and static library.
# ---------------------------------------------------------------------------

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../../qduc/cpp qduc-build)

# ---------------------------------------------------------------------------
# TableGen — generate all .inc files from qauc.td
# ---------------------------------------------------------------------------

# Path is relative to CMAKE_CURRENT_SOURCE_DIR (this file lives in cpp/)
set(LLVM_TARGET_DEFINITIONS ../td/qauc.td)

# Order matters in the header: dialect before types, types before ops.
mlir_tablegen(QaucDialect.h.inc   -gen-dialect-decls  -dialect=qauc)
mlir_tablegen(QaucDialect.cpp.inc -gen-dialect-defs   -dialect=qauc)
mlir_tablegen(QaucTypes.h.inc     -gen-typedef-decls  -typedefs-dialect=qauc)
mlir_tablegen(QaucTypes.cpp.inc   -gen-typedef-defs   -typedefs-dialect=qauc)
# qauc.td defines no AttrDef records yet — add -gen-attrdef-decls/-defs (and
# set useDefaultAttributePrinterParser = 1 on Qauc_Dialect) once it does.
# -gen-op-decls/-defs have no dialect filter flag (unlike -typedefs-dialect
# above), so without -op-include-regex they'd also emit qduc's ops pulled in
# transitively via qauc.td's `include "qduc/td/qduc.td"`, colliding with the
# definitions already generated into QducOps.h.inc.
mlir_tablegen(QaucOps.h.inc       -gen-op-decls -op-include-regex "qauc\\..*")
mlir_tablegen(QaucOps.cpp.inc     -gen-op-defs  -op-include-regex "qauc\\..*")

# Rolls all the add_custom_command outputs into one CMake target
add_public_tablegen_target(QaucIncGen)

# ---------------------------------------------------------------------------
# Library
# ---------------------------------------------------------------------------

add_library(QaucDialect STATIC
  lib/Qauc/Qauc.cpp
)

# QaucIncGen must run before we compile anything
add_dependencies(QaucDialect QaucIncGen)

target_include_directories(QaucDialect
  PUBLIC
    # Hand-written public header (include/Qauc/QaucDialect.h)
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    # Generated .inc files land flat in the binary dir
    $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}>
)

target_link_libraries(QaucDialect
  PUBLIC
    MLIRIR                   # Dialect, Type, Op, Attribute base classes
    MLIRSupport              # llvm::SmallVector, StringRef, etc.
    MLIRSideEffectInterfaces # MemoryEffects, SideEffects::Resource
    QducDialect               # qduc::LifetimeType, qduc::QducDialect
)

# Match the ABI of the installed MLIR (Homebrew LLVM is built without EH/RTTI)
if(NOT LLVM_ENABLE_EH)
  target_compile_options(QaucDialect PRIVATE -fno-exceptions)
endif()
if(NOT LLVM_ENABLE_RTTI)
  target_compile_options(QaucDialect PRIVATE -fno-rtti)
endif()

# ---------------------------------------------------------------------------
# compile_commands.json — symlink into source tree so clangd finds it
# ---------------------------------------------------------------------------

add_custom_target(link_compile_commands_qauc ALL
  COMMAND ${CMAKE_COMMAND} -E rm -f
    ${CMAKE_CURRENT_SOURCE_DIR}/compile_commands.json
  COMMAND ${CMAKE_COMMAND} -E create_symlink
    ${CMAKE_CURRENT_BINARY_DIR}/compile_commands.json
    ${CMAKE_CURRENT_SOURCE_DIR}/compile_commands.json
  VERBATIM
)
