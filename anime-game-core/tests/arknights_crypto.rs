//! 端到端验证：用逆向的 AES 密钥解密真实的 game_files 清单
use anime_game_core::arknights::crypto::decrypt_bytes_to_string;

#[test]
fn decrypt_real_game_files() {
    let encrypted = std::fs::read("tests/data/game_files.bin").expect("game_files.bin missing");

    let content = decrypt_bytes_to_string(&encrypted).expect("decryption failed");

    // 验证是 JSONL 格式且包含预期字段
    let first_line = content.lines().next().expect("empty content");
    assert!(first_line.contains("\"path\""), "first line: {first_line}");
    assert!(first_line.contains("\"md5\""));
    assert!(first_line.contains("\"size\""));

    // 验证行数（真实清单约 13315 行）
    let lines = content.lines().count();
    assert!(lines > 10000, "expected >10000 lines, got {lines}");

    println!("解密成功: {} 行, 首行: {}", lines, &first_line[..80]);
}
