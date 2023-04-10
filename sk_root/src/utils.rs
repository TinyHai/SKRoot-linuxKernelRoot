use anyhow::{Ok, Result};

use std::{
    fs::{OpenOptions},
    io::Write,
    path::{Path},
};

// https://github.com/tiann/KernelSU/blob/main/userspace/ksud/src/utils.rs

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn switch_mnt_ns(pid: i32) -> Result<()> {
    use anyhow::ensure;
    use std::os::fd::AsRawFd;
    let path = format!("/proc/{pid}/ns/mnt");
    let fd = std::fs::File::open(path)?;
    let current_dir = std::env::current_dir();
    let ret = unsafe { libc::setns(fd.as_raw_fd(), libc::CLONE_NEWNS) };
    if let std::result::Result::Ok(current_dir) = current_dir {
        let _ = std::env::set_current_dir(current_dir);
    }
    ensure!(ret == 0, "switch mnt ns failed");
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn unshare_mnt_ns() -> Result<()> {
    use anyhow::ensure;
    let ret = unsafe { libc::unshare(libc::CLONE_NEWNS) };
    ensure!(ret == 0, "unshare mnt ns failed");
    Ok(())
}

fn switch_cgroup(grp: &str, pid: u32) {
    let path = Path::new(grp).join("cgroup.procs");
    if !path.exists() {
        return;
    }

    let fp = OpenOptions::new().append(true).open(path);
    if let std::result::Result::Ok(mut fp) = fp {
        let _ = writeln!(fp, "{pid}");
    }
}

pub fn switch_cgroups() {
    let pid = std::process::id();
    switch_cgroup("/acct", pid);
    switch_cgroup("/dev/cg2_bpf", pid);
    switch_cgroup("/sys/fs/cgroup", pid);

    if getprop("ro.config.per_app_memcg")
        .filter(|prop| prop == "false")
        .is_none()
    {
        switch_cgroup("/dev/memcg/apps", pid);
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn umask(mask: u32) {
    unsafe { libc::umask(mask) };
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn getprop(prop: &str) -> Option<String> {
    android_properties::getprop(prop).value()
}

pub fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}
