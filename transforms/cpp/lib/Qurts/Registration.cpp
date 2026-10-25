#include "Qurts/Passes.h"

#include "mqt/Dialect/MQT/IR/MQTDialect.h"
#include "mqt/Dialect/QCO/IR/QCODialect.h"
#include "mqt/Dialect/QCO/QCOUtils.h"
#include "mqt/Dialect/QCO/Utils/DDFunctionality.h"
#include "mqt/Support/Passes.h"

#include "dd/Edge.hpp"
#include "dd/Package.hpp"

#include "mlir-c/IR.h"
#include "mlir/CAPI/IR.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"

#include <cstdint>

extern "C" void qurts_register_dialects(MlirContext context) {
  unwrap(context)->loadDialect<mlir::qco::QCODialect, mlir::mqt::MQTDialect>();
}

extern "C" void qurts_register_passes() {
  static const bool registered = [] {
    qurts::registerQurtsTransformsPasses();
    registerMQTCompilerPasses();
    return true;
  }();
  static_cast<void>(registered);
}

extern "C" int64_t qurts_statevector(MlirModule module, const char *function, double *re, double *im,
                                      int64_t capacity) {
  mlir::ModuleOp mod = unwrap(module);
  auto func = mod.lookupSymbol<mlir::func::FuncOp>(function);
  if (!func) return -1;
  if (mlir::failed(mlir::qco::verifyLinearity(mod))) return -2;
  dd::Package package;
  auto state = mlir::qco::simulateStatevector(func, package);
  if (mlir::failed(state)) return -3;
  dd::CVec amplitudes = dd::getVector(*state);
  auto count = static_cast<int64_t>(amplitudes.size());
  if (capacity < count) return count;
  for (int64_t i = 0; i < count; ++i) {
    re[i] = amplitudes[i].real();
    im[i] = amplitudes[i].imag();
  }
  return count;
}
