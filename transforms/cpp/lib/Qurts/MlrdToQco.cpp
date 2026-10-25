#include "Qurts/Passes.h"

#include "Mlrd/MlrdDialect.h"
#include "Qauc/QaucDialect.h"
#include "Qduc/QducDialect.h"
#include "mqt/Dialect/MQT/IR/MQTDialect.h"
#include "mqt/Dialect/QCO/IR/QCODialect.h"
#include "mqt/Dialect/QCO/IR/QCOInterfaces.h"
#include "mqt/Dialect/QCO/IR/QCOOps.h"

#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/IR/Builders.h"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/IR/IRMapping.h"
#include "llvm/ADT/SetVector.h"
#include "llvm/ADT/StringSwitch.h"
#include "llvm/ADT/TypeSwitch.h"

namespace qurts {
#define GEN_PASS_DEF_CONVERTMLRDTOQCO
#include "QurtsPasses.h.inc"
} // namespace qurts

using namespace mlir;

namespace {

bool isQbit(Type type) { return isa<qauc::QubitType>(type); }

Type refValueType(Type type) {
  if (auto ref = dyn_cast<qauc::RefType>(type)) return ref.getValueType();
  if (auto unique = dyn_cast<qauc::UniqueType>(type)) return unique.getValueType();
  return nullptr;
}

bool isQuantum(Type type) {
  if (isQbit(type)) return true;
  Type value = refValueType(type);
  return value && isQbit(value);
}

bool isQubitValue(Value value) { return isa<qco::QubitType>(value.getType()); }

bool isAlloc(Operation *op) {
  auto lifted = dyn_cast<mlrd::LiftedOp>(op);
  return lifted && lifted.getOperands().empty();
}

template <typename OpT>
Value gate1(OpBuilder &builder, Location loc, Value in) {
  return OpT::create(builder, loc, in.getType(), in).getQubitOut();
}

using Gate1 = Value (*)(OpBuilder &, Location, Value);

Gate1 singleQubitGate(StringRef name) {
  return llvm::StringSwitch<Gate1>(name)
      .Cases({"X", "NOT"}, gate1<qco::XOp>)
      .Case("Y", gate1<qco::YOp>)
      .Case("Z", gate1<qco::ZOp>)
      .Case("H", gate1<qco::HOp>)
      .Case("S", gate1<qco::SOp>)
      .Case("SDG", gate1<qco::SdgOp>)
      .Case("T", gate1<qco::TOp>)
      .Case("TDG", gate1<qco::TdgOp>)
      .Case("SX", gate1<qco::SXOp>)
      .Case("SXDG", gate1<qco::SXdgOp>)
      .Default(nullptr);
}

struct ControlledGate {
  unsigned controls;
  StringRef base;
};

std::optional<ControlledGate> controlledGate(StringRef name) {
  return llvm::StringSwitch<std::optional<ControlledGate>>(name)
      .Cases({"CX", "CNOT"}, ControlledGate{1, "X"})
      .Case("CY", ControlledGate{1, "Y"})
      .Case("CZ", ControlledGate{1, "Z"})
      .Case("CH", ControlledGate{1, "H"})
      .Cases({"CCX", "CCNOT", "TOFFOLI"}, ControlledGate{2, "X"})
      .Case("CCZ", ControlledGate{2, "Z"})
      .Cases({"CSWAP", "FREDKIN"}, ControlledGate{1, "SWAP"})
      .Default(std::nullopt);
}

SmallVector<Value> swapGate(OpBuilder &builder, Location loc, Value a, Value b) {
  auto swap = qco::SWAPOp::create(builder, loc, a.getType(), b.getType(), a, b);
  return {swap.getQubit0Out(), swap.getQubit1Out()};
}

bool changesWire(qco::UnitaryOpInterface unitary, Value input);

bool argumentIsTarget(Block &body, Value argument) {
  Value value = argument;
  while (value.hasOneUse()) {
    Operation *user = value.use_begin()->getOwner();
    auto unitary = dyn_cast<qco::UnitaryOpInterface>(user);
    if (!unitary) return false;
    if (changesWire(unitary, value)) return true;
    value = unitary.getOutputForInput(value);
  }
  return false;
}

bool changesWire(qco::UnitaryOpInterface unitary, Value input) {
  if (llvm::is_contained(unitary.getInputControls(), input)) return false;
  Region *body = nullptr;
  if (auto ctrl = dyn_cast<qco::CtrlOp>(unitary.getOperation())) body = &ctrl.getRegion();
  else if (auto inv = dyn_cast<qco::InvOp>(unitary.getOperation())) body = &inv.getRegion();
  else if (auto pow = dyn_cast<qco::PowOp>(unitary.getOperation())) body = &pow.getRegion();
  if (!body) return true;
  auto targets = unitary.getInputTargets();
  auto position = llvm::find(targets, input);
  if (position == targets.end()) return true;
  return argumentIsTarget(body->front(), body->front().getArgument(position - targets.begin()));
}

class FunctionConverter {
public:
  FunctionConverter(func::FuncOp func, OpBuilder &builder)
      : func_(func), builder_(builder), qubit_(qco::QubitType::get(func.getContext())) {}

