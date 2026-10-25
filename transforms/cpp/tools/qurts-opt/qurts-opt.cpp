#include "Mlrd/MlrdDialect.h"
#include "Qauc/QaucDialect.h"
#include "Qduc/QducDialect.h"
#include "Qurts/Passes.h"
#include "mqt/Dialect/MQT/IR/MQTDialect.h"
#include "mqt/Dialect/QCO/IR/QCODialect.h"
#include "mqt/Support/Passes.h"

#include "mlir/Dialect/Arith/IR/Arith.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/Dialect/SCF/IR/SCF.h"
#include "mlir/IR/DialectRegistry.h"
#include "mlir/Tools/mlir-opt/MlirOptMain.h"

int main(int argc, char **argv) {
  mlir::DialectRegistry registry;
  registry.insert<mlir::func::FuncDialect, mlir::arith::ArithDialect, mlir::scf::SCFDialect,
                  qduc::QducDialect, qauc::QaucDialect, mlrd::MlrdDialect, mlir::qco::QCODialect,
                  mlir::mqt::MQTDialect>();
  qurts::registerQurtsTransformsPasses();
  registerMQTCompilerPasses();
  return mlir::asMainReturnCode(mlir::MlirOptMain(argc, argv, "qurts-opt\n", registry));
}
