#include "Qurts/Passes.h"

#include "Mlrd/MlrdDialect.h"
#include "Qauc/QaucDialect.h"
#include "Qduc/QducDialect.h"

#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/IR/Builders.h"

namespace qurts {
#define GEN_PASS_DEF_QURTSINFERDROPS
#include "QurtsPasses.h.inc"
} // namespace qurts

using namespace mlir;

namespace {

struct Lifetime {
  enum Kind { Static, Linear, Scoped } kind = Static;
  Value token;

  static Lifetime linear() { return {Linear, nullptr}; }
  static Lifetime scoped(Value token) { return {Scoped, token}; }

  Lifetime meet(Lifetime other) const {
    if (kind == Linear || other.kind == Linear) return linear();
    if (kind == Scoped) return *this;
    return other;
  }
};

class DropInference {
public:
  LogicalResult run(func::FuncOp func) {
    for (Block &block : func.getBody())
      if (failed(visit(block))) return failure();
    return success();
  }

private:
  DenseMap<Value, Lifetime> lifetimes_;

  Lifetime lifetimeOf(Value value) {
    if (auto it = lifetimes_.find(value); it != lifetimes_.end()) return it->second;
    Lifetime result = Lifetime::linear();
    if (Operation *def = value.getDefiningOp()) {
      if (auto lifted = dyn_cast<mlrd::LiftedOp>(def)) {
        result = Lifetime{};
        for (Value operand : lifted.getOperands()) result = result.meet(lifetimeOf(operand));
      } else if (auto qif = dyn_cast<mlrd::QifOp>(def)) {
        result = conditionLifetime(qif.getCondition());
      }
    }
    lifetimes_[value] = result;
    return result;
  }

  static Lifetime conditionLifetime(Value condition) {
    if (auto borrow = condition.getDefiningOp<qauc::BorrowOp>())
      return Lifetime::scoped(borrow.getLifetime());
    if (auto borrow = condition.getDefiningOp<qauc::UniqueBorrowOp>())
      return Lifetime::scoped(borrow.getLifetime());
    return Lifetime{};
  }

  static bool consumes(OpOperand &use) {
    return !isa<qauc::BorrowOp, qauc::UniqueBorrowOp>(use.getOwner());
  }

  static bool isUnconsumedQubit(Value value) {
    return isa<qauc::QubitType>(value.getType()) && llvm::none_of(value.getUses(), consumes);
  }

  LogicalResult visit(Block &block) {
    SmallVector<Value> dropped;
    for (BlockArgument argument : block.getArguments())
      if (isUnconsumedQubit(argument)) dropped.push_back(argument);
    for (Operation &op : block) {
      for (Region &region : op.getRegions())
        for (Block &nested : region)
          if (failed(visit(nested))) return failure();
      for (Value result : op.getResults())
        if (isUnconsumedQubit(result)) dropped.push_back(result);
    }

    for (Value value : dropped) {
      Lifetime lifetime = lifetimeOf(value);
      if (lifetime.kind == Lifetime::Linear)
        return emitError(value.getLoc(), "linear qubit is never consumed");
      Operation *before = block.mightHaveTerminator() ? block.getTerminator() : nullptr;
      if (lifetime.kind == Lifetime::Scoped) {
        qduc::EndOp end = qduc::findUniqueEnd(lifetime.token);
        if (end && end->getBlock() == &block) before = end;
      }
      OpBuilder builder = before ? OpBuilder(before) : OpBuilder::atBlockEnd(&block);
      qauc::UncomputeOp::create(builder, value.getLoc(), value);
    }
    return success();
  }
};

struct InferDrops : qurts::impl::QurtsInferDropsBase<InferDrops> {
  void runOnOperation() override {
    if (failed(DropInference().run(getOperation()))) signalPassFailure();
  }
};

} // namespace
