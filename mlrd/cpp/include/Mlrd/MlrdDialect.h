#pragma once

#include "Qauc/QaucDialect.h"

#include "mlir/IR/BuiltinTypes.h"
#include "mlir/IR/Dialect.h"
#include "mlir/IR/OpDefinition.h"
#include "mlir/IR/OpImplementation.h"
#include "mlir/Interfaces/SideEffectInterfaces.h"

#include "MlrdDialect.h.inc"

#define GET_OP_CLASSES
#include "MlrdOps.h.inc"