  FailureOr<func::FuncOp> convert() {
    if (!func_.getBody().hasOneBlock()) return func_.emitError("expected a single-block function body");
    FunctionType oldType = func_.getFunctionType();
    SmallVector<Type> inputs, results, appended;
    for (Type type : oldType.getInputs()) {
      if (isQuantum(type)) {
        inputs.push_back(qubit_);
        if (!isQbit(type)) appended.push_back(qubit_);
      } else if (Type value = refValueType(type)) {
        inputs.push_back(value);
      } else {
        inputs.push_back(type);
      }
    }
    for (Type type : oldType.getResults()) {
      if (isQbit(type)) results.push_back(qubit_);
      else if (isQuantum(type)) return func_.emitError("returning a qubit reference is not supported");
      else if (Type value = refValueType(type)) results.push_back(value);
      else results.push_back(type);
    }
    results.append(appended);

    auto newFunc = func::FuncOp::create(builder_, func_.getLoc(), func_.getName(),
                                        builder_.getFunctionType(inputs, results));
    newFunc.setVisibility(func_.getVisibility());
    Block *entry = newFunc.addEntryBlock();
    Block &oldEntry = func_.getBody().front();
    for (auto [oldArg, newArg] : llvm::zip(oldEntry.getArguments(), entry->getArguments())) {
      if (isQuantum(oldArg.getType())) {
        set(oldArg, newArg);
        if (!isQbit(oldArg.getType())) refParams_.push_back(oldArg);
      } else {
        classical_.map(oldArg, newArg);
      }
    }

    OpBuilder::InsertionGuard guard(builder_);
    builder_.setInsertionPointToStart(entry);
    for (Operation &op : oldEntry)
      if (failed(convertOp(&op)) || failed_) return failure();
    return newFunc;
  }

private:
  func::FuncOp func_;
  OpBuilder &builder_;
  Type qubit_;
  DenseMap<Value, Value> roots_;
  DenseMap<Value, Value> current_;
  DenseMap<Value, Value> wires_;
  IRMapping classical_;
  SmallVector<Value> refParams_;
  SmallVector<mlrd::QifOp> qifs_;
  SetVector<Value> pendingSinks_;
  unsigned depth_ = 0;
  bool failed_ = false;

  Value root(Value value) {
    Value result = value;
    if (auto it = roots_.find(value); it == roots_.end()) {
      if (Operation *def = value.getDefiningOp()) {
        unsigned index = cast<OpResult>(value).getResultNumber();
        if (auto unitary = dyn_cast<mlrd::UnitaryOp>(def)) result = root(unitary.getOperands()[index]);
        else if (auto lifted = dyn_cast<mlrd::LiftedOp>(def); lifted && !lifted.getOperands().empty())
          result = root(lifted.getOperands()[index]);
        else if (auto borrow = dyn_cast<qauc::BorrowOp>(def)) result = root(borrow.getValue());
        else if (auto borrow = dyn_cast<qauc::UniqueBorrowOp>(def)) result = root(borrow.getValue());
        else if (auto qif = dyn_cast<mlrd::QifOp>(def)) {
          if (failed(unifyYields(qif))) failed_ = true;
          if (auto unified = roots_.find(value); unified != roots_.end()) result = unified->second;
        }
      }
      roots_[value] = result;
    }
    for (auto it = roots_.find(result); it != roots_.end() && it->second != result; it = roots_.find(result))
      result = it->second;
    return result;
  }

