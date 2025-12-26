mod common;
use common::create_germi;

#[test]
fn test_default_value() {
    let germi = create_germi();
    // ${VAR:-default} - use default if unset or empty
    let result = germi.interpolate("${MISSING:-default_val}").unwrap();
    assert_eq!(result, "default_val");
    
    let result = germi.interpolate("${EMPTY_VAR:-default_val}").unwrap();
    assert_eq!(result, "default_val");
    
    let result = germi.interpolate("${TEST_VAR:-default_val}").unwrap();
    assert_eq!(result, "test_value");
}

#[test]
fn test_alternate_value() {
    let germi = create_germi();
    // ${VAR-default} - use default only if unset (empty is valid value)
    let result = germi.interpolate("${MISSING-default_val}").unwrap();
    assert_eq!(result, "default_val");
    
    let result = germi.interpolate("${EMPTY_VAR-default_val}").unwrap();
    assert_eq!(result, ""); // Empty is preserved
    
    let result = germi.interpolate("${TEST_VAR-default_val}").unwrap();
    assert_eq!(result, "test_value");
}

#[test]
fn test_conditional_value() {
    let germi = create_germi();
    // ${VAR:+replacement} - use replacement if set and not empty (usually)
    // Actually spec says:
    // :+ -> If var set and not null/empty, substitute word. Otherwise null.
    
    let result = germi.interpolate("${TEST_VAR:+replaced}").unwrap();
    assert_eq!(result, "replaced");
    
    let result = germi.interpolate("${MISSING:+replaced}").unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_defaults_disabled() {
    use germi::Config;
    let mut config = Config::default();
    config.features.defaults = false;
    let germi = common::create_germi_with_config(config);
    
    // Should ignore default syntax for unset vars -> Error
    let result = germi.interpolate("${MISSING:-default}");
    assert!(result.is_err());
}

// Matrix Tests:
// State: SET(val), UNSET, EMPTY
// Op: :- (Default Strict), - (Default Null), :+ (Cond Strict), + (Cond Null)

#[test]
fn test_matrix_set_strict_default() { // ${VAR:-def} where VAR="val" -> "val"
    let germi = create_germi();
    let res = germi.interpolate("${TEST_VAR:-def}").unwrap();
    assert_eq!(res, "test_value");
}
#[test]
fn test_matrix_unset_strict_default() { // ${VAR:-def} where VAR unset -> "def"
    let germi = create_germi();
    let res = germi.interpolate("${MISSING:-def}").unwrap();
    assert_eq!(res, "def");
}
#[test]
fn test_matrix_empty_strict_default() { // ${VAR:-def} where VAR="" -> "def"
    let germi = create_germi();
    let res = germi.interpolate("${EMPTY_VAR:-def}").unwrap();
    assert_eq!(res, "def");
}

#[test]
fn test_matrix_set_loose_default() { // ${VAR-def} where VAR="val" -> "val"
    let germi = create_germi();
    let res = germi.interpolate("${TEST_VAR-def}").unwrap();
    assert_eq!(res, "test_value");
}
#[test]
fn test_matrix_unset_loose_default() { // ${VAR-def} where VAR unset -> "def"
    let germi = create_germi();
    let res = germi.interpolate("${MISSING-def}").unwrap();
    assert_eq!(res, "def");
}
#[test]
fn test_matrix_empty_loose_default() { // ${VAR-def} where VAR="" -> "" (empty valid)
    let germi = create_germi();
    let res = germi.interpolate("${EMPTY_VAR-def}").unwrap();
    assert_eq!(res, "");
}

#[test]
fn test_matrix_set_strict_conditional() { // ${VAR:+rep} where VAR="val" -> "rep"
    let germi = create_germi();
    let res = germi.interpolate("${TEST_VAR:+rep}").unwrap();
    assert_eq!(res, "rep");
}
#[test]
fn test_matrix_unset_strict_conditional() { // ${VAR:+rep} where VAR unset -> ""
    let germi = create_germi();
    let res = germi.interpolate("${MISSING:+rep}").unwrap();
    assert_eq!(res, "");
}
#[test]
fn test_matrix_empty_strict_conditional() { // ${VAR:+rep} where VAR="" -> "" (empty treated as false)
    let germi = create_germi();
    let res = germi.interpolate("${EMPTY_VAR:+rep}").unwrap();
    assert_eq!(res, "");
}

#[test]
fn test_matrix_set_loose_conditional() { // ${VAR+rep} where VAR="val" -> "rep"
    let germi = create_germi();
    let res = germi.interpolate("${TEST_VAR+rep}").unwrap();
    assert_eq!(res, "rep");
}
#[test]
fn test_matrix_unset_loose_conditional() { // ${VAR+rep} where VAR unset -> ""
    let germi = create_germi();
    let res = germi.interpolate("${MISSING+rep}").unwrap();
    assert_eq!(res, "");
}
#[test]
fn test_matrix_empty_loose_conditional() { // ${VAR+rep} where VAR="" -> "rep" (empty is "set")
    let germi = create_germi();
    let res = germi.interpolate("${EMPTY_VAR+rep}").unwrap();
    assert_eq!(res, "rep");
}
