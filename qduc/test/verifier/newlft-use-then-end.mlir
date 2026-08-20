// RUN: qduc-opt %s --allow-unregistered-dialect | FileCheck %s

// CHECK: %[[LT:.*]] = qduc.newlft : !qduc.lt
// CHECK: "test.use"(%[[LT]])
// CHECK: qduc.end %[[LT]]
%lt = qduc.newlft : !qduc.lt
"test.use"(%lt) : (!qduc.lt) -> ()
qduc.end %lt
