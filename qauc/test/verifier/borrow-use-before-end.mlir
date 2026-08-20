// RUN: qauc-opt %s --allow-unregistered-dialect | FileCheck %s

// CHECK-LABEL: func.func @valid
func.func @valid(%v: i1) {
  // CHECK: %[[LT:.*]] = qduc.newlft
  %lt = qduc.newlft : !qduc.lt
  // CHECK: %[[R:.*]] = qauc.borrow
  %r = qauc.borrow %v, %lt : (i1, !qduc.lt) -> !qauc.ref<!qduc.lt, i1>
  // CHECK: "test.use"(%[[R]])
  "test.use"(%r) : (!qauc.ref<!qduc.lt, i1>) -> ()
  // CHECK: qduc.end %[[LT]]
  qduc.end %lt
  return
}
