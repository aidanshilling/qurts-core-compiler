// RUN: qurts-opt %s --qurts-infer-drops | FileCheck %s

// The qif result inherits its control's lifetime: its drop must land before
// that lifetime's qduc.end, not at the end of the block.
// CHECK-LABEL: func.func @qif_result_dropped_before_control_lifetime_ends
// CHECK:       %[[Y:.*]] = mlrd.qif
// CHECK:       qauc.uncompute %[[Y]]
// CHECK-NEXT:  qduc.end
// CHECK-NEXT:  mlrd.meas
func.func @qif_result_dropped_before_control_lifetime_ends() -> i1 {
  %0 = mlrd.lifted "0"() : () -> !qauc.qbit
  %1 = mlrd.unitary "H"(%0) : (!qauc.qbit) -> !qauc.qbit
  %lt = qduc.newlft : !qduc.lt
  %r = qauc.borrow %1, %lt : (!qauc.qbit, !qduc.lt) -> !qauc.ref<!qduc.lt, !qauc.qbit>
  %y = mlrd.qif %r {
    %z = mlrd.lifted "1"() : () -> !qauc.qbit
    mlrd.yield %z : !qauc.qbit
  } else {
    %z = mlrd.lifted "0"() : () -> !qauc.qbit
    mlrd.yield %z : !qauc.qbit
  } : (!qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit
  qduc.end %lt
  %b = mlrd.meas %1 : !qauc.qbit -> i1
  return %b : i1
}

// A fresh ancilla left over inside a branch is dropped at the end of that branch.
// CHECK-LABEL: func.func @branch_local_ancilla_dropped_in_branch
// CHECK:       mlrd.qif
// CHECK:         %[[A:.*]] = mlrd.lifted "not"
// CHECK:         qauc.uncompute %[[A]]
// CHECK-NEXT:    mlrd.yield
func.func @branch_local_ancilla_dropped_in_branch(%t: !qauc.qbit) -> !qauc.qbit {
  %0 = mlrd.lifted "0"() : () -> !qauc.qbit
  %1 = mlrd.unitary "H"(%0) : (!qauc.qbit) -> !qauc.qbit
  %lt = qduc.newlft : !qduc.lt
  %r = qauc.borrow %1, %lt : (!qauc.qbit, !qduc.lt) -> !qauc.ref<!qduc.lt, !qauc.qbit>
  %y = mlrd.qif %r {
    %a = mlrd.lifted "0"() : () -> !qauc.qbit
    %a1 = mlrd.lifted "not"(%a) : (!qauc.qbit) -> !qauc.qbit
    %lb = qduc.newlft : !qduc.lt
    %ra = qauc.borrow %a1, %lb : (!qauc.qbit, !qduc.lt) -> !qauc.ref<!qduc.lt, !qauc.qbit>
    %t1 = mlrd.qif %ra {
      %t2 = mlrd.lifted "not"(%t) : (!qauc.qbit) -> !qauc.qbit
      mlrd.yield %t2 : !qauc.qbit
    } else {
      mlrd.yield %t : !qauc.qbit
    } : (!qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit
    qduc.end %lb
    mlrd.yield %t1 : !qauc.qbit
  } else {
    mlrd.yield %t : !qauc.qbit
  } : (!qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit
  qduc.end %lt
  %b = mlrd.meas %1 : !qauc.qbit -> i1
  return %y : !qauc.qbit
}
