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
        self.resolve(input, 0)
    }

    /// Interpolate with additional temporary variables
    pub fn interpolate_with<'b>(&self, input: &'b str, extra_vars: &HashMap<String, String>) -> Result<Cow<'b, str>, Error> {
         // Create a temporary provider that merges context + extra_vars
         // Or simpler: We can just use a Chained Provider?
         // Since VariableProvider is a trait, we can make a struct `OverlayProvider<'a, P>`.
         let overlay = OverlayProvider {
             base: self.context,
             overlay: extra_vars,
         };
         
         let temp_interpolator = Interpolator {
             context: &overlay,
             config: self.config,
         };
         
         temp_interpolator.resolve(input, 0)
    }

    #[cfg(feature = "async")]
    pub async fn interpolate_async<'b>(&self, input: &'b str) -> Result<Cow<'b, str>, Error> {
        // First pass: Variable Interpolation (Sync)
        let resolved_vars = self.interpolate(input)?;
        
        // Second pass: Command Substitution (Async)
        // Only if commands are enabled
        if self.config.features.commands {
             self.resolve_commands(resolved_vars).await
        } else {
             Ok(resolved_vars)
        }
    }

    #[cfg(feature = "async")]
    async fn resolve_commands<'b>(&self, input: Cow<'b, str>) -> Result<Cow<'b, str>, Error> {
        // We need to scan again for Command tokens.
        // Scanner works on &str.
        let source = match &input {
            Cow::Borrowed(s) => s,
            Cow::Owned(s) => s.as_str(),
        };

        if !source.contains("$(") {
            return Ok(input);
        }

        let mut scanner = Scanner::new(source);
        let mut result = String::with_capacity(source.len());
        let mut last_pos = 0;
        let mut modified = false;

        while let Some((token, range)) = scanner.scan_next()? {
             match token {
                 Token::Command(cmd) => {
                     let output = self.execute_command(cmd).await?;
                     result.push_str(&source[last_pos..range.start]); // Append text before command
                     result.push_str(&output);
                     modified = true;
                 },
                 _ => {
                     // Variables were already resolved in first pass? 
                     // Wait, `interpolate` (variable pass) returns a string where variables are replaced by their values.
                     // BUT, if the value contained `$(cmd)`, `interpolate` recursively resolved it?
                     // `resolve` recursively calls `resolve` for variables.
                     // But it does NOT touch `Command` tokens (treated as literal in sync pass).
                     // So `$(cmd)` remains `$(cmd)`.
                     // So scanning `resolved_vars` should find `$(cmd)`.
                     // Correct.
                     // We skip over literals tokens here... wait.
                     // `scan_next` returns tokens. If I have `Literal("foo")`, I append it.
                     // I need to reconstruct the string replacing ONLY Commands.
                 }
             }
             // Actually, `scan_next` gives me strictly `Command` or `Variable` or `Literal`.
             // `resolved_vars` should NOT have `Variable` tokens anymore (all resolved).
             // Unless we disabled variable interpolation separately.
             // Assuming variables are gone or ignored.
             
             // Logic:
             // If token is Command, replace.
             // Else, invalid? No, `scan_next` returns everything coverage? 
             // Yes Scanner covers full string.
             
             if let Token::Command(_) = token {
                 // Already handled above
             } else {
                 result.push_str(&source[range.clone()]);
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

    fn resolve<'b>(&self, input: &'b str, depth: usize) -> Result<Cow<'b, str>, Error> {
        if depth > self.config.max_depth {
            return Err(Error::RecursiveLookup(input.to_string()));
        }
        
        // Short-circuit if variable interpolation is disabled
        if !self.config.features.variables {
             // We still might need to process escapes or commands?
             // Sprout logic: if !variables, return valid input?
             // But escapes?
             // Let's assume generic flow. Tokenizer finds variables.
             // If disabled, we treat them as literals?
             // Scanner doesn't know config.
             // So here we check.
        }

        let mut scanner = Scanner::new(input);
        
        // Optimistic check: if no special chars found, return borrowed
        // But scanner does the finding. 
        // We iterate tokens.
        
        let mut result: Option<String> = None;
        let mut last_pos = 0;
        
        while let Some((token, range)) = scanner.scan_next()? {
            // If it's the first modification, initialize result
            if result.is_none() {
                 match token {
                     Token::Literal(_) => {
                         // No change yet
                     },
                     Token::Variable { .. } => {
                         // Will change
                         let mut s = String::with_capacity(input.len() + 32);
                         s.push_str(&input[0..range.start]);
                         result = Some(s);
                     },
                     Token::Command(_) => {
                         // In sync interpolate, we treat commands as literals (or ignore them)
                         // But we must NOT switch to owned if it's just a Command we are not processing?
                         // If we are respecting "pure sync", we output $(cmd) literally.
                         // So it is effectively a literal.
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
                             let val = self.resolve_variable(name, default, strict, conditional, depth)?;
                             res.push_str(&val);
                         } else {
                             res.push_str(&input[range.clone()]);
                         }
                     },
                     Token::Command(_) => {
                         res.push_str(&input[range.clone()]);
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
             if self.config.features.escapes && input.contains('\\') {
                 let mut res = String::with_capacity(input.len());
                 Self::unescape_into(&mut res, input);
                 Ok(Cow::Owned(res))
             } else {
                 Ok(Cow::Borrowed(input))
             }
        }
    }
    
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
                    Some(other) => {
                        // Unknown escape, keep backslash and char?
                        // Or Just push char?
                        // Bash: \c -> c.
                        // We'll behave like bash for consistency unless specified.
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
    
    fn resolve_variable<'b>(&self, name: &str, default: Option<&'b str>, strict: bool, conditional: bool, depth: usize) -> Result<Cow<'b, str>, Error> {
        let val_opt = self.context.get_value(name);

        // Determine effective modifier based on feature flags
        let (use_default, use_alternate, use_conditional) = match (default, strict, conditional) {
            (Some(_), true, false) => (self.config.features.defaults, false, false),      // :-
            (Some(_), false, false) => (false, self.config.features.alternates, false),   // -
            (Some(_), _, true) => (false, false, self.config.features.conditionals),      // :+ or +
            _ => (false, false, false), // No modifier or not relevant
        };

        // If the specific feature is disabled, we treat it as if no modifier was provided (default = None)
        // effectively falling back to basic ${VAR} resolution.
        
        let effective_default = if use_default || use_alternate || use_conditional {
            default
        } else {
            None
        };

        // If conditional is requested but disabled, treat as normal var lookup?
        // Wait, ${VAR:+val} with conditionals=false should behave like ${VAR}?
        // Or should it error? Or behave like empty?
        // Assuming "disabled" means "syntax ignored, return variable value".
        
        // Re-evaluate logic with effective flags:
        
        if conditional {
             if self.config.features.conditionals {
                  // Normal conditional logic
                  match val_opt {
                      Some(_) => {
                           // Set and strict/loose doesn't matter for :+ vs + in our token (we only track strict for - vs :-)
                           // Token says: strict means ':', conditional means '+'?
                           // Token definition: strict: bool, conditional: bool.
                           // Scanner: 
                           // :+ -> strict=true, conditional=true
                           // +  -> strict=false, conditional=true
                           // :- -> strict=true, conditional=false
                           // -  -> strict=false, conditional=false
                           
                           // Sprout config "conditionals" covers both :+ and +.
                           if let Some(def_raw) = effective_default {
                               return self.resolve(def_raw, depth + 1);
                           }
                           return Ok(Cow::Borrowed(""));
                      },
                      None => return Ok(Cow::Borrowed("")),
                  }
             } else {
                 // Feature disabled: treat as ${VAR} (ignore :+val)
                 // FALL THROUGH to normal lookup below
             }
        }

        // Logic for Defaults (:-) and Alternates (-)
        // We simplified above by masking `default`.
        // If we masked `default` to None, the match below handles it as basic var lookup.
        
        match (val_opt, strict) {
             (Some(v), _) => {
                 // Variable exists.
                 // For defaults/alternates, we use the variable value.
                 // Unless it's empty AND strict=true AND it's a default modifier (not conditional)
                 
                 // If strict=true (:-) and value is empty:
                 if !conditional && strict && v.is_empty() {
                      // Only use default if defaults feature is enabled!
                      if self.config.features.defaults {
                           if let Some(def_raw) = default {
                               return self.resolve(def_raw, depth + 1);
                           }
                      }
                 }
                 
                 // Otherwise return value
                 let resolved = self.resolve(v, depth + 1)?;
                 Ok(Cow::Owned(resolved.into_owned()))
             },
             (None, _) => {
                 // Variable Unset
                 // Use default if provided AND feature enabled AND not a conditional
                 
                 if !conditional {
                     // Case 1: :- (strict=true). Feature: defaults.
                     if strict && self.config.features.defaults {
                          if let Some(def_raw) = default {
                              return self.resolve(def_raw, depth + 1);
                          }
                     }
                     
                     // Case 2: - (strict=false). Feature: alternates.
                     if !strict && self.config.features.alternates {
                          if let Some(def_raw) = default {
                              return self.resolve(def_raw, depth + 1);
                          }
                     }
                 }
                 
                 // Fallback: No default or feature disabled.
                 Err(Error::MissingVar(name.to_string()))
             }
        }
    }
}
