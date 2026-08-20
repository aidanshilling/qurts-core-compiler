// RUN: qduc-opt %s --allow-unregistered-dialect --verify-diagnostics

// expected-error @+1 {{'qduc.newlft' op lifetime used after its qduc.end}}
%lt = qduc.newlft : !qduc.lt
qduc.end %lt
"test.use"(%lt) : (!qduc.lt) -> ()
