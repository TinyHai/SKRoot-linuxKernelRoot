use std::{
    ffi::{CString},
    fs::{self, File},
    io::{self, BufRead, Write},
    path::PathBuf,
    ptr::null_mut,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::process_wrapper::ProcessWrapper;
#[cfg(target_arch = "aarch64")]
use crate::ptrace_wrapper::PtraceWrapper;

use anyhow::anyhow;
use libc::{
    c_void, pid_t, MAP_ANONYMOUS, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE, RTLD_GLOBAL,
    RTLD_NOW,
};
use paste::paste;
use log::trace;

struct InjectHelper {
    mmap_addr: *mut libc::c_void,
    munmap_addr: *mut libc::c_void,
    getenv_addr: *mut libc::c_void,
    setenv_addr: *mut libc::c_void,
}

macro_rules! pub_call_it {
    ($fun_name:ident) => {
        paste! {
            pub fn [< call_ $fun_name>] (
                &self,
                ptrace_wrapper: &PtraceWrapper,
                parameters: &[u64],
            ) -> anyhow::Result<u64> {
                ptrace_wrapper.call(stringify!($fun_name), self.[< $fun_name _addr >], parameters)
            }
        }
    };
    ($($fun_name:ident),+) => {
        paste! {
            $(
                pub fn [< call_ $fun_name>] (
                    &self,
                    ptrace_wrapper: &PtraceWrapper,
                    parameters: &[u64],
                ) -> anyhow::Result<u64> {
                    ptrace_wrapper.call(stringify!($fun_name), self.[< $fun_name _addr >], parameters)
                }
            )+
        }
    }
}

macro_rules! cstr {
    ($ident:ident) => {
        CString::new(stringify!($ident)).unwrap()
    };
}

impl InjectHelper {
    fn new(pid: pid_t) -> anyhow::Result<Self> {
        let target_process = ProcessWrapper::new(pid);
        let self_process = ProcessWrapper::myself();
        let mut mmap_offset = 0usize;
        let mut munmap_offset = 0usize;
        let mut getenv_offset = 0usize;
        let mut setenv_offset = 0usize;
        let libc_path = self_process
            .get_so_path("libc")
            .ok_or(anyhow!("libc path not found in self proc"))?;
        InjectHelper::with_dlopen(&self_process, &libc_path, |handle, base| unsafe {
            let mmap_str = cstr!(mmap);
            let munmap_str = cstr!(munmap);
            let getenv_str = cstr!(getenv);
            let setenv_str = cstr!(setenv);
            
            mmap_offset = libc::dlsym(handle, mmap_str.as_ptr()) as usize - base;
            munmap_offset = libc::dlsym(handle, munmap_str.as_ptr()) as usize - base;
            getenv_offset = libc::dlsym(handle, getenv_str.as_ptr()) as usize - base;
            setenv_offset = libc::dlsym(handle, setenv_str.as_ptr()) as usize - base;
        })?;
        let target_base = target_process.get_so_base(&libc_path).ok_or(anyhow!(
            "get base from remote {} failed",
            target_process.target()
        ))?;
        trace!("target_base: 0x{:X}", target_base);
        trace!("mmap_offset: 0x{:X}", mmap_offset);
        trace!("munmap_offset: 0x{:X}", munmap_offset);
        trace!("getenv_offset: 0x{:X}", getenv_offset);
        trace!("setenv_offset: 0x{:X}", setenv_offset);
        let mmap_addr = (target_base + mmap_offset) as *mut c_void;
        let munmap_addr = (target_base + munmap_offset) as *mut c_void;
        let getenv_addr = (target_base + getenv_offset) as *mut c_void;
        let setenv_addr = (target_base + setenv_offset) as *mut c_void;

        Ok(Self {
            mmap_addr,
            munmap_addr,
            getenv_addr,
            setenv_addr,
        })
    }

    #[cfg(target_arch = "aarch64")]
    pub_call_it!(mmap, munmap, getenv, setenv);

    fn with_dlopen<F>(target: &ProcessWrapper, so_path: &str, block: F) -> anyhow::Result<()>
    where
        F: FnOnce(*mut libc::c_void, usize),
    {
        let base = target.get_so_base(so_path).ok_or(anyhow!(
            "get base from {} failed, pid: {}",
            so_path,
            target.target()
        ))?;
        trace!("self_base: 0x{:X}", base);
        unsafe {
            let handle = libc::dlopen(so_path as *const str as *const _, RTLD_NOW | RTLD_GLOBAL);
            trace!("dlopen {}, handle: 0x{:X}", so_path, handle as usize);
            block(handle, base);
            libc::dlclose(handle);
        }
        Ok(())
    }
}

#[cfg(target_arch = "aarch64")]
pub fn inject_path_to_pid(pid: pid_t, path_to_add: &str) -> anyhow::Result<()> {
    const PATH: &[u8] = b"PATH\0";

    let mut parameters = [0u64; 8];
    let mut ptrace_target = PtraceWrapper::attach(pid)?;
    let inject_helper = InjectHelper::new(pid)?;

    ptrace_target.backup_regs()?;

    let mmap_size = 0x4000;
    let mmap_params = prepare_mmap_params(
        &mut parameters,
        null_mut(),
        mmap_size,
        PROT_READ | PROT_WRITE | PROT_EXEC,
        MAP_ANONYMOUS | MAP_PRIVATE,
    );
    let mmap_base = inject_helper.call_mmap(&ptrace_target, mmap_params)? as *mut c_void;

    ptrace_target.write_data(
        mmap_base as *const _,
        PATH as *const _ as *const _,
        PATH.len(),
    )?;

    let getenv_params = prepare_getenv_params(&mut parameters, mmap_base as *const _);
    let origin_env_addr = inject_helper.call_getenv(&ptrace_target, getenv_params)? as *const u8;

    let mut buff = vec![];
    buff.write_all(path_to_add.as_bytes())?;
    buff.push(':' as u8);
    let mut tmp = 0u8;
    let mut count = 0;
    loop {
        ptrace_target.read_data(unsafe { origin_env_addr.add(count) }, &mut tmp, 1)?;
        count += 1;
        if tmp == 0 {
            break;
        }
        buff.push(tmp);
    }
    let complete_env = String::from_utf8(buff)?;
    trace!("will setenv: {}", &complete_env);
    let c_str = CString::new(complete_env).unwrap();
    let bytes_to_write = c_str.as_bytes_with_nul();
    ptrace_target.write_data(
        unsafe { mmap_base.add(PATH.len()) as _ },
        bytes_to_write.as_ptr(),
        bytes_to_write.len(),
    )?;

    let setenv_params = prepare_setenv_params(
        &mut parameters,
        mmap_base as _,
        unsafe { mmap_base.add(PATH.len()) as _ },
        1,
    );
    let result = inject_helper.call_setenv(&ptrace_target, setenv_params)?;
    if result != 0 {
        return Err(anyhow!("setenv failed, code: {}", result));
    }

    let getenv_params = prepare_getenv_params(&mut parameters, mmap_base as *const _);
    let latest_env_addr = inject_helper.call_getenv(&ptrace_target, getenv_params)? as *const u8;
    let mut buff = vec![];
    let mut tmp = 0u8;
    let mut count = 0;
    loop {
        ptrace_target.read_data(unsafe { latest_env_addr.add(count) }, &mut tmp, 1)?;
        count += 1;
        if tmp == 0 {
            break;
        }
        buff.push(tmp);
    }
    let latest_env = String::from_utf8(buff)?;
    trace!("lastest env: {}", latest_env);

    let munmap_params = prepare_munmap_params(&mut parameters, mmap_base, mmap_size);
    inject_helper.call_munmap(&ptrace_target, munmap_params)?;

    ptrace_target.restore_regs()?;
    Ok(())
}

fn prepare_mmap_params<'a>(
    parameters: &'a mut [u64],
    addr: *mut c_void,
    size: usize,
    prot: i32,
    flags: i32,
) -> &'a [u64] {
    parameters[0] = addr as u64;
    parameters[1] = size as u64;
    parameters[2] = prot as u64;
    parameters[3] = flags as u64;
    parameters[4] = 0;
    parameters[5] = 0;
    &parameters[0..=5]
}

