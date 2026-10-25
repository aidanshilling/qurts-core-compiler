// RUN: qurts-opt %s --convert-mlrd-to-qco | FileCheck %s

// Both branches allocate: the two fresh qubits are one hoisted qco.alloc, the
// then-branch's [1] becomes a controlled X, and the gate-free else branch emits
// nothing. Dropping the result before qduc.end inverts the controlled history
// (control included, as the current control value) and sinks the qubit.
// CHECK-LABEL: func.func @main
// CHECK-SAME:  attributes {mqt.entry_point}
// CHECK:       %[[C0:.*]] = qco.alloc
// CHECK-NEXT:  %[[C:.*]] = qco.h %[[C0]]
// CHECK-NEXT:  %[[Z:.*]] = qco.alloc
// CHECK-NEXT:  %[[C1:.*]], %[[Z1:.*]] = qco.ctrl(%[[C]]) targets (%[[BZ:.*]] = %[[Z]]) {
// CHECK-NEXT:    qco.x %[[BZ]]
// CHECK:       }
// CHECK-NEXT:  %[[I0:.*]], %[[I1:.*]] = qco.inv (%{{.*}} = %[[C1]], %{{.*}} = %[[Z1]]) {
// CHECK-NEXT:    qco.ctrl
// CHECK:       qco.sink %[[I1]]
// CHECK-NEXT:  %{{.*}}, %[[B:.*]] = qco.measure %[[I0]]
// CHECK-NEXT:  qco.sink
// CHECK-NEXT:  return %[[B]]
func.func @main() -> i1 {
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
  qauc.uncompute %y
  qduc.end %lt
  %b = mlrd.meas %1 : !qauc.qbit -> i1
  return %b : i1
}

// A borrowed qubit parameter is handed back as a trailing result (MQT helper
// contract), and the else branch here does real work.
// CHECK-LABEL: func.func @helper
// CHECK-SAME:  (%[[R:.*]]: !qco.qubit, %[[T:.*]]: !qco.qubit) -> (!qco.qubit, !qco.qubit)
// CHECK:       %[[R1:.*]], %[[T1:.*]] = qco.ctrl(%[[R]]) targets (%{{.*}} = %[[T]])
// CHECK-NEXT:    qco.x
// CHECK:       %[[RX:.*]] = qco.x %[[R1]]
// CHECK-NEXT:  %[[R2:.*]], %[[T2:.*]] = qco.ctrl(%[[RX]]) targets (%{{.*}} = %[[T1]])
// CHECK-NEXT:    qco.h
// CHECK:       %[[R3:.*]] = qco.x %[[R2]]
// CHECK-NEXT:  return %[[T2]], %[[R3]]
func.func @helper(%r: !qauc.ref<!qduc.lt, !qauc.qbit>, %t: !qauc.qbit) -> !qauc.qbit {
  %y = mlrd.qif %r {
    %t1 = mlrd.lifted "not"(%t) : (!qauc.qbit) -> !qauc.qbit
    mlrd.yield %t1 : !qauc.qbit
  } else {
    %t1 = mlrd.unitary "H"(%t) : (!qauc.qbit) -> !qauc.qbit
    mlrd.yield %t1 : !qauc.qbit
  } : (!qauc.ref<!qduc.lt, !qauc.qbit>) -> !qauc.qbit
  return %y : !qauc.qbit
}
