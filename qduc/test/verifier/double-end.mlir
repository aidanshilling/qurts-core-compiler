// RUN: qduc-opt %s --verify-diagnostics

// expected-error @+1 {{'qduc.newlft' op lifetime is ended more than once}}
%lt = qduc.newlft : !qduc.lt
qduc.end %lt
qduc.end %lt
