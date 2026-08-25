//! config.ini 加密/解密往返测试
use anime_game_core::endfield::crypto::{
    decrypt_bytes_to_string,
    encrypt_string_to_bytes,
    encrypt_string_to_file,
    read_version_from_config,
    is_valid_config
};

#[test]
fn encrypt_decrypt_roundtrip() {
    let content = "[General]\nversion=76.0.0\nchannel=1\n";

    let encrypted = encrypt_string_to_bytes(content).expect("encryption failed");
    let decrypted = decrypt_bytes_to_string(&encrypted).expect("decryption failed");

    assert_eq!(decrypted, content);
    println!("往返测试通过: {decrypted:?}");
}

#[test]
fn config_file_roundtrip() {
    let dir = std::env::temp_dir().join("endfield-config-test");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("config.ini");

    let content = "[General]\nversion=76.0.0\nsub_channel=1\n";

    encrypt_string_to_file(content, &path).expect("write failed");

    // 验证能读取版本
    let version = read_version_from_config(&path).expect("read failed");
    assert_eq!(version.as_deref(), Some("76.0.0"));

    // 验证有效性检查
    assert!(is_valid_config(&path));

    // 验证文件是加密的（不是明文）
    let raw = std::fs::read(&path).unwrap();
    let raw_str = String::from_utf8_lossy(&raw);
    assert!(!raw_str.contains("version="), "config.ini 不应是明文");

    let _ = std::fs::remove_dir_all(&dir);
    println!("config.ini 文件往返测试通过");
}
