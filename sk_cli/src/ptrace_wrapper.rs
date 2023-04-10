#![cfg(target_arch = "aarch64")]

use std::{
    cell::RefCell,
    cmp::min,
    mem::{size_of, size_of_val, MaybeUninit},
    ops::{Deref, DerefMut},
};

use anyhow::{anyhow, Ok};
use log::trace;
use libc::{
    c_void, pid_t, ptrace, waitpid, PTRACE_ATTACH, PTRACE_CONT, PTRACE_DETACH,
    PTRACE_GETREGSET, PTRACE_PEEKTEXT, PTRACE_POKETEXT, PTRACE_SETREGSET, PTRACE_SYSCALL,
    WUNTRACED,
};

pub struct PtraceWrapper {
    target: libc::pid_t,
    regs: RefCell<libc::user_regs_struct>,
    bak_regs: Option<libc::user_regs_struct>,
}

impl PtraceWrapper {
    pub fn attach(pid: libc::pid_t) -> anyhow::Result<PtraceWrapper> {
        unsafe {
            let ret = libc::ptrace(PTRACE_ATTACH, pid, 0, 0);
            if ret < 0 {
                return Err(anyhow!("attach pid: {} failed ret = {}", pid, ret));
            }
            let mut status = 0;
            libc::waitpid(pid, &mut status, WUNTRACED);
            trace!("attach status: {}", status);
        }

        let regs = unsafe { MaybeUninit::zeroed().assume_init() };
        Ok(Self {
            target: pid,
            regs: RefCell::new(regs),
            bak_regs: None,
        })
    }

    pub fn continue_run(&self) -> anyhow::Result<()> {
        unsafe {
            let ret = ptrace(PTRACE_CONT, self.target, 0, 0);
            if ret < 0 {
                Err(anyhow!("continue run failed ret: {}", ret))
            } else {
                Ok(())
            }
        }
    }

    pub fn call(
        &self,
        fun_name: &str,
        fun_addr: *mut c_void,
        parameters: &[u64],
    ) -> anyhow::Result<u64> {
        trace!("call {}", fun_name);
        let mut regs = self.regs.borrow_mut();
        self.get_regs(regs.deref_mut())?;
        self.call_internal(fun_addr, parameters, regs.deref_mut())?;
        self.get_regs(regs.deref_mut())?;
        trace!(
            "target: {} returned from {}, return value = {{0x{:X}, {}}}, pc = 0x{:x?}",
            self.target,
            fun_name,
            self.get_retval(regs.deref()),
            self.get_retval(regs.deref()),
            self.get_pc(regs.deref())
        );
        Ok(self.get_retval(regs.deref()))
    }

    #[inline(always)]
    fn get_retval(&self, regs: &libc::user_regs_struct) -> u64 {
        regs.regs[0]
    }

    #[inline(always)]
    fn get_pc(&self, regs: &libc::user_regs_struct) -> u64 {
        regs.pc
    }

    fn call_internal(
        &self,
        fun_addr: *mut c_void,
        parameters: &[u64],
        regs: &mut libc::user_regs_struct,
    ) -> anyhow::Result<()> {
        trace!(
            "regs {{ r0: 0x{:X}, r30: 0x{:X}, pc: 0x{:X}, sp: 0x{:X} }}",
            regs.regs[0], regs.regs[30], regs.pc, regs.sp
        );

        let num_parms_regs = 8;
        let parms_len = parameters.len();
        let parms_by_regs = min(num_parms_regs, parms_len);
        for i in 0..parms_by_regs {
            regs.regs[i] = parameters[i];
        }

        if parms_len > num_parms_regs {
            regs.sp -= ((parms_len - num_parms_regs) * size_of::<u64>()) as u64;
            let data = &parameters[parms_by_regs..];
            self.write_data(
                regs.sp as *const u8,
                data.as_ptr() as *const u8,
                data.len() * 8,
            )?;
        }

        regs.pc = fun_addr as u64;
        // arm64 unsupport thumb mode
        // if regs.pc & 1 > 0 {
        //     regs.pc &= u16::MAX as u32;
        //     regs.pstate |= 1u64 << 5;
        // } else {
        //     regs.pstate &= !(1u64 << 5)
        // }

        regs.regs[30] = 0;

        self.set_regs(regs)?;
        self.continue_run()?;

        #[inline(always)]
        fn print_wait_status(target: pid_t, status: i32, res: i32) {
            let is_stopped = libc::WIFSTOPPED(status);
            let stop_sig = if is_stopped {
                libc::WSTOPSIG(status)
            } else {
                0
            };
            trace!(
                "waitpid: {} status: {{ STOPPED: {}, REASON: {} }}, res: {}",
                target,
                is_stopped,
                stop_sig,
                res
            );
        }

        let mut status = 0;
        let res = unsafe { waitpid(self.target, &mut status, WUNTRACED) };
        print_wait_status(self.target, status, res);

        let ret = unsafe { ptrace(PTRACE_SYSCALL, self.target, 0, 0) };
        if ret < 0 {
            return Err(anyhow!("ptrace systemcall failed, ret: {}", ret));
        }

        let res = unsafe { waitpid(self.target, &mut status, WUNTRACED) };
        print_wait_status(self.target, status, res);

        let ret = unsafe { ptrace(PTRACE_SYSCALL, self.target, 0, 0) };
        if ret < 0 {
            return Err(anyhow!("ptrace systemcall failed, ret: {}", ret));
        }

        let res = unsafe { waitpid(self.target, &mut status, WUNTRACED) };
        print_wait_status(self.target, status, res);

        Ok(())
    }

