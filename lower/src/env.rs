use super::value::{LoweredValue, StoredValue};
use melior::ir::ValueLike;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Env<'c> {
    scopes: Vec<HashMap<String, LoweredValue<'c>>>,
    /// Lifetime name -> its `qduc.newlft` token. Not `{}`-scoped, unlike `scopes`.
    lifetimes: HashMap<String, StoredValue<'c>>,
}

impl<'c> Env<'c> {
    pub fn new() -> Self {
        Self { scopes: vec![HashMap::new()], lifetimes: HashMap::new() }
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

    /// Returns `false` if already open.
    pub fn open_lifetime(&mut self, name: impl Into<String>, token: impl ValueLike<'c>) -> bool {
        let name = name.into();
        if self.lifetimes.contains_key(&name) {
            return false;
        }
        self.lifetimes.insert(name, StoredValue::new(token));
        true
    }

    pub fn close_lifetime(&mut self, name: &str) -> Option<StoredValue<'c>> {
        self.lifetimes.remove(name)
    }
}
