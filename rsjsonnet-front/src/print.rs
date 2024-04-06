#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum TextPartKind {
    Space,
    Margin,
    Path,
    MainMessage,
    TextNormal,
    TextAlt,
    NoteLabel,
    ErrorTextNormal,
    ErrorTextAlt,
    ErrorLabel,
}

pub(crate) fn output_stderr_plain(parts: &[(String, TextPartKind)]) {
    fn inner(parts: &[(String, TextPartKind)]) -> Result<(), std::io::Error> {
        use std::io::Write as _;

        let mut stderr = std::io::stderr().lock();
        for (text, _) in parts.iter() {
            stderr.write_all(text.as_bytes())?;
        }
        stderr.flush()
    }
    inner(parts).expect("failed to write to stderr");
}

#[cfg(feature = "crossterm")]
pub(crate) fn output_stderr_colored(parts: &[(String, TextPartKind)]) {
    fn inner(parts: &[(String, TextPartKind)]) -> Result<(), std::io::Error> {
        use std::io::Write as _;

        use crossterm::style::{ContentStyle, Print, ResetColor, SetStyle, Stylize as _};

        let mut stderr = std::io::stderr().lock();
        let mut last_style = None;
        for (text, kind) in parts.iter() {
            let style = match kind {
                TextPartKind::Space => None,
                TextPartKind::Margin => Some(ContentStyle::new().blue().bold()),
                TextPartKind::Path => None,
                TextPartKind::MainMessage => Some(ContentStyle::new().white().bold()),
                TextPartKind::TextNormal => None,
                TextPartKind::TextAlt => Some(ContentStyle::new().white().reverse()),
                TextPartKind::NoteLabel => Some(ContentStyle::new().dark_green().bold()),
                TextPartKind::ErrorTextNormal => Some(ContentStyle::new().red()),
                TextPartKind::ErrorTextAlt => Some(ContentStyle::new().red().reverse()),
                TextPartKind::ErrorLabel => Some(ContentStyle::new().red().bold()),
            };
            if last_style != style {
                if let Some(style) = style {
                    crossterm::queue!(stderr, SetStyle(style))?;
                } else {
                    crossterm::queue!(stderr, ResetColor)?;
                }
                last_style = style;
            }
            crossterm::queue!(stderr, Print(text))?;
        }
        if last_style.is_some() {
            crossterm::queue!(stderr, ResetColor)?;
        }
        stderr.flush()
    }
    inner(parts).expect("failed to write to stderr");
}
