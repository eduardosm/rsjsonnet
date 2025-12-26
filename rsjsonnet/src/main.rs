#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications
)]
#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::path::Path;
use std::process::ExitCode;

use rsjsonnet_front::Session;
use rsjsonnet_lang::program::{Program, Thunk, Value};

mod cli;

#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    match main_inner() {
        Ok(()) => ExitCode::SUCCESS,
        Err(RunError::Generic) => ExitCode::from(1),
        Err(RunError::Usage) => ExitCode::from(2),
    }
}

enum RunError {
    Generic,
    Usage,
}

fn main_inner() -> Result<(), RunError> {
    let args = match <cli::Cli as clap::Parser>::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return Err(RunError::Usage);
        }
    };

    if args.string && args.yaml_stream {
        eprintln!("error: cannot use '-S' / '--string' together with '-y' / '--yaml-stream'");
        return Err(RunError::Usage);
    }

    enum Input<'a> {
        File(&'a Path),
        Virt(&'a str, Vec<u8>),
    }

    let input;
    if args.exec {
        #[cfg(unix)]
        let data = std::os::unix::ffi::OsStrExt::as_bytes(&*args.input).to_vec();
        #[cfg(not(unix))]
        let data = args.input.to_string_lossy().into_owned().into_bytes();
        input = Input::Virt("<cmdline>", data);
    } else if args.input == "-" {
        let mut data = Vec::new();
        match std::io::stdin().read_to_end(&mut data) {
            Ok(_) => {
                input = Input::Virt("<stdin>", data);
            }
            Err(e) => {
                eprintln!("failed to read stdin: {e}");
                return Err(RunError::Generic);
            }
        }
    } else {
        input = Input::File(Path::new(&args.input));
    }

    let arena = rsjsonnet_lang::arena::Arena::new();
    let mut session = Session::new(&arena);

    if let Some(max_stack) = args.max_stack {
        session.program_mut().set_max_stack(max_stack);
    }

    if let Some(max_trace) = args.max_trace {
        session.set_max_trace(max_trace);
    }

    session.set_colored_output(std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()));

    for path in args.jpath.iter().rev() {
        session.add_search_path(path.clone());
    }

    let root_thunk = match input {
        Input::File(input_path) => session.load_real_file(input_path),
        Input::Virt(input_repr_path, input_data) => {
            session.load_virt_file(input_repr_path, input_data)
        }
    };
    let Some(root_thunk) = root_thunk else {
        return Err(RunError::Generic);
    };

    let mut ext_names = HashSet::new();

    for arg in args.ext_str.iter() {
        let name = session.program().intern_str(&arg.var);
        if !ext_names.insert(name) {
            eprintln!(
                "error: external variable {:?} defined more than once",
                arg.var
            );
            return Err(RunError::Generic);
        }

        let thunk = ext_str_to_thunk(session.program_mut(), arg)?;
        session.program_mut().add_ext_var(name, &thunk);
    }

    for arg in args.ext_str_file.iter() {
        let name = session.program().intern_str(&arg.var);
        if !ext_names.insert(name) {
            eprintln!(
                "error: external variable {:?} defined more than once",
                arg.var
            );
            return Err(RunError::Generic);
        }

        let thunk = ext_str_file_to_thunk(session.program_mut(), arg)?;
        session.program_mut().add_ext_var(name, &thunk);
    }

    for arg in args.ext_code.iter() {
        let name = session.program().intern_str(&arg.var);
        if !ext_names.insert(name) {
            eprintln!(
                "error: external variable {:?} defined more than once",
                arg.var
            );
            return Err(RunError::Generic);
        }

        let thunk = ext_code_to_thunk(&mut session, "ext", arg)?;
        session.program_mut().add_ext_var(name, &thunk);
    }

    for arg in args.ext_code_file.iter() {
        let name = session.program().intern_str(&arg.var);
        if !ext_names.insert(name) {
            eprintln!(
                "error: external variable {:?} defined more than once",
                arg.var
            );
            return Err(RunError::Generic);
        }

        let thunk = ext_code_file_to_thunk(&mut session, arg)?;
        session.program_mut().add_ext_var(name, &thunk);
    }

    let mut tla_names = HashSet::new();
    let mut tla = Vec::new();

    for arg in args.tla_str.iter() {
        let name = session.program().intern_str(&arg.var);
        if !tla_names.insert(name) {
            eprintln!("error: TLA {:?} defined more than once", arg.var);
        }

        let thunk = ext_str_to_thunk(session.program_mut(), arg)?;
        tla.push((name, thunk));
    }

    for arg in args.tla_str_file.iter() {
        let name = session.program().intern_str(&arg.var);
        if !tla_names.insert(name) {
            eprintln!("error: TLA {:?} defined more than once", arg.var);
        }

        let thunk = ext_str_file_to_thunk(session.program_mut(), arg)?;
        tla.push((name, thunk));
    }

    for arg in args.tla_code.iter() {
        let name = session.program().intern_str(&arg.var);
        if !tla_names.insert(name) {
            eprintln!("error: TLA {:?} defined more than once", arg.var);
        }

        let thunk = ext_code_to_thunk(&mut session, "tla", arg)?;
        tla.push((name, thunk));
    }

    for arg in args.tla_code_file.iter() {
        let name = session.program().intern_str(&arg.var);
        if !tla_names.insert(name) {
            eprintln!("error: TLA {:?} defined more than once", arg.var);
        }

        let thunk = ext_code_file_to_thunk(&mut session, arg)?;
        tla.push((name, thunk));
    }

    session.push_custom_stack_trace_item("during top-level value evaluation".into());
    let Some(mut root_value) = session.eval_value(&root_thunk) else {
        return Err(RunError::Generic);
    };
    session.pop_custom_stack_trace_item();

    if root_value.is_function() {
        let func_thunk = session.program_mut().value_to_thunk(&root_value);
        session.push_custom_stack_trace_item("during top-level function call evaluation".into());
        let Some(call_value) = session.eval_call(&func_thunk, &[], &tla) else {
            return Err(RunError::Generic);
        };
        session.pop_custom_stack_trace_item();
        root_value = call_value;
    } else if !tla.is_empty() {
        eprintln!("error: top-level arguments provided, but root value is not a function");
        return Err(RunError::Generic);
    }

    let output = if let Some(ref dir_path) = args.multi {
        let dir_path = Path::new(dir_path);
        let Some(fields) = root_value.to_object() else {
            eprintln!("error: in multi mode, the top-level value must be an object");
            return Err(RunError::Generic);
        };
        let mut path_list = String::new();
        for (field_name, field_value) in fields.iter() {
            session.push_custom_stack_trace_item(format!(
                "during manifestation of object field {}",
                field_name.value(),
            ));
            let repr = value_to_repr(&args, &mut session, field_value)?;
            session.pop_custom_stack_trace_item();
            let path = dir_path.join(field_name.value());
            match std::fs::write(&path, repr.as_bytes()) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("error: failed to write {path:?}: {e}");
                    return Err(RunError::Generic);
                }
            }
            writeln!(path_list, "{}", path.display()).unwrap();
        }
        path_list
    } else {
        session.push_custom_stack_trace_item("during manifestation".into());
        let s = value_to_repr(&args, &mut session, &root_value)?;
        session.pop_custom_stack_trace_item();
        s
    };

    if let Some(output_path) = args.output {
        match std::fs::write(&output_path, output.as_bytes()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("failed to write {output_path:?}: {e}");
                return Err(RunError::Generic);
            }
        }
    } else {
        match std::io::stdout().write_all(output.as_bytes()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("failed to write to stdout: {e}");
                return Err(RunError::Generic);
            }
        }
    }

    Ok(())
}

