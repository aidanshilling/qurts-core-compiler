#pragma once

#include "Qduc/QducDialect.h"

#include "mlir/IR/BuiltinTypes.h"
#include "mlir/IR/Dialect.h"
#include "mlir/IR/OpDefinition.h"
#include "mlir/IR/OpImplementation.h"
#include "mlir/Interfaces/SideEffectInterfaces.h"

#include "QaucDialect.h.inc"

#define GET_TYPEDEF_CLASSES
#include "QaucTypes.h.inc"

namespace qauc {
struct QubitResource
    : public mlir::SideEffects::Resource::Base<QubitResource> {
  llvm::StringRef getName() final { return "QubitResource"; }
};
} // namespace qauc

#define GET_OP_CLASSES
#include "QaucOps.h.inc"
