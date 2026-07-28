#ifndef QAUC_TD
#define QAUC_TD

include "mlir/IR/AttrTypeBase.td"
include "mlir/IR/BuiltinAttributes.td"
include "mlir/IR/EnumAttr.td"
include "mlir/IR/OpBase.td"
include "mlir/Interfaces/SideEffectInterfaces.td"
// Path assumes the repo root is on the tablegen include path; see
// qauc/cpp/CMakeLists.txt, which adds the repo root as an include dir.
include "qduc/td/qduc.td"

// This file defines the tablegen for the QAUC dialect.

def Qauc_Dialect : Dialect {
  let name = "qauc";
  let summary = "Qurts Automatic Uncomputation Dialect";
  let description = [{
    qauc bridges the qduc lifetime system with quantum resource tracking. It
    enforces the sub-structural rules that make correct quantum
    uncomputation statically verifiable.
  }];
  let cppNamespace = "::qauc";
  let useDefaultTypePrinterParser = 1;
}

class Qauc_Type<string name, string mnemonic_>
    : TypeDef<Qauc_Dialect, name> {
  let mnemonic = mnemonic_;
}

def Qauc_QubitType : Qauc_Type<"Qubit", "qbit"> {
  let summary = "quantum bit";
  let description = [{
    A linear quantum bit value; cannot be copied (no-cloning theorem) and
    cannot be implicitly discarded. A qubit obtained via `qauc.unique_borrow`
    must be returned to |0> (marked by `qauc.uncompute`) before its lifetime
    ends.
  }];
}

def Qauc_UniqueType : Qauc_Type<"Unique", "unique"> {
  let summary = "unique reference, `#'a T`";
  let description = [{
    A unique reference to a value of type `valueType`, scoped to lifetime
    `lifetime`. Must be consumed exactly once (by `qauc.release`) before
    `lifetime` ends. This is the mechanism for tracking qubits you own: a
    `!qauc.unique<lt, !qauc.qbit>` obligates the holder to uncompute the
    qubit by the time `lt` is closed.
  }];
  let parameters = (ins
    "::qduc::LifetimeType":$lifetime,
    "::mlir::Type":$valueType
  );
  let assemblyFormat = "`<` $lifetime `,` $valueType `>`";
}

def Qauc_RefType : Qauc_Type<"Ref", "ref"> {
  let summary = "shared reference, `&'a T`";
  let description = [{
    A shared, classically-borrowed reference to a value of type `valueType`,
    scoped to lifetime `lifetime`. Carries no ownership obligation.
  }];
  let parameters = (ins
    "::qduc::LifetimeType":$lifetime,
    "::mlir::Type":$valueType
  );
  let assemblyFormat = "`<` $lifetime `,` $valueType `>`";
}

// Resources (for MemoryEffectsOpInterface)

def Qauc_QubitResource : Resource<"QubitResource"> {}

// ----- Ops -----
class Qauc_Op<string mnemonic, list<Trait> traits = []>
    : Op<Qauc_Dialect, mnemonic, traits>;

def Qauc_BorrowOp : Qauc_Op<"borrow"> {
  let summary = "creates a !qauc.ref from a value and a lifetime token";
  let description = [{
    Creates a `!qauc.ref<lt, T>` classically borrowing `$value` for
    `$lifetime`.
  }];
  let arguments = (ins AnyType:$value, Qduc_LifetimeType:$lifetime);
  let results = (outs Qauc_RefType:$result);
  let assemblyFormat = [{
    $value `,` $lifetime attr-dict `:` functional-type(operands, results)
  }];
}

def Qauc_UniqueBorrowOp : Qauc_Op<"unique_borrow"> {
  let summary = "creates a !qauc.unique from a value and a lifetime token";
  let description = [{
    Creates a `!qauc.unique<lt, T>` that obligates `$value` to `$lifetime`;
    the result must be consumed exactly once (by `qauc.release`) before
    `$lifetime` ends.
  }];
  let arguments = (ins AnyType:$value, Qduc_LifetimeType:$lifetime);
  let results = (outs Qauc_UniqueType:$result);
  let assemblyFormat = [{
    $value `,` $lifetime attr-dict `:` functional-type(operands, results)
  }];
}

def Qauc_ReleaseOp : Qauc_Op<"release"> {
  let summary = "consumes a !qauc.unique, discharging the ownership obligation";
  let description = [{
    Consumes `$input`, releasing the ownership obligation created by
    `qauc.unique_borrow`. `$input` must not be used again after this op.
  }];
  let arguments = (ins Qauc_UniqueType:$input);
  let assemblyFormat = "$input attr-dict `:` type($input)";
}

def Qauc_UncomputeOp : Qauc_Op<"uncompute", [
    DeclareOpInterfaceMethods<MemoryEffectsOpInterface>
]> {
  let summary = "marks the point at which a qbit must have been returned to |0>";
  let description = [{
    Marks that `$qbit` must be back in the |0> state at this point in the
    program.

    Implements MemoryEffectsOpInterface with Free<QubitResource>, 
    so that generic optimization passes cannot eliminate it as dead code.
  }];
  let arguments = (ins Qauc_QubitType:$qbit);
  let assemblyFormat = "$qbit attr-dict";
}

#endif // QAUC_TD
