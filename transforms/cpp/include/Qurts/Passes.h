#pragma once

#include "Qauc/QaucDialect.h"
#include "mqt/Dialect/MQT/IR/MQTDialect.h"
#include "mqt/Dialect/QCO/IR/QCODialect.h"

#include "mlir/Dialect/Arith/IR/Arith.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/Pass/Pass.h"

namespace qurts {

#define GEN_PASS_DECL
#include "QurtsPasses.h.inc"

#define GEN_PASS_REGISTRATION
#include "QurtsPasses.h.inc"

} // namespace qurts
