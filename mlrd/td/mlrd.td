#ifndef MLRD_TD
#define MLRD_TD

include "mlir/IR/AttrTypeBase.td"
include "mlir/IR/BuiltinAttributes.td"
include "mlir/IR/OpBase.td"
include "mlir/Interfaces/SideEffectInterfaces.td"
// Path assumes the repo root is on the tablegen include path; see
// mlrd/cpp/CMakeLists.txt, which adds the repo root as an include dir.
include "qauc/td/qauc.td"

// This file defines the tablegen for the MLRD dialect: the actual operations
// a qurts program performs (gate application, quantum conditionals, ...),
// as opposed to qduc (lifetime tokens) and qauc (sub-structural ownership
// bookkeeping) — mlrd depends on qauc for its qubit/ref/unique types.

def Mlrd_Dialect : Dialect {
  let name = "mlrd";
  let summary = "Qurts program operations dialect";
  let description = [{
    mlrd holds the operations a qurts program performs — gate application,
    quantum conditionals, and (later) measurement — separate from qduc's
    lifetime tokens and qauc's ownership/obligation bookkeeping.
  }];
  let cppNamespace = "::mlrd";
}

class Mlrd_Op<string mnemonic, list<Trait> traits = []>
    : Op<Mlrd_Dialect, mnemonic, traits>;

def Mlrd_LiftedOp : Mlrd_Op<"lifted"> {
  let summary = "opaque application of a lifted (lifetime-preserving) gate";
  let description = [{
    Applies the injective classical function named `$gate` (lifted to a
    quantum isometry, e.g. state prep, X, CX) to `$operands`, producing one
    result per operand — or a single fresh qubit for the nullary `[0]`/`[1]`.
    A placeholder: carries the surface-level `[c](x)` information for the
    mlrd-to-qco conversion to resolve into real gate semantics — this op does
    not itself define what `$gate` computes.
  }];
  let arguments = (ins StrAttr:$gate, Variadic<Qauc_QubitType>:$operands);
  let results = (outs Variadic<Qauc_QubitType>:$results);
  let assemblyFormat = [{
    $gate `(` $operands `)` attr-dict `:` functional-type(operands, results)
  }];
  let hasVerifier = 1;
}

def Mlrd_UnitaryOp : Mlrd_Op<"unitary"> {
  let summary = "application of a named unitary gate to one or more qubits";
  let description = [{
    Applies the gate named `$gate` (`H`, `X`, `CX`, ...) to `$operands`,
    producing one result per operand in the same order. Like `mlrd.lifted`
    this only carries the surface-level name; the mlrd-to-qco conversion
    resolves it into real gate semantics.
  }];
  let arguments = (ins StrAttr:$gate, Variadic<Qauc_QubitType>:$operands);
  let results = (outs Variadic<Qauc_QubitType>:$results);
  let assemblyFormat = [{
    $gate `(` $operands `)` attr-dict `:` functional-type(operands, results)
  }];
  let hasVerifier = 1;
}

def Mlrd_MeasOp : Mlrd_Op<"meas"> {
  let summary = "computational-basis measurement, consuming the qubit";
  let arguments = (ins Qauc_QubitType:$qbit);
  let results = (outs I1:$result);
  let assemblyFormat = "$qbit attr-dict `:` type($qbit) `->` type($result)";
}

def Mlrd_YieldOp : Mlrd_Op<"yield", [Terminator, HasParent<"QifOp">]> {
  let summary = "terminates a mlrd.qif branch region";
  let description = [{
    `scf.yield` can't be reused here — its own verifier hardcodes an allowed-
    parent list (`scf.if`/`scf.for`/...) that doesn't include `mlrd.qif`, so
    it rejects being nested inside one. Same shape as `scf.yield` otherwise.
  }];
  let arguments = (ins Variadic<AnyType>:$results);
  let assemblyFormat = "attr-dict ($results^ `:` type($results))?";
}

def Mlrd_QifOp : Mlrd_Op<"qif"> {
  let summary = "coherent quantum conditional over a borrowed control qubit";
  let description = [{
    Coherently applies `$thenRegion` or `$elseRegion` controlled on
    `$condition`'s qubit state. Unlike `scf.if`, this isn't a branch: both
    regions unitarily contribute, neither is "not executed". `$condition`
    must be a `!qauc.ref<lt, !qauc.qbit>` — checked by this op's verifier,
    since `Qauc_RefType` is generic over `T`. Each region must be a single
    block ending in `mlrd.yield`, with operand types matching `$results`.

    Not yet verified: that both regions discharge the same lifetime/
    uniqueness obligations (open the same lifetimes, release the same
    unique values) — see mlrd/README.md.
  }];
  let arguments = (ins Qauc_RefType:$condition);
  let results = (outs Variadic<AnyType>:$results);
  let regions = (region SizedRegion<1>:$thenRegion, SizedRegion<1>:$elseRegion);
  let assemblyFormat = [{
    $condition $thenRegion `else` $elseRegion attr-dict `:` functional-type($condition, results)
  }];
  let hasVerifier = 1;
}

#endif // MLRD_TD
