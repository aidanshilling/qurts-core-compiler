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
// Free on $lifetime itself (not just the resource), so effect-aware passes
// see a same-value conflict with qauc.borrow/unique_borrow's Read and won't
// reorder those past this op — also still protects this op from DCE, since
// any declared effect (value-scoped or not) counts as "not dead."
void qduc::EndOp::getEffects(
    llvm::SmallVectorImpl<
        mlir::SideEffects::EffectInstance<mlir::MemoryEffects::Effect>>
        &effects) {
  effects.emplace_back(mlir::MemoryEffects::Free::get(),
                       &getOperation()->getOpOperand(0),
                       qduc::LifetimeResource::get());
}

qduc::EndOp qduc::findUniqueEnd(mlir::Value lifetime) {
  EndOp found;
  for (mlir::OpOperand &use : lifetime.getUses()) {
    if (auto endOp = llvm::dyn_cast<EndOp>(use.getOwner())) {
      if (found)
        return EndOp();
      found = endOp;
    }
  }
  return found;
}

bool qduc::allUsesPrecede(mlir::Value value, mlir::Operation *end) {
  for (mlir::OpOperand &use : value.getUses()) {
    mlir::Operation *owner = use.getOwner();
    if (owner == end)
      continue;
    if (owner->getBlock() != end->getBlock())
      return false;
    if (!owner->isBeforeInBlock(end))
      return false;
  }
  return true;
}

llvm::LogicalResult qduc::NewLftOp::verify() {
  llvm::SmallVector<EndOp> ends;
  for (mlir::OpOperand &use : getLifetime().getUses())
    if (auto endOp = llvm::dyn_cast<EndOp>(use.getOwner()))
      ends.push_back(endOp);

  if (ends.empty())
    return emitOpError("lifetime is never ended with a qduc.end");
  if (ends.size() > 1)
    return emitOpError("lifetime is ended more than once");

  if (!allUsesPrecede(getLifetime(), ends.front()))
    return emitOpError("lifetime used after its qduc.end");
  return mlir::success();
}
