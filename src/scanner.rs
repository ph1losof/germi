use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    Literal(&'a str),
    Variable {
        name: &'a str,
        default: Option<&'a str>,
        /// true means `${VAR:-val}`, false means `${VAR-val}` (or similar logic for +)
        strict: bool,
        /// If true, this is a conditional substitution like `${VAR:+val}`
        conditional: bool,
    },
    Command(&'a str),
}

#[derive(Debug)]
pub struct Scanner<'a> {
    source: &'a str,
    byte_idx: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source, byte_idx: 0 }
    }
    
    fn remaining(&self) -> &'a str {
        &self.source[self.byte_idx..]
    }
    


    pub fn scan_next(&mut self) -> Result<Option<(Token<'a>, std::ops::Range<usize>)>, Error> {
        if self.byte_idx >= self.source.len() {
            return Ok(None);
        }

        let start = self.byte_idx;
        let mut chars = self.remaining().char_indices();
        
        while let Some((idx, c)) = chars.next() {
            let current_abs_idx = start + idx;
            
            // Handle escape sequences
            if c == '\\' {
                // Peek next
                if let Some((_, _next_char)) = chars.next() {
                    continue; // Skip next char (it's part of escape)
                } else {
                    continue;
                }
            }
            
            // Handle single quotes (disable interpolation)
            if c == '\'' {
                // Scan until closing quote
                // We are inside a literal block now.
                // We need to consume chars until next ' that is NOT escaped?
                // Spec say: "Escape sequences inside single quotes are typically NOT processed (literal strings), except typically \' and \\"
                // So valid: 'It\'s me'
                
                // Note: we are ALREADY iterating `chars`.
                while let Some((_, qc)) = chars.next() {
                    if qc == '\\' {
                        // Consume the next char to escape it (so it doesn't trigger end quote if it is ')
                        chars.next();
                    } else if qc == '\'' {
                         // Found closing quote!
                         break;
                    }
                }
                // Continue scanning
                continue;
            }

            if c == '$' {
                // If we have accumulated literal text before this $, return it first
                if idx > 0 {
                    let text = &self.source[start..current_abs_idx];
                    self.byte_idx = current_abs_idx;
                    return Ok(Some((Token::Literal(text), start..current_abs_idx)));
                }

                // Parse Variable
                let var_token_opt = self.parse_variable(current_abs_idx)?;
                if let Some(token) = var_token_opt {
                    let end = self.byte_idx;
                    return Ok(Some((token, start..end)));
                } else {
                     // It returned None if it wasn't a variable but a literal '$'
                     // parse_variable set byte_idx? 
                     // My parse_variable logic: "If not a variable... byte_idx = start_idx + 1; Ok(Some(Literal))"
                     // So I need to adjust parse_variable signature or handle it.
                     // But wait, parse_variable returns `Option<Token>`.
                     // I'll update parse_variable to return `(Token, Range)` too?
                     // Or scan_next handles range? 
                     // parse_variable modifies self.byte_idx.
                     // So range is start..self.byte_idx.
                     // But parse_variable returns `Option<Token>`.
                     // Let's rely on it.
                     unreachable!("parse_variable should return a token if it advances");
                }
            }
        }
        
        // End of string, return remaining as literal
        let text = &self.source[self.byte_idx..];
        let start_pos = self.byte_idx;
        self.byte_idx = self.source.len();
        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some((Token::Literal(text), start_pos..self.source.len())))
        }
    }

    fn parse_variable(&mut self, start_idx: usize) -> Result<Option<Token<'a>>, Error> {
        // We are at '$'
        let mut iter = self.source[start_idx..].chars();
        iter.next(); // skip '$'
        
        match iter.next() {
            Some('{') => {
                self.parse_braced_variable(start_idx)
            },
            Some('(') => {
                self.parse_command_substitution(start_idx)
            },
            Some(c) if c.is_alphabetic() || c == '_' => {
                self.parse_simple_variable(start_idx)
            },
            _ => {
                // Not a variable, return literal '$'
                self.byte_idx = start_idx + 1;
                Ok(Some(Token::Literal(&self.source[start_idx..start_idx+1])))
            }
        }
    }

    fn parse_command_substitution(&mut self, start_idx: usize) -> Result<Option<Token<'a>>, Error> {
        // $( ... )
        // Command substitution parsing needs to handle nested parentheses and quotes, 
        // similar to bash but simplified.
        let inner_start = start_idx + 2; // skip '$('
        let remaining = &self.source[inner_start..];
        
        let mut depth = 1;
        let mut in_single_quote = false;
        let mut in_double_quote = false; // Just simplistic tracking
        let mut end_idx = 0;
        let mut found = false;

        let mut chars = remaining.char_indices();
        while let Some((i, c)) = chars.next() {
            if in_single_quote {
                if c == '\'' {
                    in_single_quote = false;
                }
            } else if in_double_quote {
                if c == '"' {
                     // Check for escaping? Simplified for now as per Sprout logic
                     in_double_quote = false;
                } else if c == '\\' {
                    chars.next(); // skip next char
                }
            } else {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end_idx = inner_start + i;
                            found = true;
                            break;
                        }
                    },
                    '\'' => in_single_quote = true,
                    '"' => in_double_quote = true,
                    '\\' => { chars.next(); },
                    _ => {}
                }
            }
        }

        if !found {
            // Unclosed command
            return Err(Error::SyntaxError(format!("Unclosed command substitution starting at {}", start_idx), start_idx));
        }

        let cmd = &self.source[inner_start..end_idx];
        self.byte_idx = end_idx + 1; // skip ')'
        
        Ok(Some(Token::Command(cmd)))
    }

    fn parse_simple_variable(&mut self, start_idx: usize) -> Result<Option<Token<'a>>, Error> {
        // $VAR - scan until non-alphanumeric/underscore
        // The first char was already checked to be alpha or _
        // skip $ and first char
        let mut len = 1; // '$'
        let remaining = &self.source[start_idx + 1..];
        
        // We need to match \w+ basically
        for c in remaining.chars() {
            if c.is_alphanumeric() || c == '_' {
                len += c.len_utf8();
            } else {
                break;
            }
        }
        
        let name = &self.source[start_idx+1..start_idx+len];
        self.byte_idx = start_idx + len;
        
        Ok(Some(Token::Variable {
            name,
            default: None,
            strict: false,
            conditional: false,
        }))
    }

    fn parse_braced_variable(&mut self, start_idx: usize) -> Result<Option<Token<'a>>, Error> {
        // ${ ... }
        // We need to find the closing '}' handling nested braces optionally? 
        // Requirements say "Iterative resolution: Handles nested variable references".
        // Usually, the scanner just finds the matching brace. Variable semantics are inner.
        // But for ${VAR:-${OTHER}}, the inner part is the default value.
        
        // Scan for name first
        let inner_start = start_idx + 2; // skip '${'

        
        let remaining = &self.source[inner_start..];
        
        // Find end of generic variable block
        // We need to account for nesting in the default value part: ${VAR:-${DEFAULT}}
        let mut balance = 1;
        let mut end_idx = 0;
        
        for (i, c) in remaining.char_indices() {
             if c == '}' {
                 balance -= 1;
                 if balance == 0 {
                     end_idx = inner_start + i;
                     break;
                 }
             } else if c == '{' {
                 balance += 1;
             }
        }
        
        if balance != 0 {
            return Err(Error::UnclosedBrace(start_idx));
        }

        let content = &self.source[inner_start..end_idx];
        self.byte_idx = end_idx + 1; // skip '}'

        // Now parse name and modifier from content
        // content = "VAR:-default" or "VAR"
        
        // Find first separator
        let mut name_len = content.len();
        let mut modifier = None; // (strict, conditional, value_start_idx)
        
        // Scan content for :, -, +
        // Names are typically alpha numeric _. 
        // But let's just look for the first occurrence of operator chars.
        
        for (i, c) in content.char_indices() {
            if c == ':' {
               // Next should be - or + or ? (if supported, spec only says :- - :+ +)
               // Check next
               if let Some(next_c) = content[i+1..].chars().next() {
                   if next_c == '-' {
                       // :-
                       name_len = i;
                       modifier = Some((true, false, i+2)); // strict=true, cond=false
                       break;
                   } else if next_c == '+' {
                       // :+
                       name_len = i;
                       modifier = Some((true, true, i+2));
                       break;
                   }
               }
               // What if just ${VAR:stuff}? Spec doesn't mention it. 
               // Bash supports offsets ${VAR:offset}. Spec only listed default/alt.
               // We'll treat as part of name or invalid if strict? 
               // Assume Spec is exhaustive: -, :-, +, :+
            } else if c == '-' {
                name_len = i;
                modifier = Some((false, false, i+1)); // strict=false, cond=false
                break;
            } else if c == '+' {
                name_len = i;
                modifier = Some((false, true, i+1)); // strict=false, cond=true
                break;
            }
        }
        
        let name = &content[0..name_len];
        let default_val = if let Some((_, _, start)) = modifier {
            Some(&content[start..])
        } else {
            None
        };
        
        let (strict, conditional) = if let Some((s, c, _)) = modifier {
            (s, c)
        } else {
            (false, false)
        };

        Ok(Some(Token::Variable {
            name,
            default: default_val,
            strict,
            conditional,
        }))
    }
}
