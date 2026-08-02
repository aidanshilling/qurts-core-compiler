use super::value::LoweredValue;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Env<'c> {
    scopes: Vec<HashMap<String, LoweredValue<'c>>>,
}

impl<'c> Env<'c> {
    pub fn new() -> Self {
        Self { scopes: vec![HashMap::new()] }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, name: impl Into<String>, value: LoweredValue<'c>) {
        self.scopes
            .last_mut()
            .expect("Env always has at least one scope")
            .insert(name.into(), value);
    }

    pub fn lookup(&self, name: &str) -> Option<LoweredValue<'c>> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name).cloned())
    }
}
