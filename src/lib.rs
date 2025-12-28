mod config;
mod context;
mod error;
mod interpolator;
pub mod scanner;

use std::borrow::Cow;
use std::collections::HashMap;

use crate::interpolator::Interpolator;

pub use config::{Config, FeatureConfig};
pub use context::{SimpleContext, VariableProvider};
pub use error::Error;

use std::collections::HashSet;

/// Main entry point for the Germi interpolation engine.
#[derive(Debug, Clone)]
pub struct Germi {
    config: Config,
    context: SimpleContext,
}

impl Default for Germi {
    fn default() -> Self {
        Self::new()
    }
}

impl Germi {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            context: SimpleContext::new(),
        }
    }

    pub fn with_config(config: Config) -> Self {
        Self {
            config,
            context: SimpleContext::new(),
        }
    }

    /// Add a variable to the internal context.
    pub fn add_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.context.insert(key, value);
    }

    /// Interpolate a string using variables from the context.
    pub fn interpolate<'b>(&self, input: &'b str) -> Result<Cow<'b, str>, Error> {
        let interpolator = Interpolator::new(&self.context, &self.config);
        interpolator.interpolate(input)
    }

    /// Interpolate a string using temporary additional variables.
    pub fn interpolate_with<'b>(
        &self,
        input: &'b str,
        extra_vars: &HashMap<String, String>,
    ) -> Result<Cow<'b, str>, Error> {
        let interpolator = Interpolator::new(&self.context, &self.config);
        interpolator.interpolate_with(input, extra_vars)
    }

    /// Interpolate a string asynchronously, supporting command substitution `$(cmd)`.
    /// Requires `async` feature.
    #[cfg(feature = "async")]
    pub async fn interpolate_async<'b>(&self, input: &'b str) -> Result<Cow<'b, str>, Error> {
        let interpolator = Interpolator::new(&self.context, &self.config);
        interpolator.interpolate_async(input).await
    }
}

pub fn find_variable_references(input: &str) -> Vec<String> {
    let mut scanner = scanner::Scanner::new(input);
    let mut variables = HashSet::new();

    while let Ok(Some((token, _))) = scanner.scan_next() {
        if let scanner::Token::Variable { name, .. } = token {
            variables.insert(name.to_string());
        }
    }

    // Convert to sorted Vec for deterministic ordering
    let mut result: Vec<String> = variables.into_iter().collect();
    result.sort();
    result
}
