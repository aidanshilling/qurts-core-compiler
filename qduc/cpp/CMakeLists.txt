cmake_minimum_required(VERSION 3.20)
project(qduc-dialect C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# find_package(MLIR) also pulls in LLVMConfig.cmake
find_package(MLIR REQUIRED CONFIG)

message(STATUS "Found MLIR ${MLIR_VERSION} in ${MLIR_DIR}")

# AddMLIR.cmake and TableGen.cmake live in the MLIR/LLVM cmake dirs
list(APPEND CMAKE_MODULE_PATH "${MLIR_CMAKE_DIR}")
list(APPEND CMAKE_MODULE_PATH "${LLVM_CMAKE_DIR}")

include(TableGen)  # tablegen() function
include(AddLLVM)   # add_llvm_library, etc.
include(AddMLIR)   # mlir_tablegen(), add_mlir_library

# Make MLIR/LLVM .td files findable by mlir-tblgen (e.g. mlir/IR/AttrTypeBase.td)
include_directories(${MLIR_INCLUDE_DIRS})
include_directories(${LLVM_INCLUDE_DIRS})

# ---------------------------------------------------------------------------
# TableGen — generate all .inc files from qduc.td
# ---------------------------------------------------------------------------

# Path is relative to CMAKE_CURRENT_SOURCE_DIR (this file lives in cpp/)
set(LLVM_TARGET_DEFINITIONS ../td/qduc.td)

# Order matters in the header: enums before attrs, dialect before types.
mlir_tablegen(QducDialect.h.inc   -gen-dialect-decls  -dialect=qduc)
mlir_tablegen(QducDialect.cpp.inc -gen-dialect-defs   -dialect=qduc)
mlir_tablegen(QducEnums.h.inc     -gen-enum-decls)
mlir_tablegen(QducEnums.cpp.inc   -gen-enum-defs)
mlir_tablegen(QducTypes.h.inc     -gen-typedef-decls  -typedefs-dialect=qduc)
mlir_tablegen(QducTypes.cpp.inc   -gen-typedef-defs   -typedefs-dialect=qduc)
mlir_tablegen(QducAttrs.h.inc     -gen-attrdef-decls  -attrdefs-dialect=qduc)
mlir_tablegen(QducAttrs.cpp.inc   -gen-attrdef-defs   -attrdefs-dialect=qduc)
mlir_tablegen(QducOps.h.inc       -gen-op-decls)
mlir_tablegen(QducOps.cpp.inc     -gen-op-defs)

# Rolls all the add_custom_command outputs into one CMake target
add_public_tablegen_target(QducIncGen)

# ---------------------------------------------------------------------------
# Library
# ---------------------------------------------------------------------------

add_library(QducDialect STATIC
  lib/Qduc/Qduc.cpp
)

# QducIncGen must run before we compile anything
add_dependencies(QducDialect QducIncGen)

target_include_directories(QducDialect
  PUBLIC
    # Hand-written public header (include/Qduc/QducDialect.h)
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    # Generated .inc files land flat in the binary dir
    $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}>
)

target_link_libraries(QducDialect
  PUBLIC
    MLIRIR                   # Dialect, Type, Op, Attribute base classes
    MLIRSupport              # llvm::SmallVector, StringRef, etc.
    MLIRSideEffectInterfaces # MemoryEffects, SideEffects::Resource
)

# Match the ABI of the installed MLIR (Homebrew LLVM is built without EH/RTTI)
if(NOT LLVM_ENABLE_EH)
  target_compile_options(QducDialect PRIVATE -fno-exceptions)
endif()
if(NOT LLVM_ENABLE_RTTI)
  target_compile_options(QducDialect PRIVATE -fno-rtti)
endif()

# ---------------------------------------------------------------------------
# qduc-opt — mlir-opt-alike registering just qduc (+ func/arith/scf), for
# lit/FileCheck verifier tests in ../test.
# ---------------------------------------------------------------------------

add_executable(qduc-opt tools/qduc-opt/qduc-opt.cpp)

target_link_libraries(qduc-opt
  PRIVATE
    QducDialect
    MLIROptLib
    MLIRFuncDialect
    MLIRArithDialect
    MLIRSCFDialect
)

if(NOT LLVM_ENABLE_EH)
  target_compile_options(qduc-opt PRIVATE -fno-exceptions)
endif()
if(NOT LLVM_ENABLE_RTTI)
  target_compile_options(qduc-opt PRIVATE -fno-rtti)
endif()

# ---------------------------------------------------------------------------
# lit/FileCheck verifier tests (../test) — optional: only wired up if both
# `lit` and `FileCheck` are found.
# ---------------------------------------------------------------------------

find_program(LIT_EXECUTABLE NAMES lit llvm-lit)
find_program(FILECHECK_EXECUTABLE NAMES FileCheck HINTS ${LLVM_TOOLS_BINARY_DIR})

if(LIT_EXECUTABLE AND FILECHECK_EXECUTABLE)
  set(QDUC_TEST_SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../test)
  set(QDUC_TEST_BINARY_DIR ${CMAKE_CURRENT_BINARY_DIR}/test)
  # Ninja is a single-config generator (see build instructions in CLAUDE.md),
  # so the executable lands directly here with no Debug/Release subdirectory.
  set(QDUC_OPT_EXECUTABLE ${CMAKE_CURRENT_BINARY_DIR}/qduc-opt)

  configure_file(
    ${QDUC_TEST_SOURCE_DIR}/lit.site.cfg.py.in
    ${QDUC_TEST_BINARY_DIR}/lit.site.cfg.py
    @ONLY
  )
  add_custom_target(check-qduc
    COMMAND ${LIT_EXECUTABLE} -v ${QDUC_TEST_BINARY_DIR}
    DEPENDS qduc-opt
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
    COMMENT "Running qduc lit/FileCheck verifier tests"
  )
else()
  message(STATUS "qduc: lit and/or FileCheck not found — skipping check-qduc target")
endif()

# ---------------------------------------------------------------------------
# compile_commands.json — symlink into source tree so clangd finds it
# ---------------------------------------------------------------------------

add_custom_target(link_compile_commands ALL
  COMMAND ${CMAKE_COMMAND} -E rm -f
    ${CMAKE_CURRENT_SOURCE_DIR}/compile_commands.json
  COMMAND ${CMAKE_COMMAND} -E create_symlink
    ${CMAKE_CURRENT_BINARY_DIR}/compile_commands.json
    ${CMAKE_CURRENT_SOURCE_DIR}/compile_commands.json
  VERBATIM
)
