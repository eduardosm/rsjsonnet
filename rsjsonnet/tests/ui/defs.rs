#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestParams {
    #[serde(rename = "not-test", default = "false_")]
    pub(crate) not_test: bool,
    #[serde(default = "Vec::new")]
    pub(crate) args: Vec<String>,
    #[serde(rename = "no-color", default = "true_")]
    pub(crate) no_color: bool,
    #[serde(rename = "exit-code")]
    pub(crate) exit_code: Option<u8>,
}

impl Default for TestParams {
    fn default() -> Self {
        Self {
            not_test: false,
            args: Vec::new(),
            no_color: true,
            exit_code: None,
        }
    }
}

#[inline]
fn false_() -> bool {
    false
}

#[inline]
fn true_() -> bool {
    true
}
