use germi::{Germi, Config};

#[allow(dead_code)]
pub fn create_germi() -> Germi {
    let mut germi = Germi::default();
    germi.add_variable("TEST_VAR", "test_value");
    germi.add_variable("NESTED_VAR", "${TEST_VAR}");
    germi.add_variable("EMPTY_VAR", "");
    germi
}

#[allow(dead_code)]
pub fn create_germi_with_config(config: Config) -> Germi {
    let mut germi = Germi::with_config(config);
    germi.add_variable("TEST_VAR", "test_value");
    germi.add_variable("NESTED_VAR", "${TEST_VAR}");
    germi.add_variable("EMPTY_VAR", "");
    germi
}
