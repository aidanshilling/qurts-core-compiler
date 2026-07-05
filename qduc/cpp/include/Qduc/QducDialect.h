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
