#![cfg(feature = "async")]
mod common;
use common::create_germi;
use germi::Error;

#[tokio::test]
async fn test_backtick_simple() {
    let germi = create_germi();
    let result = germi.interpolate_async("Date: `echo 2024`").await.unwrap();
    assert_eq!(result, "Date: 2024");
}

#[tokio::test]
async fn test_backtick_equivalent_to_dollar_paren() {
    let germi = create_germi();

    let backtick = germi.interpolate_async("`echo hello`").await.unwrap();
    let dollar = germi.interpolate_async("$(echo hello)").await.unwrap();

    assert_eq!(backtick, dollar);
    assert_eq!(backtick, "hello");
}

#[tokio::test]
async fn test_backtick_with_variable() {
    let germi = create_germi();

    let result = germi
        .interpolate_async("`echo hello` ${TEST_VAR}")
        .await
        .unwrap();
    assert_eq!(result, "hello test_value");
}

#[tokio::test]
async fn test_backtick_variable_inside_command() {
    let germi = create_germi();

    // Variables inside backtick commands should be resolved first
    let result = germi.interpolate_async("`echo ${TEST_VAR}`").await.unwrap();
    assert_eq!(result, "test_value");
}

#[tokio::test]
async fn test_backtick_escaped() {
    let germi = create_germi();
    // \` should produce literal backtick, not command
    let result = germi
        .interpolate_async(r"Use \`backticks\` here")
        .await
        .unwrap();
    assert_eq!(result, "Use `backticks` here");
}

#[tokio::test]
async fn test_backtick_disabled() {
    use germi::Config;
    let mut config = Config::default();
    config.features.backtick_commands = false;
    let germi = common::create_germi_with_config(config);

    let result = germi.interpolate_async("`whoami`").await.unwrap();
    // When disabled, backticks are treated as literal
    assert_eq!(result, "`whoami`");
}

#[tokio::test]
async fn test_backtick_command_error() {
    let germi = create_germi();
    let result = germi.interpolate_async("`non_existent_command_12345`").await;
    assert!(matches!(
        result,
        Err(Error::CommandError(_)) | Err(Error::IoError(_))
    ));
}

#[test]
fn test_unclosed_backtick() {
    let germi = create_germi();
    let result = germi.interpolate("`unclosed");
    assert!(matches!(result, Err(Error::SyntaxError(_, _))));
}

#[tokio::test]
async fn test_multiple_backtick_commands() {
    let germi = create_germi();
    let result = germi
        .interpolate_async("`echo foo` and `echo bar`")
        .await
        .unwrap();
    assert_eq!(result, "foo and bar");
}

#[tokio::test]
async fn test_mixed_backtick_and_dollar_paren() {
    let germi = create_germi();
    let result = germi
        .interpolate_async("`echo backtick` and $(echo dollar)")
        .await
        .unwrap();
    assert_eq!(result, "backtick and dollar");
}

#[tokio::test]
async fn test_backtick_with_spaces_and_quotes() {
    let germi = create_germi();
    let result = germi
        .interpolate_async(r#"`echo "hello world"`"#)
        .await
        .unwrap();
    assert_eq!(result, "hello world");
}

#[tokio::test]
async fn test_backtick_preserves_output_format() {
    let germi = create_germi();
    // Trailing newlines should be trimmed (like $(cmd) behavior)
    let result = germi.interpolate_async("`printf 'test'`").await.unwrap();
    assert_eq!(result, "test");
}

#[tokio::test]
async fn test_escaped_dollar_paren_not_executed() {
    let germi = create_germi();
    // \$(...) should produce literal $(...)
    let result = germi
        .interpolate_async(r"\$(echo should not run)")
        .await
        .unwrap();
    assert_eq!(result, "$(echo should not run)");
}

#[tokio::test]
async fn test_mixed_escaped_and_unescaped() {
    let germi = create_germi();
    // Mix of escaped and real commands
    let result = germi
        .interpolate_async(r"\`literal\` and `echo real`")
        .await
        .unwrap();
    assert_eq!(result, "`literal` and real");
}
