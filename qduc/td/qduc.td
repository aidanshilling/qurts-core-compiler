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

def Qduc_RegionOp : Qduc_Op<"region"> {
  let summary = "Opens a lifetime scope";
  let description = [{ Opens a new lifetime scope.  }];
  let regions = (region SizedRegion<1>:$body);
  let assemblyFormat = "$body attr-dict";
  let skipDefaultBuilders = 1;
  let builders = [
    OpBuilder<(ins), [{
      mlir::Region *body = $_state.addRegion();
      mlir::Block *entry = new mlir::Block();
      entry->addArgument(LifetimeType::get($_builder.getContext()),
                         $_state.location);
      body->push_back(entry);
    }]>
  ];
}

def Qduc_EndOp : Qduc_Op<"end", [
    Terminator,
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
