#include "Qurts/Passes.h"

#include "mqt/Dialect/QCO/IR/QCODialect.h"
#include "mqt/Dialect/QCO/IR/QCOInterfaces.h"
#include "mqt/Dialect/QCO/IR/QCOOps.h"

#include "mlir/IR/IRMapping.h"
#include "mlir/IR/OperationSupport.h"
#include "mlir/IR/PatternMatch.h"
#include "mlir/Transforms/GreedyPatternRewriteDriver.h"

namespace qurts {
#define GEN_PASS_DEF_QURTSCONTROLLEDCONJUGATION
#include "QurtsPasses.h.inc"
} // namespace qurts

using namespace mlir;

namespace {

bool isQubit(Value value) { return isa<qco::QubitType>(value.getType()); }

struct Conjugation {
  qco::InvOp inv;
  SmallVector<Operation *> prefix;
  SmallVector<unsigned> wires;
  SmallVector<Value> afterPrefix;
};

std::optional<unsigned> traceToArgument(Value value, Block &body, SmallVectorImpl<Operation *> &ops) {
  while (true) {
    if (auto argument = dyn_cast<BlockArgument>(value))
      return argument.getOwner() == &body ? std::optional(argument.getArgNumber()) : std::nullopt;
    auto unitary = dyn_cast<qco::UnitaryOpInterface>(value.getDefiningOp());
    if (!unitary) return std::nullopt;
    ops.push_back(unitary);
    value = unitary.getInputForOutput(value);
  }
}

std::optional<Conjugation> matchConjugation(qco::CtrlOp ctrl, qco::InvOp inv) {
  Block &body = *ctrl.getBody();
  Operation *yield = body.getTerminator();
  if (!llvm::all_of(inv.getResults(), [&](Value result) {
        return result.hasOneUse() && result.use_begin()->getOwner() == yield;
      }))
    return std::nullopt;

  Conjugation match{inv, {}, {}, {}};
  SmallVector<SmallVector<Operation *>> history;
  for (Value operand : inv.getQubitsIn()) {
    SmallVector<Operation *> ops;
    auto wire = traceToArgument(operand, body, ops);
    if (!wire) return std::nullopt;
    match.wires.push_back(*wire);
    std::reverse(ops.begin(), ops.end());
    history.push_back(std::move(ops));
  }

  DenseMap<Value, unsigned> invWire, bodyWire;
  Block &invBody = *inv.getBody();
  for (auto [i, argument] : llvm::enumerate(invBody.getArguments())) {
    invWire[argument] = i;
    bodyWire[body.getArgument(match.wires[i])] = i;
  }
  SmallVector<unsigned> cursor(history.size(), 0);
  auto equivalent = [](Value lhs, Value rhs) { return success(isQubit(lhs) || lhs == rhs); };

  for (Operation &u : invBody.without_terminator()) {
    SmallVector<unsigned> wires;
    for (Value in : u.getOperands()) {
      if (!isQubit(in)) continue;
      auto it = invWire.find(in);
      if (it == invWire.end()) return std::nullopt;
      wires.push_back(it->second);
    }
    if (wires.empty()) return std::nullopt;
    Operation *f = nullptr;
    for (unsigned wire : wires) {
      if (cursor[wire] >= history[wire].size()) return std::nullopt;
      Operation *candidate = history[wire][cursor[wire]];
      if (f && candidate != f) return std::nullopt;
      f = candidate;
    }
    if (f->getNumOperands() != u.getNumOperands() || f->getNumResults() != u.getNumResults()) return std::nullopt;
    for (auto [fin, uin] : llvm::zip(f->getOperands(), u.getOperands())) {
      if (isQubit(fin) != isQubit(uin)) return std::nullopt;
      if (!isQubit(fin)) continue;
      auto it = bodyWire.find(fin);
      if (it == bodyWire.end() || it->second != invWire[uin]) return std::nullopt;
    }
    if (!OperationEquivalence::isEquivalentTo(f, &u, equivalent, nullptr, OperationEquivalence::IgnoreLocations))
      return std::nullopt;
    for (unsigned wire : wires) ++cursor[wire];
    auto fUnitary = cast<qco::UnitaryOpInterface>(f), uUnitary = cast<qco::UnitaryOpInterface>(&u);
    for (Value fin : f->getOperands())
      if (isQubit(fin)) bodyWire[fUnitary.getOutputForInput(fin)] = bodyWire[fin];
    for (Value uin : u.getOperands())
      if (isQubit(uin)) invWire[uUnitary.getOutputForInput(uin)] = invWire[uin];
    match.prefix.push_back(f);
  }

  DenseSet<Operation *> prefix(match.prefix.begin(), match.prefix.end());
  for (unsigned wire : match.wires) {
    Value value = body.getArgument(wire);
    while (value.hasOneUse() && prefix.contains(value.use_begin()->getOwner()))
      value = cast<qco::UnitaryOpInterface>(value.use_begin()->getOwner()).getOutputForInput(value);
    match.afterPrefix.push_back(value);
  }
  return match;
}

struct HoistConjugation : OpRewritePattern<qco::CtrlOp> {
  using OpRewritePattern::OpRewritePattern;

  LogicalResult matchAndRewrite(qco::CtrlOp ctrl, PatternRewriter &rewriter) const override {
    Block &body = *ctrl.getBody();
    std::optional<Conjugation> match;
    for (auto inv : body.getOps<qco::InvOp>())
      if ((match = matchConjugation(ctrl, inv))) break;
    if (!match) return failure();

    IRMapping before;
    for (unsigned wire : match->wires) before.map(body.getArgument(wire), ctrl.getTargetsIn()[wire]);
    rewriter.setInsertionPoint(ctrl);
    for (Operation *op : match->prefix) rewriter.clone(*op, before);

    rewriter.setInsertionPointAfter(ctrl);
    IRMapping after;
    for (auto [operand, wire] : llvm::zip(match->inv.getQubitsIn(), match->wires))
      after.map(operand, ctrl.getTargetsOut()[wire]);
    Operation *hoistedInv = rewriter.clone(*match->inv, after);
    for (auto [i, wire] : llvm::enumerate(match->wires))
      rewriter.replaceUsesWithIf(ctrl.getTargetsOut()[wire], hoistedInv->getResult(i),
                                 [&](OpOperand &use) { return use.getOwner() != hoistedInv; });

    SmallVector<Value> targets(ctrl.getTargetsIn().begin(), ctrl.getTargetsIn().end());
    for (auto [i, wire] : llvm::enumerate(match->wires)) targets[wire] = before.lookup(match->afterPrefix[i]);
    rewriter.modifyOpInPlace(ctrl, [&] {
      ctrl.getTargetsInMutable().assign(targets);
      for (auto [i, wire] : llvm::enumerate(match->wires))
        if (!isa<BlockArgument>(match->afterPrefix[i]))
          rewriter.replaceAllUsesWith(match->afterPrefix[i], body.getArgument(wire));
      for (auto [in, out] : llvm::zip(match->inv.getQubitsIn(), match->inv.getQubitsOut()))
        rewriter.replaceAllUsesWith(out, in);
      rewriter.eraseOp(match->inv);
      for (Operation *op : llvm::reverse(match->prefix)) rewriter.eraseOp(op);
    });
    return success();
  }
};

struct ControlledConjugation : qurts::impl::QurtsControlledConjugationBase<ControlledConjugation> {
  void runOnOperation() override {
    RewritePatternSet patterns(&getContext());
    patterns.add<HoistConjugation>(&getContext());
    if (failed(applyPatternsGreedily(getOperation(), std::move(patterns)))) signalPassFailure();
  }
};

} // namespace
