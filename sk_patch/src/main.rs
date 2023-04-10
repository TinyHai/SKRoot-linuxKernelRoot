use std::{env, io::stdin};

mod asm_helper;
mod hook;
mod patcher;

use anyhow::anyhow;
pub use asm_helper::LINE_ENDLING;
use hook::{AVCDeniedHook, DoExecveHook};
use patcher::Patcher;
use rand::random;
use regex::Regex;
use simple_logger::SimpleLogger;

fn main() -> anyhow::Result<()> {
    SimpleLogger::new().env().init().unwrap();

    let mut args = env::args();
    args.next(); // ignore self

    let image_path = args.next().unwrap_or("raw_kernel".to_string());

    let patcher = &mut Patcher::new(&image_path)?;

    let patch_start_offset = wait_input("请输入patch代码起始偏移值:", hex_to_usize);

    let root_key = {
        let need_generate = wait_input("是否需要自动随机生成ROOT密匙?(Y/y)", confirm);
        if need_generate {
            generate_random_root_key()
        } else {
            wait_input(
                "请输入ROOT密匙(48个字符的字符串,包含大小写字母和数字)",
                valid_input_key,
            )
        }
    };
    let mut next_offset = patch_start_offset;
    next_offset = patcher.patch_root_key(&root_key, next_offset);

    let cred_offset = wait_input(
        "请输入task_struct结构体里cred的十六进制偏移值:",
        hex_to_usize,
    );
    let seccomp_offset = wait_input(
        "请输入task_struct结构体里seccomp的十六进制偏移值:",
        hex_to_usize,
    );

    let do_execve_entry = wait_input("请输入do_execve函数的入口位置:", hex_to_usize);
    let do_execve_hook = DoExecveHook {
        root_key_size: root_key.len(),
        hooker_entry: next_offset,
        hookee_entry: do_execve_entry,
        cred_offset,
        seccomp_offset,
    };
    next_offset = patcher.patch_do_execve(do_execve_hook)?;

    let avc_denied_entry = wait_input("请输入avc_denied函数的入口位置:", hex_to_usize);
    let avc_denied_hook = AVCDeniedHook {
        hooker_entry: next_offset,
        hookee_entry: avc_denied_entry,
        cred_offset,
    };
    patcher.patch_avc_denied(avc_denied_hook)?;

    let apply = wait_input("是否立即修补内核文件?(Y/y)", confirm);
    if apply {
        patcher.apply_patches()?;
        println!("已完成修补内核文件")
    } else {
        println!("已放弃修改内核文件")
    }

    Ok(())
}

fn hex_to_usize(hex: &str) -> anyhow::Result<usize> {
    let prefix = "0x";
    let hex = if hex.starts_with(prefix) {
        &hex[2..]
    } else {
        hex
    };
    Ok(usize::from_str_radix(hex, 16)?)
}

fn confirm(s: &str) -> anyhow::Result<bool> {
    Ok(s.eq_ignore_ascii_case("y"))
}

fn valid_input_key(input_key: &str) -> anyhow::Result<String> {
    let input_key = input_key.trim();
    let regex = Regex::new(r"[0-9a-zA-Z]{48}")?;
    if regex.is_match(input_key) {
        Ok(input_key.to_string())
    } else {
        Err(anyhow!("密钥格式不对！"))
    }
}

fn wait_input<P, R>(tips: &str, parser: P) -> R
where
    P: Fn(&str) -> anyhow::Result<R>,
    R: Sized,
{
    loop {
        println!("{}", tips);
        let buf = &mut String::new();
        match stdin().read_line(buf) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("{:?}", e);
                continue;
            }
        };
        match parser(&buf.trim()) {
            Ok(result) => {
                break result;
            }
            Err(e) => {
                eprintln!("{:?}", e);
                continue;
            }
        }
    }
}

fn generate_random_root_key() -> String {
    const ROOT_KEY_LEN: u32 = 48;
    let chars_pool = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let pool_size = chars_pool.len();
    let mut root_key = String::new();
    for _ in 0..ROOT_KEY_LEN {
        let next: usize = random();
        root_key.push(chars_pool[next % pool_size] as char);
    }
    root_key
}

#[cfg(test)]
mod test {

    use regex::Regex;

    use crate::{
        aarch64, asm_helper::asm_to_be_bytes, generate_random_root_key,
        wait_input,
    };

    #[test]
    fn test_asm_to_bytes() {
        use crate::asm_helper::LINE_ENDLING;

        let asm_text = aarch64! {
            "MOV X0, X0";
        };
        let bytes = u32::from_str_radix("E00300AA", 16).unwrap().to_be_bytes();
        assert_eq!(&bytes as &[u8], &asm_to_be_bytes(&asm_text).unwrap());
    }

    #[test]
    fn test_generate_random_root_key() {
        let ramdom_root_key = generate_random_root_key();
        let regex = Regex::new(r"[0-9a-zA-Z]{48}").unwrap();
        assert!(regex.is_match(&ramdom_root_key));
    }

    #[test]
    fn test_wait_input() {
        use crate::hex_to_usize;
        wait_input("hex_test", hex_to_usize);
    }
}
