#include "Qduc/QducDialect.h"

#include "mlir/IR/Builders.h"
#include "mlir/IR/DialectImplementation.h"
#include "llvm/ADT/TypeSwitch.h"

// Generated qduc inc defs
#include "QducEnums.cpp.inc"

#define GET_TYPEDEF_CLASSES
#include "QducTypes.cpp.inc"

#define GET_ATTRDEF_CLASSES
#include "QducAttrs.cpp.inc"

#include "QducDialect.cpp.inc"

#define GET_OP_CLASSES
#include "QducOps.cpp.inc"

void qduc::QducDialect::initialize() {
  addTypes<
#define GET_TYPEDEF_LIST
#include "QducTypes.cpp.inc"
      >();
  addAttributes<
#define GET_ATTRDEF_LIST
#include "QducAttrs.cpp.inc"
      >();
  addOperations<
#define GET_OP_LIST
#include "QducOps.cpp.inc"
      >();
}

// Rust FFI entry point called by Melior C-API wrapper
#include "mlir-c/IR.h"
#include "mlir/CAPI/IR.h"

extern "C" void qduc_register_dialect(MlirContext ctx) {
  unwrap(ctx)->loadDialect<qduc::QducDialect>();
}

// ----- OP Definitions ------
//
// lifetime token as a freed resource so passes cannot DCE this op.
void qduc::EndOp::getEffects(
    llvm::SmallVectorImpl<
        mlir::SideEffects::EffectInstance<mlir::MemoryEffects::Effect>>
        &effects) {
  effects.emplace_back(mlir::MemoryEffects::Free::get(),
                       qduc::LifetimeResource::get());
}
