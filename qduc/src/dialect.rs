use melior::Context;
use mlir_sys::MlirContext;

unsafe extern "C" {
    fn qduc_register_dialect(ctx: MlirContext);
}

/// Must be called before creating any qduc operations or types
pub fn register(context: &Context) {
    unsafe { qduc_register_dialect(context.to_raw()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melior::ir::{
        Block, Location, Module, Type, Value,
        block::BlockLike,
        operation::{OperationBuilder, OperationLike},
    };

    fn test_context() -> Context {
        let context = Context::new();
        context.set_allow_unregistered_dialects(true);
        register(&context);
        context
    }

    fn lt_type(context: &Context) -> Type<'_> {
        Type::parse(context, "!qduc.lt").expect("qduc registered")
    }

    /// A synthetic op (unregistered dialect, allowed above) that just uses
    /// `value` as an operand — standing in for any real op that would.
    fn append_use<'c>(block: &Block<'c>, value: Value<'c, '_>, location: Location<'c>) {
        block.append_operation(
            OperationBuilder::new("test.use", location)
                .add_operands(&[value])
                .build()
                .expect("valid unregistered test op"),
        );
    }

    fn append_newlft<'c, 'a>(context: &'c Context, block: &'a Block<'c>, location: Location<'c>) -> Value<'c, 'a> {
        let op = block.append_operation(
            OperationBuilder::new("qduc.newlft", location)
                .add_results(&[lt_type(context)])
                .build()
                .expect("valid qduc.newlft"),
        );
        op.result(0).expect("newlft has a result").into()
    }

    fn append_end<'c>(block: &Block<'c>, lifetime: Value<'c, '_>, location: Location<'c>) {
        block.append_operation(
            OperationBuilder::new("qduc.end", location)
                .add_operands(&[lifetime])
                .build()
                .expect("valid qduc.end"),
        );
    }

    #[test]
    fn dialect_loads_without_panic() {
        let context = Context::new();
        register(&context);
    }

    #[test]
    fn dialect_registers_ops() {
        let context = Context::new();
        let count_before = context.loaded_dialect_count();

        register(&context);

        assert_eq!(context.loaded_dialect_count(), count_before + 1);
        assert!(context.is_registered_operation("qduc.newlft"));
        assert!(context.is_registered_operation("qduc.end"));
    }

    #[test]
    fn newlft_use_then_end_verifies() {
        let context = test_context();
        let location = Location::unknown(&context);
        let module = Module::new(location);
        let block = module.body();

        let lt = append_newlft(&context, &block, location);
        append_use(&block, lt, location);
        append_end(&block, lt, location);

        assert!(module.as_operation().verify());
    }

    #[test]
    fn use_after_end_fails_verify() {
        let context = test_context();
        let location = Location::unknown(&context);
        let module = Module::new(location);
        let block = module.body();

        let lt = append_newlft(&context, &block, location);
        append_end(&block, lt, location);
        append_use(&block, lt, location);

        assert!(!module.as_operation().verify());
    }

    #[test]
    fn missing_end_fails_verify() {
        let context = test_context();
        let location = Location::unknown(&context);
        let module = Module::new(location);
        let block = module.body();

        append_newlft(&context, &block, location);

        assert!(!module.as_operation().verify());
    }

    #[test]
    fn crossing_lifetimes_verify() {
        let context = test_context();
        let location = Location::unknown(&context);
        let module = Module::new(location);
        let block = module.body();

        let lt_a = append_newlft(&context, &block, location);
        let lt_b = append_newlft(&context, &block, location);
        // 'a closes first; 'b opened before 'a closed and closes after — crossing, not nested.
        append_end(&block, lt_a, location);
        append_end(&block, lt_b, location);

        assert!(module.as_operation().verify());
    }
}
