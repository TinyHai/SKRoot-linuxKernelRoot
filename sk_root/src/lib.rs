mod deps;
pub mod encrypt;
pub mod root;
mod utils;

pub use deps::*;

pub use utils::is_root;

#[cfg(test)]
mod tests {
    #[test]
    fn test_rand() {
        use rand::random;

        println!("{}", random::<u8>());
    }

    #[test]
    fn test_encry() {
        use base64::engine::general_purpose;
        use base64::Engine;
        use crate::deps::{RANDOM_GUID_LEN, ROOT_KEY_LEN};
        use crate::encrypt::uncry;

        const ENCRYPTED: &str = "e00f49abcb2a6b85e272f67598d13292ee678de31cb4ca21a63590c00b51a7364997db1e5ef40967ef0a529031bc2449f07191fe45a23e47a0324aacc03bb0d129b8156bf061963f5f8d8dc0c52344b5c2";
        let uncrypted = uncry(ENCRYPTED, "40");
        println!("uncrypted: {}", uncrypted);
        let root_key = general_purpose::STANDARD.decode(uncrypted).unwrap();
        if root_key.len() < RANDOM_GUID_LEN + ROOT_KEY_LEN {
        } else {
            let start_idx = root_key.len() - ROOT_KEY_LEN;
            let root_key = String::from_utf8(root_key[start_idx..].to_vec()).unwrap();
            println!("root_key = {}", root_key);
        }
    }
}
