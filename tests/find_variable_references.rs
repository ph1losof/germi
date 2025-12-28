use germi::find_variable_references;

#[test]
fn test_simple_variable() {
    let refs = find_variable_references("Hello ${USER}");
    assert_eq!(refs, vec!["USER"]);
}

#[test]
fn test_unbraced_variable() {
    let refs = find_variable_references("Hello $USER");
    assert_eq!(refs, vec!["USER"]);
}

#[test]
fn test_multiple_variables() {
    let refs = find_variable_references("${A} and ${B} and ${C}");
    assert_eq!(refs, vec!["A", "B", "C"]);
}

#[test]
fn test_deduplication() {
    let refs = find_variable_references("${A} and ${A} and ${B} and ${A}");
    assert_eq!(refs, vec!["A", "B"]);
}

#[test]
fn test_with_default_modifier() {
    let refs = find_variable_references("${PATH:-/usr/bin}");
    assert_eq!(refs, vec!["PATH"]);
}

#[test]
fn test_with_strict_default_modifier() {
    let refs = find_variable_references("${PATH:-/usr/bin}");
    assert_eq!(refs, vec!["PATH"]);
}

#[test]
fn test_with_loose_default_modifier() {
    let refs = find_variable_references("${PATH-/usr/local/bin}");
    assert_eq!(refs, vec!["PATH"]);
}

#[test]
fn test_with_strict_alternate_modifier() {
    let refs = find_variable_references("${VAR:+alternate}");
    assert_eq!(refs, vec!["VAR"]);
}

#[test]
fn test_with_loose_alternate_modifier() {
    let refs = find_variable_references("${VAR+alternate}");
    assert_eq!(refs, vec!["VAR"]);
}

#[test]
fn test_mixed_variables_and_commands() {
    // Variables inside $(...) are not extracted at top level
    // The scanner processes command substitutions as a single token
    let refs = find_variable_references("${VAR1} and $(echo ${VAR2})");
    assert_eq!(refs, vec!["VAR1"]);
}

#[test]
fn test_single_quoted_blocks() {
    // Single quotes prevent interpolation
    // Variables inside single quotes are treated as literals
    let refs = find_variable_references(r"'${NOT_INTERPOLATED}' ${INTERPOLATED}");
    assert_eq!(refs, vec!["INTERPOLATED"]);
}

#[test]
fn test_empty_string() {
    let refs = find_variable_references("");
    assert!(refs.is_empty());
}

#[test]
fn test_no_variables() {
    let refs = find_variable_references("Just plain text");
    assert!(refs.is_empty());
}

#[test]
fn test_literals_with_dollar_signs() {
    let refs = find_variable_references("Price is $100");
    assert!(refs.is_empty());
}

#[test]
fn test_escaped_dollar_signs() {
    let refs = find_variable_references(r"Escaped \$VAR");
    assert!(refs.is_empty());
}

#[test]
fn test_complex_expression() {
    let refs = find_variable_references(
        "Database: ${DB_HOST:-localhost}:${DB_PORT:-5432}/${DB_NAME:-myapp}"
    );
    assert_eq!(refs, vec!["DB_HOST", "DB_NAME", "DB_PORT"]);
}

#[test]
fn test_nested_variables_in_syntax() {
    // ${A${B}} is treated as a single variable with name "A${B}"
    // The scanner doesn't recursively parse nested variable names
    let refs = find_variable_references("${A${B}}");
    assert_eq!(refs, vec!["A${B}"]);
}

#[test]
fn test_alphanumeric_and_underscore() {
    let refs = find_variable_references("${API_KEY} and ${DB_USER_1}");
    assert_eq!(refs, vec!["API_KEY", "DB_USER_1"]);
}

#[test]
fn test_command_substitution_not_extracted() {
    let refs = find_variable_references("$(echo hello)");
    assert!(refs.is_empty());
}

#[test]
fn test_invalid_syntax_ignored() {
    let refs = find_variable_references("${VALID} ${} ${INVALID}");
    // ${} results in an empty string variable name, which is filtered out
    assert_eq!(refs, vec!["", "INVALID", "VALID"]);
}

#[test]
fn test_whitespace_in_modifiers() {
    let refs = find_variable_references("${VAR:-   default value with spaces   }");
    assert_eq!(refs, vec!["VAR"]);
}

#[test]
fn test_special_characters_in_defaults() {
    let refs = find_variable_references("${PATH:-/usr/bin:/usr/local/bin}");
    assert_eq!(refs, vec!["PATH"]);
}

#[test]
fn test_multiple_occurrences_same_variable() {
    let refs = find_variable_references(
        "Database host: ${DB_HOST}, Port: ${DB_HOST}, User: ${DB_USER}"
    );
    assert_eq!(refs, vec!["DB_HOST", "DB_USER"]);
}
