// RUN: qduc-opt %s --allow-unregistered-dialect --verify-diagnostics

// expected-error @+1 {{'qduc.newlft' op lifetime is never ended with a qduc.end}}
%lt = qduc.newlft : !qduc.lt
"test.use"(%lt) : (!qduc.lt) -> ()
