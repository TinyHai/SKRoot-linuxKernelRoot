use anyhow::{anyhow, Ok};


use getopts::Options;
use std::{
    ffi::{CStr, CString},
    os::unix::process::CommandExt,
    process::Command,
};
use log::trace;

use crate::utils;
use crate::{
    encrypt::{get_default_key, uncry},
    utils::is_root,
};

// https://github.com/tiann/KernelSU/blob/main/userspace/ksud/src/ksu.rs#L75
#[cfg(unix)]
pub fn root_shell() -> anyhow::Result<()> {
    get_root()?;
    // we are root now, this was set in kernel!

    let args: Vec<String> = std::env::args().collect();
    trace!("{{ {} }}", &args.join(","));
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt(
        "c",
        "command",
        "pass COMMAND to the invoked shell",
        "COMMAND",
    );
    opts.optflag("h", "help", "display this help message and exit");
    opts.optflag("l", "login", "pretend the shell to be a login shell");
    opts.optflag(
        "p",
        "preserve-environment",
        "preserve the entire environment",
    );
    opts.optflag(
        "s",
        "shell",
        "use SHELL instead of the default /system/bin/sh",
    );
    opts.optflag("v", "version", "display version number and exit");
    opts.optflag("V", "", "display version code and exit");
    opts.optflag(
        "M",
        "mount-master",
        "force run in the global mount namespace",
    );

    // Replace -cn with -z, -mm with -M for supporting getopt_long
    let args = args
        .into_iter()
        .map(|e| {
            if e == "-mm" {
                "-M".to_string()
            } else if e == "-cn" {
                "-z".to_string()
            } else {
                e
            }
        })
        .collect::<Vec<String>>();

    let matches = match opts.parse(&args[1..]) {
        std::result::Result::Ok(m) => m,
        Err(f) => {
            println!("{f}");
            print_usage(&program, opts);
            std::process::exit(-1);
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }

    if matches.opt_present("v") {
        println!("SKRoot: 1.0");
        return Ok(());
    }

    if matches.opt_present("V") {
        println!("1");
        return Ok(());
    }

    let shell = matches.opt_str("s").unwrap_or("/system/bin/sh".to_string());
    let mut is_login = matches.opt_present("l");
    let preserve_env = matches.opt_present("p");
    let mount_master = matches.opt_present("M");

    let mut free_idx = 0;
    let command = matches.opt_str("c").map(|cmd| {
        free_idx = matches.free.len();
        let mut cmds = vec![];
        cmds.push(cmd);
        cmds.extend(matches.free.clone());
        cmds
    });

    let mut args = vec![];
    if let Some(cmd) = command {
        args.push("-c".to_string());
        args.push(cmd.join(" "));
    };

    if free_idx < matches.free.len() && matches.free[free_idx] == "-" {
        is_login = true;
        free_idx += 1;
    }

    let mut uid = 0; // default uid = 0(root)
    if free_idx < matches.free.len() {
        let name = &matches.free[free_idx];
        uid = unsafe {
            #[cfg(target_arch = "aarch64")]
            let pw = libc::getpwnam(name.as_ptr() as *const u8).as_ref();
            #[cfg(target_arch = "x86_64")]
            let pw = libc::getpwnam(name.as_ptr() as *const i8).as_ref();

            match pw {
                Some(pw) => pw.pw_uid,
                None => name.parse::<u32>().unwrap_or(0),
            }
        }
    }

    // https://github.com/topjohnwu/Magisk/blob/master/native/src/su/su_daemon.cpp#L408
    let arg0 = if is_login { "-" } else { &shell };

    let mut command = &mut Command::new(&shell);

    if !preserve_env {
        // This is actually incorrect, i don't know why.
        // command = command.env_clear();
        let pw = unsafe { libc::getpwuid(uid).as_ref() };

        if let Some(pw) = pw {
            let home = unsafe { CStr::from_ptr(pw.pw_dir) };
            let pw_name = unsafe { CStr::from_ptr(pw.pw_name) };

            let home = home.to_string_lossy();
            let pw_name = pw_name.to_string_lossy();

            command = command
                .env("HOME", home.as_ref())
                .env("USER", pw_name.as_ref())
                .env("LOGNAME", pw_name.as_ref())
                .env("SHELL", &shell);
        }
    }

    // escape from the current cgroup and become session leader
    // WARNING!!! This cause some root shell hang forever!
    // command = command.process_group(0);
    command = unsafe {
        command.pre_exec(move || {
            utils::umask(0o22);
            utils::switch_cgroups();

            // switch to global mount namespace
            #[cfg(any(target_os = "linux", target_os = "android"))]
            if mount_master {
                let _ = utils::switch_mnt_ns(1);
                let _ = utils::unshare_mnt_ns();
            }

            set_identity(uid);

            std::result::Result::Ok(())
        })
    };

    command = command.args(args).arg0(arg0);

    Err(command.exec().into())
}

fn set_identity(uid: u32) {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        libc::seteuid(uid);
        libc::setresgid(uid, uid, uid);
        libc::setresuid(uid, uid, uid);
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("SKRoot\n\nUsage: {program} [options] [-] [user [argument...]]");
    print!("{}", opts.usage(&brief));
}

pub fn get_root() -> anyhow::Result<()> {
    if is_root() {
        return Ok(());
    }

    let root_key = get_root_key()?;
    get_root_by_root_key(&root_key)
}

pub fn get_root_by_root_key(root_key: &str) -> anyhow::Result<()> {
    if is_root() {
        return Ok(());
    }

    let root_key_c = CString::new(root_key)?;
    unsafe {
        libc::syscall(libc::SYS_execve, root_key_c.as_ptr(), 0, 0);
        if is_root() {
            Ok(())
        } else {
            Err(anyhow!("get root failed"))
        }
    }
}

fn get_root_key() -> anyhow::Result<String> {
    let program_path = std::env::current_exe()?;
    let cur = program_path.parent().ok_or(anyhow!("Key not found"))?;
    let parent_path_name = cur.file_name().unwrap().to_string_lossy();
    if !parent_path_name.starts_with("su_") {
        Err(anyhow!("Key not found in parent dir"))
    } else {
        let key = get_default_key();

        let encryted_key = parent_path_name[3..].to_string();
        let root_key = uncry(&encryted_key, &key);
        Ok(root_key)
    }
}
