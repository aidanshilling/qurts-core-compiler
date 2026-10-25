// RUN: qurts-opt %s --convert-mlrd-to-qco --split-input-file --verify-diagnostics

func.func @different_wires(%a: !qauc.qbit, %b: !qauc.qbit, %r: !qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit {
  // expected-error @+1 {{qif branches yield different qubits in the same position}}
  %y = mlrd.qif %r {
    mlrd.yield %a : !qauc.qbit
  } else {
    mlrd.yield %b : !qauc.qbit
  } : (!qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit
  return %y : !qauc.qbit
}

// -----

func.func @fresh_in_one_branch(%a: !qauc.qbit, %r: !qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit {
  // expected-error @+1 {{qif yields a fresh qubit in only one branch}}
  %y = mlrd.qif %r {
    %z = mlrd.lifted "0"() : () -> !qauc.qbit
    mlrd.yield %z : !qauc.qbit
  } else {
    mlrd.yield %a : !qauc.qbit
  } : (!qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit
  return %y : !qauc.qbit
}

// -----

func.func @measure_under_control(%t: !qauc.qbit, %r: !qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit {
  %y = mlrd.qif %r {
    // expected-error @+1 {{measurement inside qif}}
    %b = mlrd.meas %t : !qauc.qbit -> i1
    %z = mlrd.lifted "0"() : () -> !qauc.qbit
    mlrd.yield %z : !qauc.qbit
  } else {
    %z = mlrd.lifted "0"() : () -> !qauc.qbit
    mlrd.yield %z : !qauc.qbit
  } : (!qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit
  return %y : !qauc.qbit
}
