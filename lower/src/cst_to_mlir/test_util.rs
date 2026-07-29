use melior::{Context, dialect::DialectRegistry, utility::register_all_dialects};

pub fn test_context() -> Context {
    let context = Context::new();
    let registry = DialectRegistry::new();
    register_all_dialects(&registry);
    context.append_dialect_registry(&registry);
    context.load_all_available_dialects();
    qduc::dialect::register(&context);
    qauc::dialect::register(&context);
    context
}
