use std::borrow::Cow;
use std::collections::HashMap;
use crate::context::VariableProvider;
use crate::error::Error;
use crate::scanner::{Scanner, Token};
use crate::config::Config;

struct OverlayProvider<'a, P: VariableProvider + ?Sized> {
    base: &'a P,
    overlay: &'a HashMap<String, String>,
}

impl<'a, P: VariableProvider + ?Sized> VariableProvider for OverlayProvider<'a, P> {
    fn get_value(&self, key: &str) -> Option<&str> {
        self.overlay.get(key).map(|s| s.as_str()).or_else(|| self.base.get_value(key))
    }
}

pub struct Interpolator<'a> {
    context: &'a dyn VariableProvider,
    config: &'a Config,
}

impl<'a> Interpolator<'a> {
    pub fn new(context: &'a dyn VariableProvider, config: &'a Config) -> Self {
        Self { context, config }
    }

    /// Interpolate a string using variables from the provider, respecting the configuration.
    ///
    /// Returns `Cow::Borrowed` if no interpolation happened (zero-copy), or `Cow::Owned`
    /// if the string was modified.
    pub fn interpolate<'b>(&self, input: &'b str) -> Result<Cow<'b, str>, Error> {
        self.resolve(input, 0, false)
    }

    /// Interpolate with additional temporary variables
    pub fn interpolate_with<'b>(&self, input: &'b str, extra_vars: &HashMap<String, String>) -> Result<Cow<'b, str>, Error> {
         let overlay = OverlayProvider {
             base: self.context,
             overlay: extra_vars,
         };

         let temp_interpolator = Interpolator {
             context: &overlay,
             config: self.config,
         };

         temp_interpolator.resolve(input, 0, false)
    }

    #[cfg(feature = "async")]
    pub async fn interpolate_async<'b>(&self, input: &'b str) -> Result<Cow<'b, str>, Error> {
        // First pass: Variable Interpolation (Sync)
        // Preserve command-related escapes for async pass to handle
        let resolved_vars = self.resolve(input, 0, true)?;

        // Second pass: Command Substitution (Async)
        // Only if commands or backtick_commands are enabled
        if self.config.features.commands || self.config.features.backtick_commands {
             self.resolve_commands(resolved_vars).await
        } else {
             // No command processing, but we need to resolve any preserved escapes
             self.finalize_escapes(resolved_vars)
        }
    }

    /// Resolve preserved escape sequences (used when async commands are disabled)
    #[cfg(feature = "async")]
    fn finalize_escapes<'b>(&self, input: Cow<'b, str>) -> Result<Cow<'b, str>, Error> {
        let source = input.as_ref();
        if !source.contains('\\') {
            return Ok(input);
        }

        let mut scanner = Scanner::new(source);
        let mut result = String::with_capacity(source.len());
        let mut last_pos = 0;
        let mut modified = false;

        while let Some((token, range)) = scanner.scan_next()? {
            match token {
                Token::Escape(c) => {
                    result.push_str(&source[last_pos..range.start]);
                    result.push(c);
                    modified = true;
                },
                _ => {
                    result.push_str(&source[range.clone()]);
                }
            }
            last_pos = range.end;
        }

        if modified {
            if last_pos < source.len() {
                result.push_str(&source[last_pos..]);
            }
            Ok(Cow::Owned(result))
        } else {
            Ok(input)
        }
    }

    #[cfg(feature = "async")]
    async fn resolve_commands<'b>(&self, input: Cow<'b, str>) -> Result<Cow<'b, str>, Error> {
        let source = match &input {
            Cow::Borrowed(s) => s,
            Cow::Owned(s) => s.as_str(),
        };

        // Quick check: if no command syntax, return as-is
        // Note: We still need to scan if there might be Escape tokens from first pass
        if !source.contains("$(") && !source.contains('`') && !source.contains('\\') {
            return Ok(input);
        }

        let mut scanner = Scanner::new(source);
        let mut result = String::with_capacity(source.len());
        let mut last_pos = 0;
        let mut modified = false;

        while let Some((token, range)) = scanner.scan_next()? {
             match token {
                 Token::Command(cmd) => {
                     if self.config.features.commands {
                         // Resolve variables inside command, don't preserve escapes
                         let resolved_cmd = self.resolve(cmd, 0, false)?;
                         let output = self.execute_command(&resolved_cmd).await?;
                         result.push_str(&source[last_pos..range.start]);
                         result.push_str(&output);
                         modified = true;
                     } else {
                         result.push_str(&source[range.clone()]);
                     }
                 },
                 Token::BacktickCommand(cmd) => {
                     if self.config.features.backtick_commands {
                         // Resolve variables inside command, don't preserve escapes
                         let resolved_cmd = self.resolve(cmd, 0, false)?;
                         let output = self.execute_command(&resolved_cmd).await?;
                         result.push_str(&source[last_pos..range.start]);
                         result.push_str(&output);
                         modified = true;
                     } else {
                         result.push_str(&source[range.clone()]);
                     }
                 },
                 Token::Escape(c) => {
                     // Escape tokens are emitted by scanner for \` and \$
                     // Output the escaped character literally (not interpreted as command)
                     result.push_str(&source[last_pos..range.start]);
                     result.push(c);
                     modified = true;
                 },
                 _ => {
                     // Literals and Variables (already resolved in first pass)
                     result.push_str(&source[range.clone()]);
                 }
             }
             last_pos = range.end;
        }

        if modified {
            if last_pos < source.len() {
                result.push_str(&source[last_pos..]);
            }
            Ok(Cow::Owned(result))
        } else {
            Ok(input)
        }
    }
    
    #[cfg(feature = "async")]
    async fn execute_command(&self, cmd: &str) -> Result<String, Error> {
        use tokio::process::Command;
        
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        
        // Simple shell execution
        let output = Command::new(&shell)
            .arg("-c")
            .arg(cmd)
            .output()
            .await // This requires tokio runtime


            .map_err(|e| Error::IoError(e.to_string()))?;
            
        if !output.status.success() {
             return Err(Error::CommandError(String::from_utf8_lossy(&output.stderr).to_string()));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim_end().to_string())
    }

    /// Internal resolve function.
    /// `preserve_cmd_escapes`: if true, preserve \` and \$ escapes for later processing
    fn resolve<'b>(&self, input: &'b str, depth: usize, preserve_cmd_escapes: bool) -> Result<Cow<'b, str>, Error> {
        if depth > self.config.max_depth {
            return Err(Error::RecursiveLookup(input.to_string()));
        }

        let mut scanner = Scanner::new(input);
        let mut result: Option<String> = None;
        let mut last_pos = 0;

        while let Some((token, range)) = scanner.scan_next()? {
            // If it's the first modification, initialize result
            if result.is_none() {
                 match &token {
                     Token::Literal(_) => {
                         // No change yet
                     },
                     Token::Variable { .. } => {
                         // Will change
                         let mut s = String::with_capacity(input.len() + 32);
                         s.push_str(&input[0..range.start]);
                         result = Some(s);
                     },
                     Token::Escape(_) => {
                         // Only causes change if not preserving
                         if !preserve_cmd_escapes {
                             let mut s = String::with_capacity(input.len() + 32);
                             s.push_str(&input[0..range.start]);
                             result = Some(s);
                         }
                     },
                     Token::Command(_) | Token::BacktickCommand(_) => {
                         // In sync interpolate, we treat commands as literals
                         // No change needed.
                     }
                 }
            }

            if let Some(res) = &mut result {
                 // Append literal or resolved value
                 match token {
                     Token::Literal(s) => {
                         if self.config.features.escapes && s.contains('\\') {
                             Self::unescape_into(res, s);
                         } else {
                             res.push_str(s);
                         }
                     },
                     Token::Variable { name, default, strict, conditional } => {
                         if self.config.features.variables {
                             let val = self.resolve_variable(name, default, strict, conditional, depth, preserve_cmd_escapes)?;
                             res.push_str(&val);
                         } else {
                             res.push_str(&input[range.clone()]);
                         }
                     },
                     Token::Command(_) | Token::BacktickCommand(_) => {
                         // In sync mode, commands are treated as literals
                         res.push_str(&input[range.clone()]);
                     },
                     Token::Escape(c) => {
                         if preserve_cmd_escapes {
                             // Keep original escape sequence for async pass
                             res.push_str(&input[range.clone()]);
                         } else {
                             // Resolve escape: output the character
                             res.push(c);
                         }
                     }
                 }
            } else {
                // We are still borrowed. Check if we need to switch due to escapes in Literal?
                if let Token::Literal(s) = token {
                     if self.config.features.escapes && s.contains('\\') {
                         // Switch to owned!
                         let mut res = String::with_capacity(input.len() + 16);
                         res.push_str(&input[..range.start]);
                         Self::unescape_into(&mut res, s);
                         result = Some(res);
                     }
                }
            }

            last_pos = range.end;
        }

        if let Some(mut res) = result {
            if last_pos < input.len() {
                let tail = &input[last_pos..];
                 if self.config.features.escapes && tail.contains('\\') {
                     Self::unescape_into(&mut res, tail);
                 } else {
                     res.push_str(tail);
                 }
            }
            Ok(Cow::Owned(res))
        } else {
             // Input might have escapes that need processing even if no variables
             // BUT if preserve_cmd_escapes is true, we return as-is (escapes preserved)
             if self.config.features.escapes && input.contains('\\') && !preserve_cmd_escapes {
                 let mut res = String::with_capacity(input.len());
                 Self::unescape_into(&mut res, input);
                 Ok(Cow::Owned(res))
             } else {
                 Ok(Cow::Borrowed(input))
             }
        }
    }
    
    /// Unescape standard escape sequences in a string.
    /// Note: \` and \$ are handled by the scanner as Escape tokens, not here.
    fn unescape_into(buf: &mut String, s: &str) {
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => buf.push('\n'),
                    Some('r') => buf.push('\r'),
                    Some('t') => buf.push('\t'),
                    Some('\\') => buf.push('\\'),
                    Some('"') => buf.push('"'),
                    Some('\'') => buf.push('\''),
                    // Note: \` and \$ won't appear here - they're emitted as Escape tokens
                    Some(other) => {
                        // Unknown escape: Bash behavior is \c -> c
                        buf.push(other);
                    }
                    None => {
                        // Trailing backslash
                        buf.push('\\');
                    }
                }
            } else {
                buf.push(c);
            }
        }
    }
    
    fn resolve_variable<'b>(&self, name: &str, default: Option<&'b str>, strict: bool, conditional: bool, depth: usize, preserve_cmd_escapes: bool) -> Result<Cow<'b, str>, Error> {
        let val_opt = self.context.get_value(name);

        // Determine effective modifier based on feature flags
        let (use_default, use_alternate, use_conditional) = match (default, strict, conditional) {
            (Some(_), true, false) => (self.config.features.defaults, false, false),      // :-
            (Some(_), false, false) => (false, self.config.features.alternates, false),   // -
            (Some(_), _, true) => (false, false, self.config.features.conditionals),      // :+ or +
            _ => (false, false, false), // No modifier or not relevant
        };

        let effective_default = if use_default || use_alternate || use_conditional {
            default
        } else {
            None
        };

        if conditional {
             if self.config.features.conditionals {
                  match val_opt {
                      Some(v) => {
                           if strict && v.is_empty() {
                               return Ok(Cow::Borrowed(""));
                           }

                           if let Some(def_raw) = effective_default {
                               return self.resolve(def_raw, depth + 1, preserve_cmd_escapes);
                           }
                           return Ok(Cow::Borrowed(""));
                      },
                      None => return Ok(Cow::Borrowed("")),
                  }
             }
        }

        match (val_opt, strict) {
             (Some(v), _) => {
                 if !conditional && strict && v.is_empty() {
                      if self.config.features.defaults {
                           if let Some(def_raw) = default {
                               return self.resolve(def_raw, depth + 1, preserve_cmd_escapes);
                           }
                      }
                 }

                 let resolved = self.resolve(v, depth + 1, preserve_cmd_escapes)?;
                 Ok(Cow::Owned(resolved.into_owned()))
             },
             (None, _) => {
                 if !conditional {
                     if strict && self.config.features.defaults {
                          if let Some(def_raw) = default {
                              return self.resolve(def_raw, depth + 1, preserve_cmd_escapes);
                          }
                     }

                     if !strict && self.config.features.alternates {
                          if let Some(def_raw) = default {
                              return self.resolve(def_raw, depth + 1, preserve_cmd_escapes);
                          }
                     }
                 }

                 Err(Error::MissingVar(name.to_string()))
             }
        }
    }
}