fn prepare_munmap_params<'a>(
    parameters: &'a mut [u64],
    addr: *const c_void,
    size: usize,
) -> &'a [u64] {
    parameters[0] = addr as u64;
    parameters[1] = size as u64;
    &parameters[0..=1]
}

fn prepare_getenv_params<'a>(parameters: &'a mut [u64], name: *const u8) -> &'a [u64] {
    parameters[0] = name as u64;
    &parameters[0..=0]
}

fn prepare_setenv_params<'a>(
    parameters: &'a mut [u64],
    name: *const u8,
    value: *const u8,
    rewrite: usize,
) -> &'a [u64] {
    parameters[0] = name as u64;
    parameters[1] = value as u64;
    parameters[2] = rewrite as u64;
    &parameters[0..=2]
}

pub fn find_pid_by_cmd(cmd: &str, timeout: Duration) -> anyhow::Result<pid_t> {
    let clock = Instant::now();
    loop {
        if clock.elapsed() >= timeout {
            break Err(anyhow!("target pid not found due to timeout, cmd: {}", cmd));
        }
        let pid = _find_pid_by_cmd(cmd);
        if pid.is_ok() {
            break pid;
        }

        sleep(Duration::ZERO)
    }
}

fn _find_pid_by_cmd(cmd: &str) -> anyhow::Result<pid_t> {
    let mut target_pid: pid_t = 0;
    let proc_path = PathBuf::from("/proc");
    let proc_entry = fs::read_dir(&proc_path)?;

    let all_proc_entries = proc_entry
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let is_dir = entry.file_type().ok()?.is_dir();
            let is_dot_file = entry.file_name().to_string_lossy().starts_with('.');
            let is_numbers = entry
                .file_name()
                .to_string_lossy()
                .chars()
                .all(char::is_numeric);
            if is_dir && !is_dot_file && is_numbers {
                Some(entry)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for entry in all_proc_entries {
        let cmdline_path = entry.path().join("cmdline");
        let cmdline_file = File::options().read(true).open(cmdline_path);
        match cmdline_file {
            Ok(cmd_file) => {
                let lines = io::BufReader::new(cmd_file).lines();
                for line in lines {
                    let line = line.unwrap_or_default();
                    if line.starts_with(cmd) {
                        target_pid = entry.file_name().to_string_lossy().parse().unwrap_or(0);
                        break;
                    }
                }
            }
            Err(_) => continue,
        }
    }

    if target_pid == 0 {
        Err(anyhow!("target pid not found, cmd: {}", cmd))
    } else {
        Ok(target_pid)
    }
}
