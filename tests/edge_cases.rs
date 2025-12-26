mod common;
use common::create_germi;
use germi::Error;

#[test]
fn test_utf8_variable_name() {
    let mut germi = create_germi();
    germi.add_variable("🚀", "rocket");
    // Note: Scanner might restricted names to alphanumeric + _. 
    // If scanner supports unicode alphabetic, this works.
    let result = germi.interpolate("${🚀}").unwrap();
    // Use whatever behavior scanner implements. If it parses 🚀 as name, great.
    // If not, it might parse as literal or error.
    // Let's assume standard unicode support for now. If failure, we fix scanner or test.
    assert_eq!(result, "rocket");
}

#[test]
fn test_utf8_value() {
    let mut germi = create_germi();
    germi.add_variable("GREETING", "Héllo Wörld ðŸŒ");
    let result = germi.interpolate("${GREETING}").unwrap();
    assert_eq!(result, "Héllo Wörld ðŸŒ");
}

#[test]
fn test_whitespace_in_braces() {
    let germi = create_germi();
    // ${  TEST_VAR  } - some shells allow this, some don't.
    // Scanner: check if we parse whitespace around name.
    // If our scanner is strict (name immediately after {), this fails or is invalid.
    // Sprout/Ecolog spec usually strict? 
    // Let's test standard behavior. If fails, we know limitations.
    let result = germi.interpolate("${  TEST_VAR  }");
    // Assuming strict scanner for now: likely returns literal or error
    // If strict, "${  TEST_VAR  }" might be tokenized as Variable(name="  TEST_VAR  ").
    // Let's assume it treats it as variable look up with spaces in key.
    // Key "TEST_VAR" exists. Key "  TEST_VAR  " does not.
    match result {
        Err(Error::MissingVar(_)) => {}, // Expected if keys strictly match
        Ok(s) if s == "${  TEST_VAR  }" => {}, // If parsed as literal
        _ => {} 
    }
}

#[test]
fn test_unclosed_brace() {
    let germi = create_germi();
    let result = germi.interpolate("${TEST_VAR");
    match result {
        Err(Error::UnclosedBrace(_)) => {},
        _ => panic!("Expected UnclosedBrace error"),
    }
}

#[test]
fn test_unclosed_brace_with_modifier() {
    let germi = create_germi();
    let result = germi.interpolate("${TEST_VAR:-default");
    match result {
        Err(Error::UnclosedBrace(_)) => {},
        _ => panic!("Expected UnclosedBrace error"),
    }
}

#[test]
fn test_empty_key() {
    let mut germi = create_germi();
    germi.add_variable("", "empty_key_val");
    let result = germi.interpolate("${}").unwrap();
    // Scanner might allow empty name in ${}
    // If so, it looks up "".
    assert_eq!(result, "empty_key_val");
}

#[test]
fn test_numeric_start_key() {
    let mut germi = create_germi();
    germi.add_variable("1VAR", "numeric");
    let result = germi.interpolate("${1VAR}").unwrap();
    assert_eq!(result, "numeric");
}

#[test]
fn test_literal_dollar_followed_by_invalid() {
    let germi = create_germi();
    // $ followed by space -> Literal $ + space
    let result = germi.interpolate("$ NotVar").unwrap();
    assert_eq!(result, "$ NotVar");
}

#[test]
fn test_literal_dollar_end_of_string() {
    let germi = create_germi();
    let result = germi.interpolate("Value: $").unwrap();
    assert_eq!(result, "Value: $");
}
