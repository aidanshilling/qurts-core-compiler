// Same derived-obligation case as unique-borrow-release-after-end.mlir, but
// exercising qauc::BorrowOp::verify() instead of UniqueBorrowOp::verify().
//
// RUN: qauc-opt %s --allow-unregistered-dialect --verify-diagnostics

func.func @invalid(%v: i1) {
  %lt = qduc.newlft : !qduc.lt
  // expected-error @+1 {{'qauc.borrow' op result used after its lifetime's qduc.end}}
  %r = qauc.borrow %v, %lt : (i1, !qduc.lt) -> !qauc.ref<!qduc.lt, i1>
  qduc.end %lt
  "test.use"(%r) : (!qauc.ref<!qduc.lt, i1>) -> ()
  return
}
