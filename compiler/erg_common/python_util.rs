//! utilities for calling CPython.
//!
//! CPythonを呼び出すためのユーティリティー
use std::process::Command;

use crate::serialize::get_magic_num_from_bytes;

pub fn which_python() -> String {
    let python = if cfg!(windows) { "python" } else { "python3" };
    let out = Command::new("which")
        .arg(python)
        .output()
        .expect("python not found");
    let res = String::from_utf8(out.stdout)
        .unwrap()
        .replace('\n', "")
        .replace('\r', "");
    if res.is_empty() {
        panic!("python not found");
    }
    res
}

pub fn detect_magic_number() -> u32 {
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(which_python())
            .arg("-c")
            .arg("import importlib.util as util;print(util.MAGIC_NUMBER.hex())")
            .output()
            .expect("cannot get the magic number from python")
    } else {
        let python_command = format!(
            "{} -c 'import importlib.util as util;print(util.MAGIC_NUMBER.hex())'",
            which_python()
        );
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .output()
            .expect("cannot get the magic number from python")
    };
    let s_hex_magic_num = String::from_utf8(out.stdout).unwrap();
    let first_byte = u8::from_str_radix(&s_hex_magic_num[0..=1], 16).unwrap();
    let second_byte = u8::from_str_radix(&s_hex_magic_num[2..=3], 16).unwrap();
    get_magic_num_from_bytes(&[first_byte, second_byte, 0, 0])
}

/// executes over a shell, cause `python` may not exist as an executable file (like pyenv)
pub fn exec_pyc<S: Into<String>>(file: S) {
    let mut out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(which_python())
            .arg(&file.into())
            .spawn()
            .expect("cannot execute python")
    } else {
        let python_command = format!("{} {}", which_python(), file.into());
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .spawn()
            .expect("cannot execute python")
    };
    out.wait().expect("python doesn't work");
}

/// evaluates over a shell, cause `python` may not exist as an executable file (like pyenv)
pub fn eval_pyc<S: Into<String>>(file: S) -> String {
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(which_python())
            .arg(&file.into())
            .spawn()
            .expect("cannot execute python")
    } else {
        let python_command = format!("{} {}", which_python(), file.into());
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .spawn()
            .expect("cannot execute python")
    };
    let out = out.wait_with_output().expect("python doesn't work");
    String::from_utf8(out.stdout).expect("failed to decode python output")
}

pub fn exec_py(code: &str) {
    if cfg!(windows) {
        Command::new(which_python())
            .arg("-c")
            .arg(code)
            .spawn()
            .expect("cannot execute python");
    } else {
        let python_command = format!("{} -c \"{}\"", which_python(), code);
        Command::new("sh")
            .arg("-c")
            .arg(python_command)
            .spawn()
            .expect("cannot execute python");
    }
}
