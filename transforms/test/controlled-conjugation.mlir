// RUN: qurts-opt %s --qurts-controlled-conjugation | FileCheck %s

// ctrl(c){ H a; ctrl(a){ X t }; inv{ H a } }  ==>  H a; ctrl(c){ ctrl(a){ X t } }; inv{ H a }
// CHECK-LABEL: func.func @hoists_prefix_and_inverse
// CHECK:       %[[A1:.*]] = qco.h %[[A0:.*]] : !qco.qubit
// CHECK-NEXT:  %[[C1:.*]], %[[T:.*]]:2 = qco.ctrl(%{{.*}}) targets (%[[BA:.*]] = %[[A1]], %[[BT:.*]] = %{{.*}}) {
// CHECK-NEXT:    %{{.*}}, %{{.*}} = qco.ctrl(%[[BA]]) targets (%[[IT:.*]] = %[[BT]]) {
// CHECK-NEXT:      qco.x %[[IT]]
// CHECK-NOT:     qco.inv
// CHECK:       %{{.*}} = qco.inv (%[[IA:.*]] = %[[T]]#0) {
// CHECK-NEXT:    qco.h %[[IA]]
func.func @hoists_prefix_and_inverse(%c: !qco.qubit, %a: !qco.qubit, %t: !qco.qubit) -> (!qco.qubit, !qco.qubit, !qco.qubit) {
  %c1, %r:2 = qco.ctrl(%c) targets(%ba = %a, %bt = %t) {
    %a1 = qco.h %ba : !qco.qubit
    %a2, %t1 = qco.ctrl(%a1) targets(%it = %bt) {
      %it1 = qco.x %it : !qco.qubit
      qco.yield %it1 : !qco.qubit
    } : ({!qco.qubit}, {!qco.qubit})
    %a3 = qco.inv (%ia = %a2) {
      %ia1 = qco.h %ia : !qco.qubit
      qco.yield %ia1 : !qco.qubit
    } : {!qco.qubit} -> {!qco.qubit}
    qco.yield %a3, %t1 : !qco.qubit, !qco.qubit
  } : ({!qco.qubit}, {!qco.qubit, !qco.qubit})
  return %c1, %r#0, %r#1 : !qco.qubit, !qco.qubit, !qco.qubit
}

// The inverse does not match the body prefix (X vs H), so nothing moves.
// CHECK-LABEL: func.func @leaves_non_conjugation_alone
// CHECK:       qco.ctrl
// CHECK:         qco.h
// CHECK:         qco.inv
func.func @leaves_non_conjugation_alone(%c: !qco.qubit, %a: !qco.qubit) -> (!qco.qubit, !qco.qubit) {
  %c1, %r = qco.ctrl(%c) targets(%ba = %a) {
    %a1 = qco.h %ba : !qco.qubit
    %a2 = qco.inv (%ia = %a1) {
      %ia1 = qco.x %ia : !qco.qubit
      qco.yield %ia1 : !qco.qubit
    } : {!qco.qubit} -> {!qco.qubit}
    qco.yield %a2 : !qco.qubit
  } : ({!qco.qubit}, {!qco.qubit})
  return %c1, %r : !qco.qubit, !qco.qubit
}
