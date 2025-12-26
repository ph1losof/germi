mod common;
use common::create_germi;

#[test]
fn test_basic_escapes() {
    let germi = create_germi();
    let result = germi.interpolate(r#"Line1\nLine2"#).unwrap();
    assert_eq!(result, "Line1\nLine2");
    
    let result = germi.interpolate(r#"Tab\tSpace"#).unwrap();
    assert_eq!(result, "Tab\tSpace");
}

#[test]
fn test_escaped_variable() {
    let germi = create_germi();
    let result = germi.interpolate(r#"\${TEST_VAR}"#).unwrap();
    assert_eq!(result, "${TEST_VAR}");
}

#[test]
fn test_escapes_disabled() {
    use germi::Config;
    let mut config = Config::default();
    config.features.escapes = false;
    let germi = common::create_germi_with_config(config);
    
    let result = germi.interpolate(r#"Line1\nLine2"#).unwrap();
    assert_eq!(result, r#"Line1\nLine2"#); // Literal backslash
}
