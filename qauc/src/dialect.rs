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
}
