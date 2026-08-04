use melior::Context;
use mlir_sys::MlirContext;

unsafe extern "C" {
    fn mlrd_register_dialect(ctx: MlirContext);
}

pub fn register(context: &Context) {
    unsafe { mlrd_register_dialect(context.to_raw()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melior::{
        dialect::DialectRegistry,
        ir::{
            Block, Location, Module, Region, RegionLike, Type, Value,
            block::BlockLike,
            operation::{OperationBuilder, OperationLike},
        },
        utility::register_all_dialects,
    };

    fn test_context() -> Context {
        let context = Context::new();
        let registry = DialectRegistry::new();
        register_all_dialects(&registry);
        context.append_dialect_registry(&registry);
        context.load_all_available_dialects();
        context.set_allow_unregistered_dialects(true);
        qduc::dialect::register(&context);
        qauc::dialect::register(&context);
        register(&context);
        context
    }

    fn append_opaque<'c, 'a>(
        context: &'c Context,
        block: &'a Block<'c>,
        name: &str,
        result_type: &str,
        location: Location<'c>,
    ) -> Value<'c, 'a> {
        let result_type = Type::parse(context, result_type).expect("type parses");
        let op = block.append_operation(
            OperationBuilder::new(name, location)
                .add_results(&[result_type])
                .build()
                .expect("valid unregistered test op"),
        );
        op.result(0).expect("test op has a result").into()
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

    fn append_borrow<'c, 'a>(
        context: &'c Context,
        block: &'a Block<'c>,
        value: Value<'c, '_>,
        lifetime: Value<'c, '_>,
        ref_type: &str,
        location: Location<'c>,
    ) -> Value<'c, 'a> {
        let ref_type = Type::parse(context, ref_type).expect("qauc registered");
        let op = block.append_operation(
            OperationBuilder::new("qauc.borrow", location)
                .add_operands(&[value, lifetime])
                .add_results(&[ref_type])
                .build()
                .expect("valid qauc.borrow"),
        );
        op.result(0).expect("borrow has a result").into()
    }

    fn empty_yield_region<'c>(location: Location<'c>) -> Region<'c> {
        let block = Block::new(&[]);
        block.append_operation(
            OperationBuilder::new("mlrd.yield", location).build().expect("valid mlrd.yield"),
        );
        let region = Region::new();
        region.append_block(block);
        region
    }

    fn append_qif<'c>(
        block: &Block<'c>,
        condition: Value<'c, '_>,
        then_region: Region<'c>,
        else_region: Region<'c>,
        location: Location<'c>,
    ) {
        block.append_operation(
            OperationBuilder::new("mlrd.qif", location)
                .add_operands(&[condition])
                .add_regions([then_region, else_region])
                .build()
                .expect("valid mlrd.qif"),
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
        assert!(context.is_registered_operation("mlrd.qif"));
        assert!(context.is_registered_operation("mlrd.lifted"));
    }

    #[test]
    fn qif_over_qbit_ref_verifies() {
        let context = test_context();
        let location = Location::unknown(&context);
        let module = Module::new(location);
        let block = module.body();

        let q = append_opaque(&context, &block, "test.qbit", "!qauc.qbit", location);
        let lt = append_newlft(&context, &block, location);
        let r = append_borrow(&context, &block, q, lt, "!qauc.ref<!qduc.lt, !qauc.qbit>", location);

        append_qif(
            &block,
            r,
            empty_yield_region(location),
            empty_yield_region(location),
            location,
        );
        append_end(&block, lt, location);

        assert!(module.as_operation().verify());
    }

    #[test]
    fn qif_over_non_qbit_ref_fails_verify() {
        let context = test_context();
        let location = Location::unknown(&context);
        let module = Module::new(location);
        let block = module.body();

        let value = append_opaque(&context, &block, "test.bool", "i1", location);
        let lt = append_newlft(&context, &block, location);
        let r = append_borrow(&context, &block, value, lt, "!qauc.ref<!qduc.lt, i1>", location);

        append_qif(
            &block,
            r,
            empty_yield_region(location),
            empty_yield_region(location),
            location,
        );
        append_end(&block, lt, location);

        assert!(!module.as_operation().verify());
    }
}
