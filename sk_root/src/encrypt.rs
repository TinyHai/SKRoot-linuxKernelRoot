use std::io::Write;

use base64::{engine::general_purpose, Engine};
use rand::random;
use time::OffsetDateTime;

use crate::{deps::RANDOM_GUID_LEN, ROOT_KEY_LEN};

const CHARS_POOL: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn get_random_str(len: usize) -> String {
    let chars = CHARS_POOL.as_bytes();
    let pool_size = chars.len();
    let mut random_vec = Vec::with_capacity(len);
    for _ in 0..len {
        let next_value = chars[random::<usize>() % pool_size];
        random_vec.push(next_value);
    }
    String::from_utf8(random_vec).unwrap_or(String::new())
}

pub fn get_default_key() -> String {
    let time = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let day_of_month = time.day();
    (day_of_month * RANDOM_GUID_LEN as u8).to_string()
}

pub fn encry(src: &str, key: &str) -> String {
    let random_prefix = get_random_str(RANDOM_GUID_LEN);
    let src = random_prefix + src;
    let base64_src = general_purpose::STANDARD.encode(src);
    _encry(base64_src, key)
}

fn _encry(base64_src: String, key: &str) -> String {
    let src_bytes = base64_src.as_bytes();
    let key_bytes = key.as_bytes();
    let key_len = key.len();
    let mut buff = Vec::with_capacity(2);
    let mut offset: u8 = random();
    let mut result = Vec::new();

    write!(&mut buff, "{:02x}", offset).unwrap();
    result.append(&mut buff);

    let mut key_pos = 0usize;
    let mut v: u8;
    for &src_byte in src_bytes {
        v = ((src_byte as u16 + offset as u16) % 255) as u8;
        v ^= key_bytes[key_pos];

        write!(&mut buff, "{:02x}", v).unwrap();
        result.append(&mut buff);

        offset = v;

        if key_pos < key_len - 1 {
            key_pos += 1;
        } else {
            key_pos = 0;
        }
    }
    String::from_utf8(result).unwrap()
}

pub fn uncry(src: &str, key: &str) -> String {
    let uncryed = _uncry(src, key);
    let clear_with_prefix = general_purpose::STANDARD
        .decode(uncryed)
        .expect("uncrped decode base64 failed");
    let start = clear_with_prefix.len() - ROOT_KEY_LEN;
    let root_key = clear_with_prefix[start..].to_vec();
    String::from_utf8(root_key).unwrap()
}

fn _uncry(src: &str, key: &str) -> String {
    let src_bytes = src.as_bytes();
    let key_bytes = key.as_bytes();
    let src_len = src.len();
    let key_len = key.len();
    let mut result = Vec::new();
    let mut offset =
        u8::from_str_radix(std::str::from_utf8(&src_bytes[0..2]).unwrap(), 16).unwrap();

    let mut key_pos = 0;
    let mut i = 2;
    let mut v: u8;
    let mut next_offset: u8;
    while i < src_len {
        v = u8::from_str_radix(std::str::from_utf8(&src_bytes[i..i + 2]).unwrap(), 16).unwrap();
        next_offset = v;
        v ^= key_bytes[key_pos];

        if v <= offset {
            v += 255u8 - offset;
        } else {
            v -= offset;
        }

        result.push(v);

        if key_pos < key_len - 1 {
            key_pos += 1;
        } else {
            key_pos = 0;
        }

        offset = next_offset;
        i += 2;
    }

    String::from_utf8(result).unwrap()
}
