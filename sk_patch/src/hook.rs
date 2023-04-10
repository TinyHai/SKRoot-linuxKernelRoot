#[derive(Debug)]
pub struct DoExecveHook {
    pub root_key_size: usize,
    pub hooker_entry: usize,
    pub hookee_entry: usize,
    pub cred_offset: usize,
    pub seccomp_offset: usize,
}

#[derive(Debug)]
pub struct AVCDeniedHook {
    pub hooker_entry: usize,
    pub hookee_entry: usize,
    pub cred_offset: usize,
}

pub trait Hook {
    fn hookee_entry(&self) -> usize;
    fn hooker_entry(&self) -> usize;
}

macro_rules! impl_hook_for {
    ($($type:ty),*) => {
        $(
            impl Hook for $type {
                fn hookee_entry(&self) -> usize {
                    self.hookee_entry
                }

                fn hooker_entry(&self) -> usize {
                    self.hooker_entry
                }
            }
        )*
    };
}

impl_hook_for!(DoExecveHook, AVCDeniedHook);