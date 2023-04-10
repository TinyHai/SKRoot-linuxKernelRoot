use std::{
    fs::{self, set_permissions, DirBuilder, OpenOptions, Permissions},
    os::unix::prelude::PermissionsExt,
    path::PathBuf,
    time::Duration,
};

#[cfg(target_os = "android")]
use android_logger::Config;
use anyhow::{Ok, Result, anyhow};
use base64::{engine::general_purpose, Engine};
use clap::{Parser, Subcommand};
use log::trace;
use log::LevelFilter;
use sk_root::{
    encrypt::{self, encry, get_default_key},
    root::get_root_by_root_key, is_root,
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
        root_key: Option<String>,
        /// Target app package
        cmd: String,
        /// The su path
        su_path: String,
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
        }
        Commands::Deploy {
            root_key,
            su_path,
            target_path,
        } => {
            get_root_by_root_key(&root_key)?;
            deploy_su(&root_key, &su_path, &target_path)?;
        }
        Commands::Inject { root_key, cmd, su_path } => {
            if let Some(root_key) = root_key {
                get_root_by_root_key(&root_key)?;
            }
            if !is_root() {
                return Err(anyhow!("Please pass root key for getting root permission"));
            }
            let pid = find_pid_by_cmd(&cmd, Duration::from_secs(30))?;
            inject_path_to_pid(pid, &su_path)?;
            trace!("inject pid {} success", pid);
        }
    };
    Ok(())
}

fn deploy_su(root_key: &str, su_path: &str, target: &str) -> Result<()> {
    #[inline(always)]
    fn perm_all() -> Permissions {
        Permissions::from_mode(0o777)
    }
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
                    set_permissions(entry.path(), perm_all())?;
                    set_permissions(su, perm_all())?;
                    println!("{}", entry.path().to_string_lossy());
                    return Ok(());
                }
            } else {
                fs::remove_dir_all(entry.path())?;
            }
        }
    }

    let base64_root_key = general_purpose::STANDARD.encode(root_key);
    let encrypted_base64 = encry(&base64_root_key, &get_default_key());
    let target_su_parent_path_name = format!("su_{}", encrypted_base64);
    let target_su_parent_dir = &PathBuf::from(target).join(target_su_parent_path_name);
    let target_su_path = &target_su_parent_dir.join("su");
    let _ = DirBuilder::new().create(&target_su_parent_dir)?;
    let _ = OpenOptions::new().create(true).open(&target_su_path)?;
    set_permissions(target_su_parent_dir, perm_all())?;
    set_permissions(target_su_path, perm_all())?;

    fs::copy(su_path, target_su_path)?;
    println!("{}", target_su_parent_dir.to_string_lossy());
    Ok(())
}
