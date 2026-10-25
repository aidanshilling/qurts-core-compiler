// A use nested inside a region of an op that precedes the qduc.end counts as
// preceding it (the position that matters is the enclosing op's).
//
// RUN: qauc-opt %s --allow-unregistered-dialect | FileCheck %s

// CHECK-LABEL: func.func @valid
func.func @valid(%v: i1) {
  %lt = qduc.newlft : !qduc.lt
  %r = qauc.borrow %v, %lt : (i1, !qduc.lt) -> !qauc.ref<!qduc.lt, i1>
  "test.region"() ({
    "test.use"(%r) : (!qauc.ref<!qduc.lt, i1>) -> ()
    "test.yield"() : () -> ()
  }) : () -> ()
  qduc.end %lt
  return
}
