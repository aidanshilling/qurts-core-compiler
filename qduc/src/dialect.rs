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

    #[test]
    fn dialect_loads_without_panic() {
        let context = Context::new();
        register(&context);
    }
}