  void set(Value identity, Value qco) {
    current_[identity] = qco;
    wires_[qco] = identity;
  }

  FailureOr<Value> get(Value value, Location loc) {
    auto it = current_.find(root(value));
    if (it == current_.end()) return emitError(loc, "qubit is no longer available here");
    return it->second;
  }

  static bool isFreshIn(Value identity, mlrd::QifOp qif) {
    Operation *def = identity.getDefiningOp();
    return def && isAlloc(def) && qif->isProperAncestor(def);
  }

  LogicalResult unifyYields(mlrd::QifOp qif) {
    auto thenYield = cast<mlrd::YieldOp>(qif.getThenRegion().front().getTerminator());
    auto elseYield = cast<mlrd::YieldOp>(qif.getElseRegion().front().getTerminator());
    for (auto [result, thenValue, elseValue] :
         llvm::zip(qif.getResults(), thenYield.getResults(), elseYield.getResults())) {
      if (!isQbit(result.getType())) return qif.emitError("qif may only yield qubits");
      Value thenRoot = root(thenValue), elseRoot = root(elseValue);
      bool thenFresh = isFreshIn(thenRoot, qif), elseFresh = isFreshIn(elseRoot, qif);
      if (thenFresh && elseFresh) {
        roots_[elseRoot] = thenRoot;
        roots_[result] = thenRoot;
      } else if (!thenFresh && !elseFresh) {
        if (thenRoot != elseRoot) return qif.emitError("qif branches yield different qubits in the same position");
        roots_[result] = thenRoot;
      } else {
        return qif.emitError("qif yields a fresh qubit in only one branch");
      }
    }
    return success();
  }

  LogicalResult convertOp(Operation *op) {
    return llvm::TypeSwitch<Operation *, LogicalResult>(op)
        .Case<qduc::NewLftOp, qduc::EndOp, qauc::ReleaseOp>([](auto) { return success(); })
        .Case<qauc::BorrowOp>([&](qauc::BorrowOp borrow) { return convertBorrow(borrow.getResult(), borrow.getValue()); })
        .Case<qauc::UniqueBorrowOp>([&](qauc::UniqueBorrowOp borrow) { return convertBorrow(borrow.getResult(), borrow.getValue()); })
        .Case<qauc::UncomputeOp>([&](qauc::UncomputeOp uncompute) { return convertUncompute(uncompute); })
        .Case<mlrd::LiftedOp>([&](mlrd::LiftedOp lifted) { return convertLifted(lifted); })
        .Case<mlrd::UnitaryOp>([&](mlrd::UnitaryOp unitary) {
          return convertGate(unitary, unitary.getGate(), unitary.getOperands(), unitary.getResults());
        })
        .Case<mlrd::MeasOp>([&](mlrd::MeasOp meas) { return convertMeas(meas); })
        .Case<mlrd::QifOp>([&](mlrd::QifOp qif) { return convertQif(qif); })
        .Case<func::ReturnOp>([&](func::ReturnOp ret) { return convertReturn(ret); })
        .Default([&](Operation *other) { return cloneClassical(other); });
  }

  LogicalResult convertBorrow(Value result, Value source) {
    if (isQuantum(source.getType())) return success();
    Value mapped = classical_.lookupOrNull(source);
    if (!mapped) return emitError(result.getLoc(), "borrowed value is not available");
    classical_.map(result, mapped);
    return success();
  }

  LogicalResult cloneClassical(Operation *op) {
    bool quantum = false;
    op->walk([&](Operation *nested) {
      quantum |= llvm::any_of(nested->getOperandTypes(), isQuantum) ||
                 llvm::any_of(nested->getResultTypes(), isQuantum);
    });
    if (quantum) return op->emitError("unsupported operation on quantum values");
    builder_.clone(*op, classical_);
    return success();
  }

