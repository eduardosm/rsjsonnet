#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestParams {
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
            args: Vec::new(),
            no_color: true,
            exit_code: None,
        }
    }
}

#[inline]
fn true_() -> bool {
    true
}
