#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FeatureConfig {
    /// Enable variable substitution (${VAR}, $VAR)
    pub variables: bool,
    /// Enable default values (${VAR:-default})
    pub defaults: bool,
    /// Enable alternate values (${VAR-default})
    pub alternates: bool,
    /// Enable conditional values (${VAR:+value})
    pub conditionals: bool,
    /// Enable escape sequences
    pub escapes: bool,
    /// Enable command substitution ($(cmd))
    pub commands: bool,
    /// Enable backtick command substitution (`cmd`)
    pub backtick_commands: bool,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            variables: true,
            defaults: true,
            alternates: true,
            conditionals: true,
            escapes: true,
            commands: true,
            backtick_commands: true,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// Maximum recursion depth for variable expansion
    pub max_depth: usize,
    /// Enable strict mode (fail on empty values if using :?)
    pub strict_unsets: bool,
    /// Feature flags
    pub features: FeatureConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_depth: 10,
            strict_unsets: false,
            features: FeatureConfig::default(),
        }
    }
}