  LogicalResult convertLifted(mlrd::LiftedOp lifted) {
    if (!lifted.getOperands().empty())
      return convertGate(lifted, lifted.getGate(), lifted.getOperands(), lifted.getResults());
    StringRef state = lifted.getGate();
    if (state != "0" && state != "1") return lifted.emitError("unsupported state preparation: ") << state;
    Value identity = root(lifted.getResults()[0]);
    if (!current_.contains(identity)) set(identity, qco::AllocOp::create(builder_, lifted.getLoc()).getResult());
    if (state == "1") set(identity, gate1<qco::XOp>(builder_, lifted.getLoc(), current_[identity]));
    return success();
  }

  LogicalResult convertGate(Operation *op, StringRef gate, ValueRange operands, ValueRange results) {
    Location loc = op->getLoc();
    SmallVector<Value> ins;
    for (Value operand : operands) {
      auto in = get(operand, loc);
      if (failed(in)) return failure();
      ins.push_back(*in);
    }
    std::string name = gate.upper();
    SmallVector<Value> outs;
    if (Gate1 make = singleQubitGate(name)) {
      if (ins.size() != 1) return op->emitError() << gate << " takes 1 qubit, got " << ins.size();
      outs.push_back(make(builder_, loc, ins[0]));
    } else if (name == "SWAP") {
      if (ins.size() != 2) return op->emitError() << gate << " takes 2 qubits, got " << ins.size();
      outs = swapGate(builder_, loc, ins[0], ins[1]);
    } else if (auto controlled = controlledGate(name)) {
      unsigned arity = controlled->controls + (controlled->base == "SWAP" ? 2 : 1);
      if (ins.size() != arity) return op->emitError() << gate << " takes " << arity << " qubits, got " << ins.size();
      ArrayRef<Value> all(ins);
      auto ctrl = qco::CtrlOp::create(
          builder_, loc, ValueRange(all.take_front(controlled->controls)),
          ValueRange(all.drop_front(controlled->controls)), [&](ValueRange args) -> SmallVector<Value> {
            if (controlled->base == "SWAP") return swapGate(builder_, loc, args[0], args[1]);
            return {singleQubitGate(controlled->base)(builder_, loc, args[0])};
          });
      outs.append(ctrl.getControlsOut().begin(), ctrl.getControlsOut().end());
      outs.append(ctrl.getTargetsOut().begin(), ctrl.getTargetsOut().end());
    } else {
      return op->emitError("unknown gate: ") << gate;
    }
    for (auto [operand, result, out] : llvm::zip(operands, results, outs)) {
      Value identity = root(operand);
      roots_[result] = identity;
      set(identity, out);
    }
    return success();
  }

  LogicalResult convertMeas(mlrd::MeasOp meas) {
    if (depth_ > 0) return meas.emitError("measurement inside qif");
    auto in = get(meas.getQbit(), meas.getLoc());
    if (failed(in)) return failure();
    auto measure = qco::MeasureOp::create(builder_, meas.getLoc(), in->getType(), builder_.getI1Type(), *in);
    qco::SinkOp::create(builder_, meas.getLoc(), measure.getQubitOut());
    current_.erase(root(meas.getQbit()));
    classical_.map(meas.getResult(), measure.getResult());
    return success();
  }

  LogicalResult convertReturn(func::ReturnOp ret) {
    SmallVector<Value> values;
    for (Value operand : ret.getOperands()) {
      if (isQuantum(operand.getType())) {
        auto value = get(operand, ret.getLoc());
        if (failed(value)) return failure();
        values.push_back(*value);
        current_.erase(root(operand));
      } else {
        Value value = classical_.lookupOrNull(operand);
        if (!value) return ret.emitError("returned value is not available");
        values.push_back(value);
      }
    }
    for (Value param : refParams_) {
      auto value = get(param, ret.getLoc());
      if (failed(value)) return failure();
      values.push_back(*value);
    }
    func::ReturnOp::create(builder_, ret.getLoc(), values);
    return success();
  }

