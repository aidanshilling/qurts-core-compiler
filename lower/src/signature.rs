use super::value::Shape;
use melior::ir::Type;

#[derive(Debug, Clone)]
pub struct FunctionSignature<'c> {
    pub param_names: Vec<String>,
    pub param_shapes: Vec<Shape>,
    pub param_types: Vec<Type<'c>>,
    pub result_types: Vec<Type<'c>>,
    pub result_shape: Shape,
}
