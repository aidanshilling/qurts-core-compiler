// RUN: qauc-opt %s | FileCheck %s

// CHECK-LABEL: func.func @valid
func.func @valid(%v: i1) {
  // CHECK: %[[LT:.*]] = qduc.newlft
  %lt = qduc.newlft : !qduc.lt
  // CHECK: %[[U:.*]] = qauc.unique_borrow
  %u = qauc.unique_borrow %v, %lt : (i1, !qduc.lt) -> !qauc.unique<!qduc.lt, i1>
  // CHECK: qauc.release %[[U]]
  qauc.release %u : !qauc.unique<!qduc.lt, i1>
  // CHECK: qduc.end %[[LT]]
  qduc.end %lt
  return
}
