use std::{
    fs::{self, set_permissions, DirBuilder, Permissions},
    os::unix::{prelude::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(target_os = "android")]
use android_logger::Config;
use anyhow::{anyhow, Ok, Result};

use clap::{Parser, Subcommand};
use log::trace;
use log::LevelFilter;
use sk_root::{
    encrypt::{self, encry, get_default_key},
    is_root,
    root::get_root_by_root_key,
};
use time::OffsetDateTime;

use crate::inject::{find_pid_by_cmd, inject_path_to_pid};

#[derive(Parser, Debug)]
#[command(author, version = "1.0", about, long_about = None)]
struct Args {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Encry or uncry root key
    Key {
        /// Pass root key to encry
        #[arg(short, long)]
        encry: Option<String>,
        /// Pass root key to uncry
        #[arg(short, long)]
        uncry: Option<String>,
    },
    /// Get a root shell
    Su {
        /// The clear root key
        root_key: String,
    },
    /// Deploy su to special path
    Deploy {
        /// The clear root key
        root_key: String,
        /// The su path
        su_path: String,
        /// The path to deploy su
        target_path: String,
    },
    /// inject su to target PATH
    Inject {
        /// The clear root key
        #[arg(short, long)]
        root_key: Option<String>,
        /// Target app package
        #[arg(short, long)]
        cmd: String,
        /// The su path
        #[arg(short, long)]
        su_path: String,
        /// The timeout seconds
        #[arg(short, long, default_value_t = 10)]
        timeout: u32,
    },
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        Config::default()
            .with_max_level(LevelFilter::Trace) // limit log level
            .with_tag("SK_CLI"), // logs will show under mytag tag
    );

    let args = Args::parse();
    match args.commands {
        Commands::Key { encry, uncry } => {
            if let Some(encry) = encry {
                println!("{}", encrypt::encry(&encry, &get_default_key()));
            }
            if let Some(uncry) = uncry {
                println!("{}", encrypt::uncry(&uncry, &get_default_key()));
            }
            Ok(())
        }
        Commands::Su { root_key } => {
            get_root_by_root_key(&root_key)?;
            Err(std::process::Command::new("sh").exec().into())
        }
        Commands::Deploy {
            root_key,
            su_path,
            target_path,
        } => {
            get_root_by_root_key(&root_key)?;
            deploy_su(&root_key, &su_path, &target_path)
        }
        Commands::Inject {
            root_key,
            cmd,
            su_path,
            timeout,
        } => {
            trace!(
                "inject root_key {:?}, cmd: {}, su_path: {}, timeout:{}",
                root_key.as_ref(),
                cmd,
                su_path,
                timeout
            );
            if let Some(root_key) = root_key {
                get_root_by_root_key(&root_key)?;
            }
            if !is_root() {
                return Err(anyhow!("Please pass root key for getting root permission"));
            }
            let pid = find_pid_by_cmd(&cmd, Duration::from_secs(timeout.into()))?;
            inject_path_to_pid(pid, &su_path)?;
            trace!("inject pid {} success", pid);
            Ok(())
        }
    }
}

fn deploy_su(root_key: &str, su_path: &str, target: &str) -> Result<()> {
    let today = OffsetDateTime::now_local().unwrap_or(OffsetDateTime::now_utc());
    let target_dir = fs::read_dir(target)?;
    for entry in target_dir {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with("su_") {
            let modified_date: OffsetDateTime = entry.metadata()?.modified()?.into();
            if today.day() == modified_date.day() {
                let su = entry.path().join("su");
                if su.exists() {
                    ensure_accessable(entry.path())?;
                    ensure_accessable(su)?;
                    println!("{}", entry.path().to_string_lossy());
                    return Ok(());
                }
            } else {
                fs::remove_dir_all(entry.path())?;
            }
        }
    }

    let encrypted_root_key = encry(&root_key, &get_default_key());
    let target_su_parent_path_name = format!("su_{}", encrypted_root_key);
    let target_su_parent_dir = &PathBuf::from(target).join(target_su_parent_path_name);
    let target_su_path = &target_su_parent_dir.join("su");

    trace!(
        "deploy create dir {}",
        target_su_parent_dir.to_string_lossy()
    );
    let _ = DirBuilder::new().create(&target_su_parent_dir)?;
    trace!(
        "deploy copy {} -> {}",
        su_path,
        target_su_path.to_string_lossy()
    );
    fs::copy(su_path, target_su_path)?;

    ensure_accessable(target_su_parent_dir)?;
    ensure_accessable(target_su_path)?;

    println!("{}", target_su_parent_dir.to_string_lossy());
    Ok(())
}

const SELINUX_XATTR: &str = "security.selinux";
const SYSTEM_FILE: &str = "u:object_r:system_file:s0";

fn ensure_accessable<P>(path: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    trace!("setxattr for {}", path.as_ref().to_string_lossy());
    #[inline(always)]
    fn perm_all() -> Permissions {
        Permissions::from_mode(0o777)
    }
    set_permissions(path.as_ref(), perm_all())?;
    xattr::set(path, SELINUX_XATTR, SYSTEM_FILE.as_bytes())?;
    Ok(())
}