  LogicalResult convertUncompute(qauc::UncomputeOp uncompute) {
    Location loc = uncompute.getLoc();
    Value identity = root(uncompute.getQbit());
    auto value = get(uncompute.getQbit(), loc);
    if (failed(value)) return failure();
    if (depth_ > 0 && !isFreshIn(identity, qifs_.back()))
      return uncompute.emitError("only qubits allocated inside the enclosing qif branch can be dropped there");

    SmallVector<Operation *> chain;
    Value wire = *value;
    while (!isa<BlockArgument>(wire)) {
      Operation *def = wire.getDefiningOp();
      if (isa<qco::AllocOp>(def)) break;
      auto unitary = dyn_cast<qco::UnitaryOpInterface>(def);
      if (!unitary) return uncompute.emitError("qubit history contains a non-unitary operation");
      wire = unitary.getInputForOutput(wire);
      if (changesWire(unitary, wire)) chain.push_back(def);
    }
    if (isa<BlockArgument>(wire) && depth_ == 0) return uncompute.emitError("cannot uncompute a function parameter");

    if (!chain.empty()) {
      SetVector<Value> wires;
      for (Operation *op : llvm::reverse(chain))
        for (Value in : op->getOperands())
          if (isQubitValue(in)) wires.insert(wires_.lookup(in));
      SmallVector<Value> operands;
      for (Value w : wires) {
        auto it = current_.find(w);
        if (it == current_.end()) return uncompute.emitError("a qubit this history depends on is no longer available");
        operands.push_back(it->second);
      }
      auto inv = qco::InvOp::create(builder_, loc, ValueRange(operands), [&](ValueRange args) -> SmallVector<Value> {
        DenseMap<Value, Value> bodyCurrent;
        for (auto [w, arg] : llvm::zip(wires, args)) bodyCurrent[w] = arg;
        for (Operation *op : llvm::reverse(chain)) {
          IRMapping mapping;
          for (Value in : op->getOperands())
            if (isQubitValue(in)) mapping.map(in, bodyCurrent[wires_.lookup(in)]);
          Operation *clone = builder_.clone(*op, mapping);
          for (auto [result, out] : llvm::zip(op->getResults(), clone->getResults()))
            if (isQubitValue(result)) bodyCurrent[wires_.lookup(result)] = out;
        }
        SmallVector<Value> outs;
        for (Value w : wires) outs.push_back(bodyCurrent[w]);
        return outs;
      });
      for (auto [w, out] : llvm::zip(wires, inv.getQubitsOut())) set(w, out);
    }

    if (depth_ == 0) {
      qco::SinkOp::create(builder_, loc, current_[identity]);
      current_.erase(identity);
    } else {
      pendingSinks_.insert(identity);
    }
    return success();
  }

  LogicalResult collectWires(Block &block, SetVector<Value> &wires) {
    for (Operation &op : block.without_terminator()) {
      if (isa<qduc::NewLftOp, qduc::EndOp, qauc::BorrowOp, qauc::UniqueBorrowOp, qauc::ReleaseOp>(op)) continue;
      if (auto nested = dyn_cast<mlrd::QifOp>(op)) {
        wires.insert(root(nested.getCondition()));
        if (failed(collectWires(nested.getThenRegion().front(), wires)) ||
            failed(collectWires(nested.getElseRegion().front(), wires)))
          return failure();
        continue;
      }
      if (isAlloc(&op)) {
        wires.insert(root(op.getResult(0)));
        continue;
      }
      if (isa<mlrd::MeasOp>(op)) return op.emitError("measurement inside qif");
      if (!isa<mlrd::UnitaryOp, mlrd::LiftedOp, qauc::UncomputeOp>(op)) {
        if (llvm::any_of(op.getOperandTypes(), isQuantum) || llvm::any_of(op.getResultTypes(), isQuantum))
          return op.emitError("unsupported operation inside qif");
        continue;
      }
      for (Value operand : op.getOperands())
        if (isQbit(operand.getType())) wires.insert(root(operand));
    }
    return success();
  }

  static bool hasGates(Block &block) {
    return llvm::any_of(block.without_terminator(), [](Operation &op) {
      if (auto lifted = dyn_cast<mlrd::LiftedOp>(op)) return !lifted.getOperands().empty() || lifted.getGate() == "1";
      if (auto qif = dyn_cast<mlrd::QifOp>(op))
        return hasGates(qif.getThenRegion().front()) || hasGates(qif.getElseRegion().front());
      return isa<mlrd::UnitaryOp, qauc::UncomputeOp>(op);
    });
  }

