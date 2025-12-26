mod common;
use common::create_germi;
use germi::Error;

#[test]
fn test_simple_variable() {
    let germi = create_germi();
    let result = germi.interpolate("Value is ${TEST_VAR}").unwrap();
    assert_eq!(result, "Value is test_value");
}

#[test]
fn test_nested_variable() {
    let germi = create_germi();
    let result = germi.interpolate("Value is ${NESTED_VAR}").unwrap();
    assert_eq!(result, "Value is test_value");
}

#[test]
fn test_missing_variable() {
    let germi = create_germi();
    let result = germi.interpolate("Value is ${MISSING_VAR}");
    match result {
        Err(Error::MissingVar(var)) => assert_eq!(var, "MISSING_VAR"),
        _ => panic!("Expected MissingVar error"),
    }
}

#[test]
fn test_recursive_loop() {
    let mut germi = create_germi();
    germi.add_variable("A", "${B}");
    germi.add_variable("B", "${A}");
    
    let result = germi.interpolate("${A}");
    match result {
        Err(Error::RecursiveLookup(_)) => {},
        _ => panic!("Expected RecursiveLookup error"),
    }
}

#[test]
fn test_variables_disabled() {
    use germi::Config;
    let mut config = Config::default();
    config.features.variables = false;
    
    let germi = common::create_germi_with_config(config);
    let result = germi.interpolate("${TEST_VAR}").unwrap();
    assert_eq!(result, "${TEST_VAR}");
}

#[test]
fn test_variable_overwrite() {
    let mut germi = common::create_germi();
    germi.add_variable("TEST_VAR", "original");
    germi.add_variable("TEST_VAR", "overwritten");
    
    let result = germi.interpolate("Value is ${TEST_VAR}").unwrap();
    assert_eq!(result, "Value is overwritten");
}
