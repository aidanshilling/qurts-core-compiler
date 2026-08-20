// This is the derived-obligation case a qduc-only check can't see: %u's use
// (qauc.release) never appears in %lt's own use-list, so qduc's verifier
// alone would miss it — qauc::UniqueBorrowOp::verify() is what catches it.
//
// RUN: qauc-opt %s --verify-diagnostics

func.func @invalid(%v: i1) {
  %lt = qduc.newlft : !qduc.lt
  // expected-error @+1 {{'qauc.unique_borrow' op result used after its lifetime's qduc.end}}
  %u = qauc.unique_borrow %v, %lt : (i1, !qduc.lt) -> !qauc.unique<!qduc.lt, i1>
  qduc.end %lt
  qauc.release %u : !qauc.unique<!qduc.lt, i1>
  return
}
