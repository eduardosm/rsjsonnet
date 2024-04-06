use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
pub(crate) struct Cli {
    #[clap(value_name = "filename")]
    pub(crate) input: OsString,
    #[clap(long = "exec", short = 'e', help = "Treat filename as code")]
    pub(crate) exec: bool,
    #[clap(
        long = "jpath",
        short = 'J',
        value_name = "dir",
        help = "Specify an additional library search dir (right-most wins)"
    )]
    pub(crate) jpath: Vec<PathBuf>,
    #[clap(
        long = "output-file",
        short = 'o',
        value_name = "file",
        help = "Write to the output file rather than stdout"
    )]
    pub(crate) output: Option<PathBuf>,
    #[clap(
        long = "multi",
        short = 'm',
        value_name = "dir",
        help = "Write multiple files to the directory, list files on stdout"
    )]
    pub(crate) multi: Option<PathBuf>,
    #[clap(
        long = "yaml-stream",
        short = 'y',
        help = "Write output as a YAML stream of JSON documents"
    )]
    pub(crate) yaml_stream: bool,
    #[clap(
        long = "string",
        short = 'S',
        help = "Expect a string, manifest as plain text"
    )]
    pub(crate) string: bool,
    #[clap(
        long = "max-stack",
        short = 's',
        help = "Number of allowed stack frames",
        value_name = "n"
    )]
    pub(crate) max_stack: Option<usize>,
    #[clap(
        long = "max-trace",
        short = 't',
        help = "Max length of stack trace before cropping",
        value_name = "n"
    )]
    pub(crate) max_trace: Option<usize>,
    #[clap(long = "ext-str", short = 'V', value_name = "var=[val]")]
    pub(crate) ext_str: Vec<VarOptVal>,
    #[clap(long = "ext-str-file", value_name = "var=file")]
    pub(crate) ext_str_file: Vec<VarFile>,
    #[clap(long = "ext-code", value_name = "var[=code]")]
    pub(crate) ext_code: Vec<VarOptVal>,
    #[clap(long = "ext-code-file", value_name = "var=file")]
    pub(crate) ext_code_file: Vec<VarFile>,
    #[clap(long = "tla-str", short = 'A', value_name = "var[=val]")]
    pub(crate) tla_str: Vec<VarOptVal>,
    #[clap(long = "tla-str-file", value_name = "var=file")]
    pub(crate) tla_str_file: Vec<VarFile>,
    #[clap(long = "tla-code", value_name = "var[=code]")]
    pub(crate) tla_code: Vec<VarOptVal>,
    #[clap(long = "tla-code-file", value_name = "var=file")]
    pub(crate) tla_code_file: Vec<VarFile>,
}

#[derive(Clone, Debug)]
pub(crate) struct VarFile {
    pub(crate) var: String,
    // FIXME: This should be `PathBuf`
    pub(crate) file: String,
}

impl std::str::FromStr for VarFile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((key, path)) = s.split_once('=') {
            Ok(Self {
                var: key.into(),
                file: path.into(),
            })
        } else {
            Err("argument not in form 'var=file'".into())
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct VarOptVal {
    pub(crate) var: String,
    pub(crate) val: Option<String>,
}

impl From<&str> for VarOptVal {
    fn from(s: &str) -> Self {
        if let Some((key, val)) = s.split_once('=') {
            Self {
                var: key.into(),
                val: Some(val.into()),
            }
        } else {
            Self {
                var: s.into(),
                val: None,
            }
        }
    }
}
