#![cfg(feature = "async")]
mod common;
use common::create_germi;
use germi::Error;

#[tokio::test]
async fn test_command_execution() {
    let germi = create_germi();
    let result = germi.interpolate_async("Echo says: $(echo 'hello')").await.unwrap();
    assert_eq!(result, "Echo says: hello");
}

#[tokio::test]
async fn test_command_with_variable() {
    let germi = create_germi();
    // Assuming command runs in shell so it might access environment?
    // Actually our execute_command implementation just runs the string passed.
    // It doesn't propagate the internal context as env vars automatically unless we implemented that.
    // But we can interpolate variables INTO the command string first! Let's verify that flow.
    // If input is " $(echo ${TEST_VAR}) ", first pass resolves ${TEST_VAR} -> "test_value"
    // Second pass sees " $(echo test_value) " -> executes "echo test_value"
    
    let result = germi.interpolate_async("Value: $(echo ${TEST_VAR})").await.unwrap();
    assert_eq!(result, "Value: test_value");
}

#[tokio::test]
async fn test_command_error() {
    let germi = create_germi();
    let result = germi.interpolate_async("$(non_existent_command_12345)").await;
    assert!(matches!(result, Err(Error::CommandError(_)) | Err(Error::IoError(_))));
}

#[tokio::test]
async fn test_commands_disabled() {
    use germi::Config;
    let mut config = Config::default();
    config.features.commands = false;
    let germi = common::create_germi_with_config(config);
    
    let result = germi.interpolate_async("$(echo hi)").await.unwrap();
    assert_eq!(result, "$(echo hi)");
}
