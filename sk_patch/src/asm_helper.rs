use std::{
    env::{self},
    fs::{DirBuilder, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Output},
};

use anyhow::{anyhow, Ok};

#[cfg(not(windows))]
pub const LINE_ENDLING: &'static str = "\n";

#[cfg(windows)]
pub const LINE_ENDLING: &'static str = "\r\n";

#[macro_export]
macro_rules! aarch64 {
    ($expr:expr) => {{
        let mut builder = String::new();
        builder.push_str($expr);
        builder.push_str(LINE_ENDLING);
        builder
    }};
    ($expr:expr;) => {{
        let mut builder = String::new();
        builder.push_str($expr);
        builder.push_str(LINE_ENDLING);
        builder
    }};
    ($($($expr:expr),+);+) => {{
        let mut builder = String::new();
        $(
            let fmt = format!($($expr, )+);
            builder.push_str(&fmt);
            builder.push_str(LINE_ENDLING);
        )+
        builder
    }};
    ($($($expr:expr),+);+;) => {{
        let mut builder = String::new();
        $(
            let fmt = format!($($expr, )+);
            builder.push_str(&fmt);
            builder.push_str(LINE_ENDLING);
        )+
        builder
    }};
}

pub fn asm_to_be_bytes(asm_text: &str) -> anyhow::Result<Vec<u8>> {
    let current_dir = env::current_dir()?;
    let temp_dir = &current_dir.join("temp");
    let _ = DirBuilder::new().recursive(true).create(temp_dir);

    let temp = temp_dir;
    let input = &temp.join("input.txt");
    let mut input_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(input)?;
    input_file.write_all(asm_text.as_bytes())?;

    let output = &temp.join("output.txt");
    {
        let _ = OpenOptions::new().create(true).truncate(true).open(output);
    }

    let output = run_aarch64_as(input.to_str().unwrap())?;

    if output.status.code().unwrap() == 0 {
        read_bytes_from(&output)
    } else {
        Err(anyhow!("output does not exist"))
    }
}

fn read_bytes_from(output: &Output) -> anyhow::Result<Vec<u8>> {
    let mut bytes = vec![];
    let lines = BufReader::new(output.stdout.as_slice()).lines();
    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with("AARCH64") {
            continue;
        }
        let fragments = line.split_ascii_whitespace().collect::<Vec<_>>();
        let hex = fragments[2];
        let mut inst_be_bytes = u32::from_str_radix(hex, 16)?.to_be_bytes().to_vec();
        bytes.append(&mut inst_be_bytes);
    }
    Ok(bytes)
}

fn run_aarch64_as(input_file: &str) -> anyhow::Result<Output> {
    let my_path = env!("PATH");
    let ndk_home = PathBuf::from(env!("NDK_HOME"));
    let prebuilt_bin = if cfg!(target_os = "linux") {
        ndk_home.join("toolchains/aarch64-linux-android-4.9/prebuilt/linux-x86_64/bin")
    } else {
        ndk_home.join("toolchains/aarch64-linux-android-4.9/prebuilt/windows-x86_64/bin")
    };
    let mut new_path = String::from(prebuilt_bin.to_str().unwrap());
    new_path.push(':');
    new_path.push_str(my_path);
    let mut cmd = Command::new("aarch64-linux-android-as");
    Ok(cmd
        .arg("-ahlm")
        .arg(input_file)
        .env("PATH", &new_path)
        .output()?)
}
