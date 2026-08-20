#pragma once

#include "mlir/IR/BuiltinTypes.h"
#include "mlir/IR/Dialect.h"
#include "mlir/IR/OpDefinition.h"
#include "mlir/IR/OpImplementation.h"
#include "mlir/Interfaces/SideEffectInterfaces.h"

// Enums must come before attrs/ops that reference OrderingKind.
#include "QducEnums.h.inc"

#include "QducDialect.h.inc"

#define GET_TYPEDEF_CLASSES
#include "QducTypes.h.inc"

#define GET_ATTRDEF_CLASSES
#include "QducAttrs.h.inc"

namespace qduc {
struct LifetimeResource
    : public mlir::SideEffects::Resource::Base<LifetimeResource> {
  llvm::StringRef getName() final { return "LifetimeResource"; }
};
} // namespace qduc

#define GET_OP_CLASSES
#include "QducOps.h.inc"

namespace qduc {
// Returns the unique EndOp consuming `lifetime`, or a null EndOp if there
// isn't exactly one — silently, no diagnostic. Callers other than
// NewLftOp::verify() should just skip their own check when this returns
// null, since NewLftOp::verify() independently reports the "wrong number of
// ends" error; we don't want it reported twice.
EndOp findUniqueEnd(mlir::Value lifetime);

// True if every use of `value` occurs before `end` in program order. Only
// supports uses in the same block as `end` today.
bool allUsesPrecede(mlir::Value value, mlir::Operation *end);
} // namespace qduc
