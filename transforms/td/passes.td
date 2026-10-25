#ifndef QURTS_PASSES_TD
#define QURTS_PASSES_TD

include "mlir/Pass/PassBase.td"

def QurtsInferDrops : Pass<"qurts-infer-drops", "mlir::func::FuncOp"> {
  let summary = "insert qauc.uncompute for owned qubits that are never consumed";
  let description = [{
    Implements the paper's drop inference: every owned `!qauc.qbit` value with
    no consuming use is dropped as late as possible — before the `qduc.end` of
    the lifetime its type carries (a `qif` result inherits its control's
    lifetime, a lifted result its operands'), otherwise before the block
    terminator. Results of `mlrd.unitary` are linear (`#'0 qbit`) and cannot be
    dropped; leaving one unconsumed is an error.
  }];
  let dependentDialects = ["::qauc::QaucDialect"];
}

def ConvertMlrdToQco : Pass<"convert-mlrd-to-qco", "mlir::ModuleOp"> {
  let summary = "lower mlrd/qauc/qduc to the MQT qco dialect";
  let description = [{
    Value-semantics gate lowering. Lifetime tokens and borrows disappear (a
    borrow of a qubit is just the qubit); `mlrd.qif` becomes a `qco.ctrl`
    region per branch (the else branch is sandwiched in `qco.x` on the
    control), with fresh allocations inside branches hoisted out and the two
    branches' yielded fresh qubits unified; `qauc.uncompute` becomes a
    `qco.inv` of the qubit's gate history followed by `qco.sink`.
  }];
  let dependentDialects = [
    "::mlir::qco::QCODialect", "::mlir::mqt::MQTDialect",
    "::mlir::arith::ArithDialect", "::mlir::func::FuncDialect"
  ];
}

def QurtsControlledConjugation
    : Pass<"qurts-controlled-conjugation", "mlir::ModuleOp"> {
  let summary = "hoist compute/uncompute pairs out of qco.ctrl bodies";
  let description = [{
    Rewrites `ctrl(U V U^-1)` into `U ctrl(V) U^-1`: when a `qco.ctrl` body
    ends in a `qco.inv` whose body is a copy of the ops that start the same
    wires, those ops and the inverse move outside the control, leaving only V
    controlled.
  }];
  let dependentDialects = ["::mlir::qco::QCODialect"];
}

#endif // QURTS_PASSES_TD
