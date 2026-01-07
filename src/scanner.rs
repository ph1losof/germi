
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
    /// Command substitution using $(cmd) syntax
    Command(&'a str),
    /// Command substitution using legacy `cmd` backtick syntax
    BacktickCommand(&'a str),
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

    pub fn scan_next(&mut self) -> Result<Option<(Token<'a>, std::ops::Range<usize>)>, Error> {
        if self.byte_idx >= self.source.len() {
            return Ok(None);
        }

        let start = self.byte_idx;
        let mut current = start;

        while current < self.source.len() {
            let rem = &self.source.as_bytes()[current..];

            // memchr only supports up to 3 chars, so we find min of two searches
            let pos_special = memchr::memchr3(b'$', b'\\', b'\'', rem);
            let pos_backtick = memchr::memchr(b'`', rem);
            let combined = match (pos_special, pos_backtick) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };

            match combined {
                Some(p) => {
                    let abs_p = current + p;
                    let char_found = self.source.as_bytes()[abs_p];
                    
                    if char_found == b'\\' {
                        // Backslash escape. Skip this backslash and the next character.
                        if abs_p + 1 < self.source.len() {
                            let next_str = &self.source[abs_p+1..];
                            if let Some(c) = next_str.chars().next() {
                                current = abs_p + 1 + c.len_utf8();
                            } else {
                                current = self.source.len();
                            }
                        } else {
                             current = self.source.len();
                        }
                    } else if char_found == b'\'' {
                        // Single quote block. Skip until closing quote.
                        let inner_start = abs_p + 1;
                        let inner_rem = &self.source.as_bytes()[inner_start..];
                        
                        let mut found_close = false;
                        let mut scan_pos = 0;
                        
                        // Scan for ' or \ inside
                        while let Some(q_pos) = memchr::memchr2(b'\'', b'\\', &inner_rem[scan_pos..]) {
                            let abs_q = inner_start + scan_pos + q_pos;
                            let qc = self.source.as_bytes()[abs_q];
                            
                            if qc == b'\\' {
                                // Escaped char inside single quotes (e.g. \')
                                if abs_q + 1 < self.source.len() {
                                    let next_str = &self.source[abs_q+1..];
                                    if let Some(c) = next_str.chars().next() {
                                        scan_pos = (abs_q + 1 + c.len_utf8()) - inner_start;
                                    } else {
                                        scan_pos = self.source.len() - inner_start;
                                    }
                                } else {
                                    scan_pos = self.source.len() - inner_start;
                                }
                            } else {
                                // Found closing '
                                current = abs_q + 1;
                                found_close = true;
                                break;
                            }
                        }
                        
                        if !found_close {
                            current = self.source.len();
                        }
                    } else if char_found == b'$' {
                        // Found Variable start

                        // If we have accumulated text before this $, return it as Literal first
                        if abs_p > start {
                            let text = &self.source[start..abs_p];
                            self.byte_idx = abs_p;
                            return Ok(Some((Token::Literal(text), start..abs_p)));
                        }

                        // Valid variable start at start index
                        self.byte_idx = abs_p;

                        let var_token_opt = self.parse_variable(abs_p)?;
                        if let Some(token) = var_token_opt {
                             let end = self.byte_idx;
                             return Ok(Some((token, start..end)));
                        } else {
                             unreachable!("parse_variable returned None");
                        }
                    } else if char_found == b'`' {
                        // Found backtick command substitution start

                        // If we have accumulated text before this `, return it as Literal first
                        if abs_p > start {
                            let text = &self.source[start..abs_p];
                            self.byte_idx = abs_p;
                            return Ok(Some((Token::Literal(text), start..abs_p)));
                        }

                        // Parse backtick command
                        self.byte_idx = abs_p;

                        let backtick_token_opt = self.parse_backtick_command(abs_p)?;
                        if let Some(token) = backtick_token_opt {
                             let end = self.byte_idx;
                             return Ok(Some((token, start..end)));
                        } else {
                             unreachable!("parse_backtick_command returned None");
                        }
                    }
                },
                None => {
                    // No more special chars. Rest is literal.
                    current = self.source.len();
                }
            }
        }
        
        // Loop finished (reached end of string)
        let text = &self.source[start..current];
        self.byte_idx = current;
        if text.is_empty() {
             Ok(None)
        } else {
             Ok(Some((Token::Literal(text), start..current)))
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
        let inner_start = start_idx + 2; // skip '$('
        let remaining = &self.source[inner_start..];
        
        let mut depth = 1;
        let mut in_single_quote = false;
        let mut in_double_quote = false;
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
            return Err(Error::SyntaxError(format!("Unclosed command substitution starting at {}", start_idx), start_idx));
        }

        let cmd = &self.source[inner_start..end_idx];
        self.byte_idx = end_idx + 1; // skip ')'

        Ok(Some(Token::Command(cmd)))
    }

    fn parse_backtick_command(&mut self, start_idx: usize) -> Result<Option<Token<'a>>, Error> {
        // `command`
        let inner_start = start_idx + 1; // skip opening backtick
        let remaining = &self.source[inner_start..];

        let mut end_idx = 0;
        let mut found = false;

        let mut chars = remaining.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '\\' => {
                    // Handle escapes inside backticks
                    // Only \` and \\ are meaningful escapes inside backticks
                    if let Some((_, next_c)) = chars.next() {
                        match next_c {
                            '`' | '\\' => continue, // escaped, skip
                            _ => {} // pass through (backslash consumed with next char)
                        }
                    }
                }
                '`' => {
                    // Found closing backtick
                    end_idx = inner_start + i;
                    found = true;
                    break;
                }
                _ => {}
            }
        }

        if !found {
            return Err(Error::SyntaxError(
                format!("Unclosed backtick command starting at {}", start_idx),
                start_idx,
            ));
        }

        let cmd = &self.source[inner_start..end_idx];
        self.byte_idx = end_idx + 1; // skip closing backtick

        Ok(Some(Token::BacktickCommand(cmd)))
    }

    fn parse_simple_variable(&mut self, start_idx: usize) -> Result<Option<Token<'a>>, Error> {
        let mut len = 1; // '$'
        let remaining = &self.source[start_idx + 1..];
        
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
        let inner_start = start_idx + 2; // skip '${'
        let remaining = &self.source[inner_start..];
        
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

        let mut name_len = content.len();
        let mut modifier = None;
        
        for (i, c) in content.char_indices() {
            if c == ':' {
               if let Some(next_c) = content[i+1..].chars().next() {
                   if next_c == '-' {
                       name_len = i;
                       modifier = Some((true, false, i+2));
                       break;
                   } else if next_c == '+' {
                       name_len = i;
                       modifier = Some((true, true, i+2));
                       break;
                   }
               }
            } else if c == '-' {
                name_len = i;
                modifier = Some((false, false, i+1));
                break;
            } else if c == '+' {
                name_len = i;
                modifier = Some((false, true, i+1));
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
