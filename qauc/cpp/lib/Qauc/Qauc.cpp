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

// Read on $lifetime itself (not the resource generally), so effect-aware
// passes see a same-value conflict with qduc.end's Free and won't reorder
// this past it — advisory only, the verifier below is the real backstop.
void qauc::BorrowOp::getEffects(
    llvm::SmallVectorImpl<
        mlir::SideEffects::EffectInstance<mlir::MemoryEffects::Effect>>
        &effects) {
  effects.emplace_back(mlir::MemoryEffects::Read::get(),
                       &getOperation()->getOpOperand(1),
                       qduc::LifetimeResource::get());
}

void qauc::UniqueBorrowOp::getEffects(
    llvm::SmallVectorImpl<
        mlir::SideEffects::EffectInstance<mlir::MemoryEffects::Effect>>
        &effects) {
  effects.emplace_back(mlir::MemoryEffects::Read::get(),
                       &getOperation()->getOpOperand(1),
                       qduc::LifetimeResource::get());
}

// $result inherits $lifetime's obligation: it must not be used after
// $lifetime's qduc.end, even though that use never shows up in $lifetime's
// own use-list. If $lifetime doesn't have exactly one qduc.end, that's
// qduc::NewLftOp::verify()'s diagnostic to give, not ours — skip silently.
llvm::LogicalResult qauc::BorrowOp::verify() {
  qduc::EndOp end = qduc::findUniqueEnd(getLifetime());
  if (!end)
    return mlir::success();
  if (!qduc::allUsesPrecede(getResult(), end))
    return emitOpError("result used after its lifetime's qduc.end");
  return mlir::success();
}

llvm::LogicalResult qauc::UniqueBorrowOp::verify() {
  qduc::EndOp end = qduc::findUniqueEnd(getLifetime());
  if (!end)
    return mlir::success();
  if (!qduc::allUsesPrecede(getResult(), end))
    return emitOpError("result used after its lifetime's qduc.end");
  return mlir::success();
}
