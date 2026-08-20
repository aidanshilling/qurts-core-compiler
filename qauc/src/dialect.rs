// qauc dialect registration
use melior::Context;
use mlir_sys::MlirContext;

unsafe extern "C" {
    fn qauc_register_dialect(ctx: MlirContext);
}

pub fn register(context: &Context) {
    unsafe { qauc_register_dialect(context.to_raw()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melior::{
        dialect::{DialectRegistry, arith},
        ir::{
            Block, Location, Module, Type, Value, ValueLike,
            attribute::IntegerAttribute,
            block::BlockLike,
            operation::{OperationBuilder, OperationLike},
            r#type::IntegerType,
        },
        utility::register_all_dialects,
    };

    fn test_context() -> Context {
        let context = Context::new();
        let registry = DialectRegistry::new();
        register_all_dialects(&registry);
        context.append_dialect_registry(&registry);
        context.load_all_available_dialects();
        qduc::dialect::register(&context);
        register(&context);
        context
    }

    fn append_bool<'c, 'a>(context: &'c Context, block: &'a Block<'c>, location: Location<'c>) -> Value<'c, 'a> {
        let attr = IntegerAttribute::new(IntegerType::new(context, 1).into(), 1);
        let op = block.append_operation(arith::constant(context, attr.into(), location));
        op.result(0).expect("arith.constant has a result").into()
    }

    fn append_newlft<'c, 'a>(context: &'c Context, block: &'a Block<'c>, location: Location<'c>) -> Value<'c, 'a> {
        let lt_type = Type::parse(context, "!qduc.lt").expect("qduc registered");
        let op = block.append_operation(
            OperationBuilder::new("qduc.newlft", location)
                .add_results(&[lt_type])
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

    fn append_unique_borrow<'c, 'a>(
        context: &'c Context,
        block: &'a Block<'c>,
        value: Value<'c, '_>,
        lifetime: Value<'c, '_>,
        location: Location<'c>,
    ) -> Value<'c, 'a> {
        let unique_type = Type::parse(context, &format!("!qauc.unique<!qduc.lt, {}>", value.r#type()))
            .expect("qauc registered");
        let op = block.append_operation(
            OperationBuilder::new("qauc.unique_borrow", location)
                .add_operands(&[value, lifetime])
                .add_results(&[unique_type])
                .build()
                .expect("valid qauc.unique_borrow"),
        );
        op.result(0).expect("unique_borrow has a result").into()
    }

    fn append_release<'c>(block: &Block<'c>, unique: Value<'c, '_>, location: Location<'c>) {
        block.append_operation(
            OperationBuilder::new("qauc.release", location)
                .add_operands(&[unique])
                .build()
                .expect("valid qauc.release"),
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
        assert!(context.is_registered_operation("qauc.borrow"));
        assert!(context.is_registered_operation("qauc.unique_borrow"));
        assert!(context.is_registered_operation("qauc.release"));
        assert!(context.is_registered_operation("qauc.uncompute"));
    }

    #[test]
    fn unique_borrow_release_before_end_verifies() {
        let context = test_context();
        let location = Location::unknown(&context);
        let module = Module::new(location);
        let block = module.body();

        let value = append_bool(&context, &block, location);
        let lt = append_newlft(&context, &block, location);
        let unique = append_unique_borrow(&context, &block, value, lt, location);
        append_release(&block, unique, location);
        append_end(&block, lt, location);

        assert!(module.as_operation().verify());
    }

    #[test]
    fn unique_borrow_release_after_end_fails_verify() {
        let context = test_context();
        let location = Location::unknown(&context);
        let module = Module::new(location);
        let block = module.body();

        let value = append_bool(&context, &block, location);
        let lt = append_newlft(&context, &block, location);
        let unique = append_unique_borrow(&context, &block, value, lt, location);
        append_end(&block, lt, location);
        append_release(&block, unique, location);

        assert!(!module.as_operation().verify());
    }
}
