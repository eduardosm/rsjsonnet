pub(crate) struct TestParams {
    pub(crate) args: Vec<String>,
    pub(crate) no_color: bool,
    pub(crate) exit_code: Option<u8>,
}

impl TestParams {
    pub(crate) fn from_source(src: &[u8]) -> Result<Self, String> {
        let mut this = Self {
            args: Vec::new(),
            no_color: true,
            exit_code: None,
        };

        fn trim_ascii_start(s: &str) -> &str {
            s.trim_start_matches(|c: char| c.is_ascii_whitespace())
        }

        fn trim_ascii(s: &str) -> &str {
            s.trim_matches(|c: char| c.is_ascii_whitespace())
        }

        let cmds = extract_commands(src)?;
        for cmd in cmds.iter() {
            let cmd = trim_ascii_start(cmd);
            if let Some(args) = cmd.strip_prefix("args:") {
                this.args = shlex::split(args).ok_or_else(|| format!("invalid args: {args:?}"))?;
            } else if let Some(no_color) = cmd.strip_prefix("no-color:") {
                let no_color = trim_ascii(no_color);
                this.no_color = no_color
                    .parse()
                    .map_err(|_| format!("invalid no-color: {no_color:?}"))?;
            } else if let Some(exit_code) = cmd.strip_prefix("exit-code:") {
                let exit_code = trim_ascii(exit_code);
                this.exit_code = Some(
                    exit_code
                        .parse()
                        .map_err(|_| format!("invalid exit-code: {exit_code:?}"))?,
                );
            } else {
                return Err(format!("unknown command: {cmd:?}"));
            }
        }

        Ok(this)
    }
}

fn extract_commands(input: &[u8]) -> Result<Vec<String>, String> {
    let mut rem = input;
    let mut commands = Vec::new();
    loop {
        rem = strip_sp_and_nl(rem);
        if let Some(data) = rem.strip_prefix(b"//") {
            let line;
            (line, rem) = split_line(data);
            if let Some(command) = line.strip_prefix(b"@") {
                let command = std::str::from_utf8(command).map_err(|_| "invalid UTF-8")?;
                commands.push(command.into());
            }
        } else if let Some(data) = rem.strip_prefix(b"#") {
            let line;
            (line, rem) = split_line(data);
            if let Some(command) = line.strip_prefix(b"@") {
                let command = std::str::from_utf8(command).map_err(|_| "invalid UTF-8")?;
                commands.push(command.into());
            }
        } else {
            break;
        }
    }
    Ok(commands)
}

fn strip_sp_and_nl(mut s: &[u8]) -> &[u8] {
    while matches!(s.first(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        s = &s[1..];
    }
    s
}

fn split_line(s: &[u8]) -> (&[u8], &[u8]) {
    if let Some(nl_pos) = s.iter().position(|&c| c == b'\n') {
        (&s[..nl_pos], &s[(nl_pos + 1)..])
    } else {
        (s, b"")
    }
}
