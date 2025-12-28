use std::collections::HashMap;

/// A trait for providing variable values during interpolation.
pub trait VariableProvider {
    /// Retrieve the value of a variable by name.
    fn get_value(&self, key: &str) -> Option<&str>;
}

impl VariableProvider for HashMap<String, String> {
    fn get_value(&self, key: &str) -> Option<&str> {
        self.get(key).map(|s| s.as_str())
    }
}

impl VariableProvider for HashMap<&str, &str> {
    fn get_value(&self, key: &str) -> Option<&str> {
        self.get(key).copied()
    }
}

/// A simple in-memory context.
#[derive(Debug, Clone, Default)]
pub struct SimpleContext {
    vars: HashMap<String, String>,
}

impl SimpleContext {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(key.into(), value.into());
    }
}

impl VariableProvider for SimpleContext {
    fn get_value(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|s| s.as_str())
    }
}
