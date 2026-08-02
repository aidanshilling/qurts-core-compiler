// Crossing, non-nested spans are the whole point of flat tokens over a
// region-based design — see the "Lifetimes are flat ... tokens" note in
// CLAUDE.md. 'a opens first but closes first too; 'b opens while 'a is
// still open and closes after 'a. Neither is nested inside the other.
//
// RUN: qduc-opt %s | FileCheck %s

// CHECK: %[[A:.*]] = qduc.newlft : !qduc.lt
// CHECK: %[[B:.*]] = qduc.newlft : !qduc.lt
// CHECK: qduc.end %[[A]]
// CHECK: qduc.end %[[B]]
%a = qduc.newlft : !qduc.lt
%b = qduc.newlft : !qduc.lt
qduc.end %a
qduc.end %b
