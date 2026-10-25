cmake_minimum_required(VERSION 3.24)
project(qurts-transforms C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

find_package(MLIR REQUIRED CONFIG)
message(STATUS "Found MLIR ${MLIR_VERSION} in ${MLIR_DIR}")

list(APPEND CMAKE_MODULE_PATH "${MLIR_CMAKE_DIR}" "${LLVM_CMAKE_DIR}")
include(TableGen)
include(AddLLVM)
include(AddMLIR)

include_directories(${MLIR_INCLUDE_DIRS} ${LLVM_INCLUDE_DIRS})
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/../..)

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../../mlrd/cpp mlrd-build)

# ---------------------------------------------------------------------------
# mqt-core (git submodule): only the targets we link get built.
# ---------------------------------------------------------------------------

set(MQT_CORE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../../third_party/mqt-core
    CACHE PATH "mqt-core checkout to build the qco dialect and passes from")
if(NOT EXISTS ${MQT_CORE_DIR}/CMakeLists.txt)
  message(FATAL_ERROR "mqt-core not found at ${MQT_CORE_DIR}; run `git submodule update --init`")
endif()

set(BUILD_MQT_CORE_TESTS OFF CACHE BOOL "" FORCE)
set(BUILD_MQT_CORE_BINDINGS OFF CACHE BOOL "" FORCE)
set(BUILD_MQT_CORE_QDMI_DDSIM_DEVICE OFF CACHE BOOL "" FORCE)
set(BUILD_MQT_CORE_QDMI_SC_DEVICE OFF CACHE BOOL "" FORCE)
set(MQT_CORE_INSTALL OFF CACHE BOOL "" FORCE)
# LTO bitcode from Homebrew clang cannot be consumed by the system linker
# that rustc drives, so mqt-core's default thin-LTO must stay off.
set(ENABLE_IPO OFF CACHE BOOL "" FORCE)
set(CMAKE_UNITY_BUILD ON)
add_subdirectory(${MQT_CORE_DIR} mqt-core-build EXCLUDE_FROM_ALL)
set(CMAKE_UNITY_BUILD OFF)

# ---------------------------------------------------------------------------
# Passes
# ---------------------------------------------------------------------------

set(LLVM_TARGET_DEFINITIONS ../td/passes.td)
mlir_tablegen(QurtsPasses.h.inc -gen-pass-decls -name QurtsTransforms)
add_public_tablegen_target(QurtsPassesIncGen)

add_library(QurtsTransforms STATIC
  lib/Qurts/InferDrops.cpp
  lib/Qurts/MlrdToQco.cpp
  lib/Qurts/ControlledConjugation.cpp
  lib/Qurts/Registration.cpp
)
add_dependencies(QurtsTransforms QurtsPassesIncGen)

target_include_directories(QurtsTransforms
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}>
)

target_link_libraries(QurtsTransforms
  PUBLIC
    MlrdDialect
    QaucDialect
    QducDialect
    MLIRQCODialect
    MLIRQCOTransforms
    MLIRQCOUtils
    MLIRQCODDFunctionality
    MLIRMQTDialect
    MLIRSupportMQT
    MLIRFuncDialect
    MLIRArithDialect
    MLIRPass
    MLIRTransforms
    MLIRTransformUtils
    MLIRCAPIIR
)

if(NOT LLVM_ENABLE_RTTI)
  target_compile_options(QurtsTransforms PRIVATE -fno-rtti)
endif()

# ---------------------------------------------------------------------------
# qurts-opt + lit tests (../test)
# ---------------------------------------------------------------------------

add_executable(qurts-opt tools/qurts-opt/qurts-opt.cpp)
target_link_libraries(qurts-opt PRIVATE QurtsTransforms MLIROptLib MLIRSCFDialect)
if(NOT LLVM_ENABLE_RTTI)
  target_compile_options(qurts-opt PRIVATE -fno-rtti)
endif()

find_program(LIT_EXECUTABLE NAMES lit llvm-lit)
find_program(FILECHECK_EXECUTABLE NAMES FileCheck HINTS ${LLVM_TOOLS_BINARY_DIR})

if(LIT_EXECUTABLE AND FILECHECK_EXECUTABLE)
  set(QURTS_TEST_SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../test)
  set(QURTS_TEST_BINARY_DIR ${CMAKE_CURRENT_BINARY_DIR}/test)
  set(QURTS_OPT_EXECUTABLE ${CMAKE_CURRENT_BINARY_DIR}/qurts-opt)
  configure_file(
    ${QURTS_TEST_SOURCE_DIR}/lit.site.cfg.py.in
    ${QURTS_TEST_BINARY_DIR}/lit.site.cfg.py
    @ONLY
  )
  add_custom_target(check-transforms
    COMMAND ${LIT_EXECUTABLE} -v ${QURTS_TEST_BINARY_DIR}
    DEPENDS qurts-opt
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
    COMMENT "Running transforms lit/FileCheck tests"
  )
else()
  message(STATUS "transforms: lit and/or FileCheck not found — skipping check-transforms target")
endif()

add_custom_target(link_compile_commands_transforms ALL
  COMMAND ${CMAKE_COMMAND} -E rm -f ${CMAKE_CURRENT_SOURCE_DIR}/compile_commands.json
  COMMAND ${CMAKE_COMMAND} -E create_symlink
    ${CMAKE_CURRENT_BINARY_DIR}/compile_commands.json
    ${CMAKE_CURRENT_SOURCE_DIR}/compile_commands.json
  VERBATIM
)