fn ext_str_to_thunk<'p>(
    program: &mut Program<'p>,
    arg: &cli::VarOptVal,
) -> Result<Thunk<'p>, RunError> {
    let value = get_opt_val(arg)?;
    Ok(program.value_to_thunk(&Value::string(&value)))
}

fn ext_str_file_to_thunk<'p>(
    program: &mut Program<'p>,
    arg: &cli::VarFile,
) -> Result<Thunk<'p>, RunError> {
    let value = match std::fs::read(&arg.file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: failed to read {:?}: {e}", arg.file);
            return Err(RunError::Generic);
        }
    };
    let value = match String::from_utf8(value) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("error: {:?} is not valid UTF-8", arg.file);
            return Err(RunError::Generic);
        }
    };
    Ok(program.value_to_thunk(&Value::string(&value)))
}

fn ext_code_to_thunk<'p>(
    session: &mut Session<'p>,
    prefix: &str,
    arg: &cli::VarOptVal,
) -> Result<Thunk<'p>, RunError> {
    let code = get_opt_val(arg)?;
    let virt_path = format!("<{prefix}:{}>", arg.var);
    session
        .load_virt_file(&virt_path, code.into_owned().into())
        .ok_or(RunError::Generic)
}

fn ext_code_file_to_thunk<'p>(
    session: &mut Session<'p>,
    arg: &cli::VarFile,
) -> Result<Thunk<'p>, RunError> {
    session
        .load_real_file(Path::new(&arg.file))
        .ok_or(RunError::Generic)
}

fn get_opt_val(arg: &cli::VarOptVal) -> Result<Cow<'_, str>, RunError> {
    if let Some(ref value) = arg.val {
        Ok(Cow::Borrowed(value))
    } else if let Some(value) = std::env::var_os(&arg.var) {
        match value.into_string() {
            Ok(v) => Ok(Cow::Owned(v)),
            Err(v) => {
                eprintln!(
                    "error: value of environment variable {:?} is not valid unicode: {v:?}",
                    arg.var,
                );
                Err(RunError::Generic)
            }
        }
    } else {
        eprintln!("error: environment variable {:?} is not defined", arg.var);
        Err(RunError::Generic)
    }
}

fn value_to_repr<'p>(
    args: &cli::Cli,
    session: &mut Session<'p>,
    value: &Value<'p>,
) -> Result<String, RunError> {
    if args.string {
        let Some(s) = value.to_string() else {
            session.print_error("in string mode, the value must be a string");
            return Err(RunError::Generic);
        };
        Ok(s + "\n")
    } else if args.yaml_stream {
        let Some(items) = value.to_array() else {
            session.print_error("in YAML stream mode, the value must be an array");
            return Err(RunError::Generic);
        };
        let mut manifested_items = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            session.push_custom_stack_trace_item(format!("during manifestation of array item {i}"));
            let Some(item_s) = session.manifest_json(item, true) else {
                return Err(RunError::Generic);
            };
            session.pop_custom_stack_trace_item();
            manifested_items.push(item_s);
        }
        if manifested_items.is_empty() {
            Ok(String::new())
        } else {
            let mut yaml = String::new();
            for manifested_item in manifested_items.iter() {
                yaml.push_str("---\n");
                yaml.push_str(manifested_item);
                yaml.push('\n');
            }
            yaml.push_str("...\n");
            Ok(yaml)
        }
    } else {
        let Some(mut s) = session.manifest_json(value, true) else {
            return Err(RunError::Generic);
        };
        s.push('\n');
        Ok(s)
    }
}
