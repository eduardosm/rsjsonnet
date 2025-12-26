use std::fmt::Write as _;
use std::path::Path;

use crate::defs;

pub(crate) fn run(
    root_path: &Path,
    test_subpath: &Path,
    cmd_bin_path: &Path,
    bless: bool,
) -> Result<(), String> {
    let test_path = root_path.join(test_subpath);
    let test_dir = test_path.parent().unwrap();
    let test_name = test_path.file_stem().unwrap();

    let src =
        std::fs::read(&test_path).map_err(|e| format!("failed to read {test_path:?}: {e}"))?;
    let test_params = defs::TestParams::from_source(&src)
        .map_err(|e| format!("failed to parse params from {test_path:?}: {e}"))?;

    let mut stdout_name = test_name.to_os_string();
    stdout_name.push(".stdout");
    let stdout_path = test_dir.join(stdout_name);

    let mut stderr_name = test_name.to_os_string();
    stderr_name.push(".stderr");
    let stderr_path = test_dir.join(stderr_name);

    let expected_stderr = match std::fs::read(&stderr_path) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("failed to read {stderr_path:?}: {e}")),
    };

    let expected_stdout = match std::fs::read(&stdout_path) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if expected_stderr.is_empty() {
                b"true\n".to_vec()
            } else {
                Vec::new()
            }
        }
        Err(e) => return Err(format!("failed to read {stdout_path:?}: {e}")),
    };

    let mut expected_exit_code = test_params
        .exit_code
        .unwrap_or(if expected_stderr.is_empty() { 0 } else { 1 });

    let test_dir = test_path.parent().unwrap();
    let test_file_name = test_path.file_name().unwrap();

    let cmd_output = std::process::Command::new(cmd_bin_path)
        .current_dir(test_dir)
        .env_remove("NO_COLOR")
        .envs(test_params.no_color.then_some(("NO_COLOR", "1")))
        .args(&test_params.args)
        .arg(test_file_name)
        .output()
        .map_err(|e| format!("failed to execute {cmd_bin_path:?}: {e}"))?;

    if bless && test_params.exit_code.is_none() {
        expected_exit_code = if cmd_output.stderr.is_empty() { 0 } else { 1 };
    }

    if cmd_output
        .status
        .code()
        .is_none_or(|code| code != i32::from(expected_exit_code))
    {
        let mut err = String::new();
        if let Some(exit_code) = cmd_output.status.code() {
            writeln!(err, "process terminated with exit code {exit_code}").unwrap();
        } else {
            writeln!(
                err,
                "process terminated with status {:?}\n",
                cmd_output.status,
            )
            .unwrap();
        }
        writeln!(
            err,
            "stdout:\n{}",
            String::from_utf8_lossy(&cmd_output.stdout),
        )
        .unwrap();
        writeln!(
            err,
            "stderr:\n{}",
            String::from_utf8_lossy(&cmd_output.stderr),
        )
        .unwrap();
        return Err(err);
    }

    if bless {
        if (!cmd_output.stderr.is_empty() && cmd_output.stdout.is_empty())
            || (cmd_output.stderr.is_empty() && cmd_output.stdout == b"true\n")
        {
            if stdout_path
                .try_exists()
                .map_err(|e| format!("failed to check {stdout_path:?}: {e}"))?
            {
                std::fs::remove_file(&stdout_path)
                    .map_err(|e| format!("failed to remove {stdout_path:?}: {e}"))?;
            }
        } else {
            std::fs::write(&stdout_path, &cmd_output.stdout)
                .map_err(|e| format!("failed to write {stdout_path:?}: {e}"))?;
        }

        if cmd_output.stderr.is_empty() {
            if stderr_path
                .try_exists()
                .map_err(|e| format!("failed to check {stderr_path:?}: {e}"))?
            {
                std::fs::remove_file(&stderr_path)
                    .map_err(|e| format!("failed to remove {stderr_path:?}: {e}"))?;
            }
        } else {
            std::fs::write(&stderr_path, &cmd_output.stderr)
                .map_err(|e| format!("failed to write {stderr_path:?}: {e}"))?;
        }
    } else if cmd_output.stdout != expected_stdout || cmd_output.stderr != expected_stderr {
        let mut err = String::new();
        if cmd_output.stdout != expected_stdout {
            let stdout_path_repr = format!("{}.stdout", test_subpath.display());
            let stdout_diff = unified_diff::diff(
                &expected_stdout,
                &stdout_path_repr,
                &cmd_output.stdout,
                &stdout_path_repr,
                3,
            );
            writeln!(
                err,
                "stdout diff:\n{}",
                String::from_utf8_lossy(&stdout_diff),
            )
            .unwrap();
        }
        if cmd_output.stderr != expected_stderr {
            let stderr_path_repr = format!("{}.stderr", test_subpath.display());
            let stderr_diff = unified_diff::diff(
                &expected_stderr,
                &stderr_path_repr,
                &cmd_output.stderr,
                &stderr_path_repr,
                3,
            );
            writeln!(
                err,
                "stderr diff:\n{}",
                String::from_utf8_lossy(&stderr_diff),
            )
            .unwrap();
        }
        return Err(err);
    }

    Ok(())
}
