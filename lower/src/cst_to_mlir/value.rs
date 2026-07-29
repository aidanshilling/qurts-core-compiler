use melior::ir::{Value, ValueLike};
use mlir_sys::MlirValue;
use std::marker::PhantomData;

/// A `Value` stored by its raw handle rather than a borrow-checked reference.
///
/// melior ties `Value<'c, 'a>`'s `'a` to the specific `Block` borrow that produced
/// it, which makes it unusable to store across nested-region boundaries (e.g. a
/// value from an `if`-branch's temporary block can't flow back into the outer
/// scope through a shared, single-lifetime `Env`). `MlirValue` is just a non-owning
/// pointer handle into MLIR's own heap-allocated IR tree, which outlives this
/// pass regardless of which Rust-local `Block` produced it, so re-deriving a
/// `Value<'c, 'c>` from the stored handle on demand is sound.
#[derive(Debug, Clone, Copy)]
pub struct StoredValue<'c> {
    raw: MlirValue,
    _context: PhantomData<&'c ()>,
}

impl<'c> StoredValue<'c> {
    pub fn new(value: impl ValueLike<'c>) -> Self {
        Self { raw: value.to_raw(), _context: PhantomData }
    }

    pub fn as_value(&self) -> Value<'c, 'c> {
        unsafe { Value::from_raw(self.raw) }
    }
}

#[derive(Debug, Clone)]
pub enum LoweredValue<'c> {
    Single(StoredValue<'c>),
    Tuple(Vec<LoweredValue<'c>>),
}

impl<'c> LoweredValue<'c> {
    pub fn single(value: impl ValueLike<'c>) -> Self {
        LoweredValue::Single(StoredValue::new(value))
    }

    pub fn flatten(&self) -> Vec<Value<'c, 'c>> {
        match self {
            LoweredValue::Single(value) => vec![value.as_value()],
            LoweredValue::Tuple(values) => values.iter().flat_map(Self::flatten).collect(),
        }
    }

    pub fn as_single(&self) -> Option<Value<'c, 'c>> {
        match self {
            LoweredValue::Single(value) => Some(value.as_value()),
            LoweredValue::Tuple(_) => None,
        }
    }
}

/// Structural shape of a source-level type: how many real MLIR values (and how
/// they nest back into tuples) a value of this type turns into.
#[derive(Debug, Clone)]
pub enum Shape {
    Unit,
    Single,
    Tuple(Vec<Shape>),
}

impl Shape {
    pub fn unflatten<'c>(
        &self,
        values: &mut impl Iterator<Item = Value<'c, 'c>>,
    ) -> LoweredValue<'c> {
        match self {
            Shape::Unit => LoweredValue::Tuple(vec![]),
            Shape::Single => {
                LoweredValue::single(values.next().expect("value count matches shape"))
            }
            Shape::Tuple(shapes) => {
                LoweredValue::Tuple(shapes.iter().map(|shape| shape.unflatten(values)).collect())
            }
        }
    }
}
