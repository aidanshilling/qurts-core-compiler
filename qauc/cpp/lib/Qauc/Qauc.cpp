#include "Qauc/QaucDialect.h"

#include "mlir/IR/Builders.h"
#include "mlir/IR/DialectImplementation.h"
#include "llvm/ADT/TypeSwitch.h"

// Generated qauc inc defs
#define GET_TYPEDEF_CLASSES
#include "QaucTypes.cpp.inc"

#include "QaucDialect.cpp.inc"

#define GET_OP_CLASSES
#include "QaucOps.cpp.inc"

void qauc::QaucDialect::initialize() {
  addTypes<
#define GET_TYPEDEF_LIST
#include "QaucTypes.cpp.inc"
      >();
  addOperations<
#define GET_OP_LIST
#include "QaucOps.cpp.inc"
      >();
}

// Rust FFI entry point called by Melior C-API wrapper
#include "mlir-c/IR.h"
#include "mlir/CAPI/IR.h"

extern "C" void qauc_register_dialect(MlirContext ctx) {
  unwrap(ctx)->loadDialect<qauc::QaucDialect>();
}

// ----- OP Definitions ------
//
// qbit as a freed resource so generic optimization passes cannot DCE this
// op, mirroring qduc::EndOp.
void qauc::UncomputeOp::getEffects(
    llvm::SmallVectorImpl<
        mlir::SideEffects::EffectInstance<mlir::MemoryEffects::Effect>>
        &effects) {
  effects.emplace_back(mlir::MemoryEffects::Free::get(),
                       qauc::QubitResource::get());
}