  LogicalResult emitBranch(Block &block, const SetVector<Value> &wires, Value control, Location loc) {
    if (wires.empty() || !hasGates(block)) return success();
    SmallVector<Value> targets;
    for (Value wire : wires) {
      auto it = current_.find(wire);
      if (it == current_.end()) return emitError(loc, "qubit used in qif branch is no longer available");
      targets.push_back(it->second);
    }
    bool ok = true;
    auto ctrl = qco::CtrlOp::create(
        builder_, loc, ValueRange(current_[control]), ValueRange(targets), [&](ValueRange args) -> SmallVector<Value> {
          for (auto [wire, arg] : llvm::zip(wires, args)) set(wire, arg);
          ++depth_;
          for (Operation &op : block.without_terminator())
            if (failed(convertOp(&op)) || failed_) {
              ok = false;
              break;
            }
          --depth_;
          SmallVector<Value> outs;
          for (Value wire : wires) outs.push_back(current_.lookup(wire));
          return outs;
        });
    if (!ok) return failure();
    set(control, ctrl.getControlsOut()[0]);
    for (auto [wire, out] : llvm::zip(wires, ctrl.getTargetsOut())) set(wire, out);
    return success();
  }

  LogicalResult convertQif(mlrd::QifOp qif) {
    Location loc = qif.getLoc();
    Value control = root(qif.getCondition());
    if (!isQuantum(control.getType())) return qif.emitError("qif condition must borrow a qubit");
    if (failed(unifyYields(qif))) return failure();
    SetVector<Value> thenWires, elseWires;
    if (failed(collectWires(qif.getThenRegion().front(), thenWires)) ||
        failed(collectWires(qif.getElseRegion().front(), elseWires)) || failed_)
      return failure();
    if (thenWires.contains(control) || elseWires.contains(control))
      return qif.emitError("qif branch uses its own control qubit");
    for (Value wire : llvm::concat<const Value>(thenWires.getArrayRef(), elseWires.getArrayRef()))
      if (isFreshIn(wire, qif) && !current_.contains(wire))
        set(wire, qco::AllocOp::create(builder_, loc).getResult());

    qifs_.push_back(qif);
    if (failed(emitBranch(qif.getThenRegion().front(), thenWires, control, loc))) return failure();
    if (!elseWires.empty() && hasGates(qif.getElseRegion().front())) {
      set(control, gate1<qco::XOp>(builder_, loc, current_[control]));
      if (failed(emitBranch(qif.getElseRegion().front(), elseWires, control, loc))) return failure();
      set(control, gate1<qco::XOp>(builder_, loc, current_[control]));
    }
    qifs_.pop_back();

    if (depth_ == 0) {
      for (Value identity : pendingSinks_) {
        qco::SinkOp::create(builder_, loc, current_[identity]);
        current_.erase(identity);
      }
      pendingSinks_.clear();
    }
    return success();
  }
};

bool isQubitType(Type type) { return isa<qco::QubitType>(type); }

struct ConvertMlrdToQco : qurts::impl::ConvertMlrdToQcoBase<ConvertMlrdToQco> {
  void runOnOperation() override {
    ModuleOp module = getOperation();
    OpBuilder builder(module.getContext());
    SmallVector<func::FuncOp> programs;
    for (auto func : llvm::make_early_inc_range(module.getOps<func::FuncOp>())) {
      if (func.isExternal()) continue;
      builder.setInsertionPoint(func);
      FunctionConverter converter(func, builder);
      auto converted = converter.convert();
      if (failed(converted)) return signalPassFailure();
      func.erase();
      FunctionType type = converted->getFunctionType();
      if (llvm::none_of(type.getInputs(), isQubitType) && llvm::none_of(type.getResults(), isQubitType))
        programs.push_back(*converted);
    }
    auto main = llvm::find_if(programs, [](func::FuncOp f) { return f.getName() == "main"; });
    if (main != programs.end()) mqt::setEntryPoint(*main);
    else if (programs.size() == 1) mqt::setEntryPoint(programs.front());
    if (failed(mqt::verifyQuantumAllocations(module))) signalPassFailure();
  }
};

} // namespace