    pub fn read_data(&self, src: *const u8, buf: *mut u8, size: usize) -> anyhow::Result<()> {
        union U {
            val: libc::c_long,
            chars: [libc::c_char; size_of::<libc::c_long>()],
        }
        let bytes_width = size_of::<libc::c_long>();
        let mut u = U { val: 0 };
        let cnt = size / bytes_width;
        for i in 0..cnt {
            unsafe {
                let src = src.add(i * bytes_width);
                u.val = ptrace(PTRACE_PEEKTEXT, self.target, src, 0);
                let buf = buf.add(i * bytes_width);
                *(buf as *mut libc::c_long) = u.val;
            }
        }

        let remain = size % bytes_width;
        let read = size - remain;
        if remain > 0 {
            unsafe {
                let buf = buf.add(read);
                let src = src.add(read);
                u.val = ptrace(PTRACE_PEEKTEXT, self.target, src, 0);
                for i in 0..remain {
                    *(buf.add(i) as *mut _) = u.chars[i];
                }
            }
        }

        Ok(())
    }

    pub fn write_data(&self, dest: *const u8, data: *const u8, size: usize) -> anyhow::Result<()> {
        union U {
            val: libc::c_long,
            chars: [libc::c_char; size_of::<libc::c_long>()],
        }
        let bytes_width = size_of::<libc::c_long>();
        let mut u = U { val: 0 };
        let cnt = size / bytes_width;
        for i in 0..cnt {
            unsafe {
                let dest = dest.add(i * bytes_width);
                let slice = std::slice::from_raw_parts(data.add(i * bytes_width), bytes_width);
                u.val = libc::c_long::from_ne_bytes(slice.to_vec().try_into().unwrap());
                libc::ptrace(PTRACE_POKETEXT, self.target, dest, u.val);
            }
        }

        let remain = size % bytes_width;
        let written = size - remain;
        if remain > 0 {
            let dest = unsafe { dest.add(written) };
            unsafe {
                u.val = ptrace(PTRACE_PEEKTEXT, self.target, dest, 0);
                for i in 0..remain {
                    u.chars[i] = *data.add(written + i);
                }
                ptrace(PTRACE_POKETEXT, self.target, dest, u.val);
            }
        }

        Ok(())
    }

    pub fn detach(&mut self) -> bool {
        if self.target != 0 {
            self.detach_internal();
            true
        } else {
            false
        }
    }

    fn detach_internal(&mut self) {
        unsafe {
            let ret = libc::ptrace(PTRACE_DETACH, self.target, 0, 0);
            if ret < 0 {
                trace!("detach pid: {}, ret: {}", self.target, ret);
            }
        }
        self.target = 0;
    }

    fn get_regs(&self, regs: &mut libc::user_regs_struct) -> anyhow::Result<()> {
        let io_vec = libc::iovec {
            iov_base: regs as *mut _ as *mut _,
            iov_len: size_of_val(regs),
        };
        unsafe {
            let ret = libc::ptrace(
                PTRACE_GETREGSET,
                self.target,
                1, /* NT_PRSTATUS */
                &io_vec,
            );
            if ret < 0 {
                Err(anyhow!("getregs falied ret: {}", ret))
            } else {
                Ok(())
            }
        }
    }

    fn set_regs(&self, regs: &mut libc::user_regs_struct) -> anyhow::Result<()> {
        let io_vec = libc::iovec {
            iov_base: regs as *mut _ as *mut _,
            iov_len: size_of_val(regs),
        };
        unsafe {
            let ret = libc::ptrace(
                PTRACE_SETREGSET,
                self.target,
                1, /* NT_PRSTATUS */
                &io_vec,
            );
            if ret < 0 {
                Err(anyhow!("setregs failed ret: {}", ret))
            } else {
                Ok(())
            }
        }
    }

    pub fn backup_regs(&mut self) -> anyhow::Result<()> {
        let mut regs = unsafe { MaybeUninit::<libc::user_regs_struct>::zeroed().assume_init() };
        self.get_regs(&mut regs)?;
        self.bak_regs = Some(regs);
        Ok(())
    }

    pub fn restore_regs(&mut self) -> anyhow::Result<()> {
        let mut regs = self.bak_regs.take();
        if let Some(ref mut regs) = regs {
            self.set_regs(regs)?;
        }
        Ok(())
    }
}

impl Drop for PtraceWrapper {
    fn drop(&mut self) {
        self.restore_regs().unwrap();
        self.detach();
    }
}
