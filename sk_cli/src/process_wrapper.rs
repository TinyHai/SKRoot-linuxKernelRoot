use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    process,
};

use libc::pid_t;

pub struct ProcessWrapper {
    target: libc::pid_t,
}

impl ProcessWrapper {
    pub fn new(pid: libc::pid_t) -> Self {
        Self { target: pid }
    }

    pub fn myself() -> Self {
        Self {
            target: process::id() as i32,
        }
    }

    pub fn target(&self) -> pid_t {
        self.target
    }

    pub fn get_so_base(&self, so_path: &str) -> Option<usize> {
        let pid = self.target;
        let maps_filename = format!("/proc/{}/maps", pid);
        let maps_path = Path::new(&maps_filename);
        let maps_file = File::options().read(true).open(maps_path).ok()?;
        let maps_lines = BufReader::new(maps_file).lines();
        for line in maps_lines {
            let line = line.ok()?;
            if line.contains(so_path) {
                let end = line.find('-')?;
                let mut base = usize::from_str_radix(&line[..end], 16).ok()?;
                if base == 0x8000usize {
                    base = 0;
                }
                return Some(base);
            }
        }
        None
    }

    pub fn get_so_path(&self, name: &str) -> Option<String> {
        let pid = self.target;
        let lib_name = if name.ends_with(".so") {
            name.to_string()
        } else {
            format!("{}.so", name)
        };
        let maps_filename = format!("/proc/{}/maps", pid);
        let maps_path = Path::new(&maps_filename);
        let maps_file = File::options().read(true).open(maps_path).ok()?;
        let maps_lines = BufReader::new(maps_file).lines();
        for line in maps_lines {
            let line = line.ok()?;
            if line.contains(&lib_name) {
                let start = line.find('/')?;
                return Some(line[start..].to_string());
            }
        }
        None
    }
}
