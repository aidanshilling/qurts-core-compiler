#ifndef QDUC_TD
#define QDUC_TD

include "mlir/IR/AttrTypeBase.td"
include "mlir/IR/BuiltinAttributes.td"
include "mlir/IR/EnumAttr.td"
include "mlir/IR/OpBase.td"
include "mlir/Interfaces/SideEffectInterfaces.td"

// This file defines the tablegen for the QDUC dialect.

def Qduc_Dialect : Dialect {
  let name = "qduc";
  let summary = "Lifetime and region constraint dialect for qurts";
  let description = [{
    qduc provides SSA lifetime tokens and the ops that open and close lifetime
    scopes.
  }];
  let cppNamespace = "::qduc";
  let useDefaultTypePrinterParser = 1;
  let useDefaultAttributePrinterParser = 1;
}

class Qduc_Type<string name, string mnemonic_>
    : TypeDef<Qduc_Dialect, name> {
  let mnemonic = mnemonic_;
}

def Qduc_LifetimeType : Qduc_Type<"Lifetime", "lt"> {
  let summary = "SSA lifetime token";
  let description = [{
    Represents an open lifetime scope as a first-class SSA value.
  }];
}

// Resources (for MemoryEffectsOpInterface)

def Qduc_LifetimeResource : Resource<"LifetimeResource"> {}


// ----- Ops -----
class Qduc_Op<string mnemonic, list<Trait> traits = []>
    : Op<Qduc_Dialect, mnemonic, traits>;

def Qduc_NewLftOp : Qduc_Op<"newlft"> {
  let summary = "opens a new lifetime scope, producing its token";
  let description = [{
    Produces a fresh `!qduc.lt` token as an ordinary op result. Lifetimes are
    not structurally nested here (unlike a region-based design) because
    source-level lifetimes are only required to be minimal in their ordering
    preorder, not stack-disciplined — two lifetimes with no ordering
    constraint between them may have crossing newlft/endlft spans. This op's
    verifier enforces: exactly one `qduc.end` consumes the result, and every
    other use of it precedes that `qduc.end` in the same block.
  }];
  let results = (outs Qduc_LifetimeType:$lifetime);
  let assemblyFormat = "attr-dict `:` type($lifetime)";
  let hasVerifier = 1;
}

def Qduc_EndOp : Qduc_Op<"end", [
    DeclareOpInterfaceMethods<MemoryEffectsOpInterface>
]> {
  let summary = "Closes a lifetime scope";
  let description = [{
    Closes lifetime scope and consumes token.

    Implements MemoryEffectsOpInterface with Free<LifetimeResource> so that
    generic optimization passes that run at the QIR level cannot eliminate it.
  }];
  let arguments = (ins Qduc_LifetimeType:$lifetime);
  let assemblyFormat = "$lifetime attr-dict";
}


// ----- Attributes -----
def Qduc_OrderingKind : I32EnumAttr<"OrderingKind",
    "lifetime ordering constraint kind", [
  I32EnumAttrCase<"LessEq", 0, "le">,
  I32EnumAttrCase<"NotEq",  1, "ne">
]> {
  let cppNamespace = "::qduc";
}

def Qduc_OrderingConstraintAttr
    : AttrDef<Qduc_Dialect, "OrderingConstraint"> {
  let mnemonic = "ordering";
  let summary = "Encodes a lifetime ordering constraint on a function signature";
  let description = [{
    Represents a constraint between two lifetime parameters of a function.
    `lhs` and `rhs` are indices into the function's lifetime parameter list.

    Kinds:
      le  — lhs must end before or at the same point as rhs  ('a <= 'b)
      ne  — lhs and rhs must not be the same lifetime         ('a != 'b)
  }];
  let parameters = (ins
    "::qduc::OrderingKind":$kind,
    "unsigned":$lhs,
    "unsigned":$rhs
  );
  let assemblyFormat = "`<` $kind `,` $lhs `,` $rhs `>`";
}

#endif // QDUC_TD
