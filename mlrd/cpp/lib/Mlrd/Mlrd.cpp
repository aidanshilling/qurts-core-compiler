#include "Mlrd/MlrdDialect.h"

#include "mlir/IR/Builders.h"
#include "mlir/IR/DialectImplementation.h"
#include "llvm/ADT/TypeSwitch.h"

// Generated mlrd inc defs
#include "MlrdDialect.cpp.inc"

#define GET_OP_CLASSES
#include "MlrdOps.cpp.inc"

void mlrd::MlrdDialect::initialize() {
  addOperations<
#define GET_OP_LIST
#include "MlrdOps.cpp.inc"
      >();
}

// Rust FFI entry point called by Melior C-API wrapper
#include "mlir-c/IR.h"
#include "mlir/CAPI/IR.h"

extern "C" void mlrd_register_dialect(MlirContext ctx) {
  unwrap(ctx)->loadDialect<mlrd::MlrdDialect>();
}

// ----- OP Definitions ------

static llvm::LogicalResult verifyQifBranch(mlrd::QifOp op, mlir::Region &region,
                                            llvm::StringRef branchName) {
  mlir::Block &block = region.front();
  auto yield = llvm::dyn_cast<mlrd::YieldOp>(block.back());
  if (!yield)
    return op.emitOpError() << branchName << " region must end in scf.yield";

  mlir::ResultRange results = op.getResults();
  if (yield.getResults().size() != results.size())
    return op.emitOpError() << branchName << " region yields "
                             << yield.getResults().size()
                             << " value(s), expected " << results.size();

  for (auto [yielded, expected] : llvm::zip(yield.getResults(), results))
    if (yielded.getType() != expected.getType())
      return op.emitOpError()
             << branchName << " region yields " << yielded.getType()
             << ", expected " << expected.getType();

  return mlir::success();
}

llvm::LogicalResult mlrd::QifOp::verify() {
  auto refType = llvm::dyn_cast<qauc::RefType>(getCondition().getType());
  if (!refType || !llvm::isa<qauc::QubitType>(refType.getValueType()))
    return emitOpError("condition must be a !qauc.ref of !qauc.qbit");

  if (mlir::failed(verifyQifBranch(*this, getThenRegion(), "then")))
    return mlir::failure();
  return verifyQifBranch(*this, getElseRegion(), "else");
}
