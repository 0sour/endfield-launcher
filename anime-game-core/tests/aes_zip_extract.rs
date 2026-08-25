//! AES 加密 zip 解压测试（验证 7z + 密码链路）
use anime_game_core::installer::archives::Archive;

#[test]
fn extract_encrypted_zip() {
    let dir = std::path::PathBuf::from("/home/sour/文档/YJ-Lunch/aes-zip-test");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    // 用 7z 创建 AES 加密 zip
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("file1.txt"), "test content 123").unwrap();
    std::fs::write(src.join("file2.txt"), "another file").unwrap();

    let zip_path = dir.join("encrypted.zip");
    let password = "Slw3HQEZNqM83Q9HYKfQpp2mPLB4rA7p";

    eprintln!("zip_path: {}", zip_path.display());
    eprintln!("src files: {} {}", src.join("file1.txt").display(), src.join("file2.txt").display());

    let output = std::process::Command::new("7z")
        .arg("a")
        .arg("-y")
        .arg(format!("-p{password}"))
        .arg("-mem=AES256")
        .arg("-v1k")
        .arg(&zip_path)
        .arg(src.join("file1.txt"))
        .arg(src.join("file2.txt"))
        .output()
        .expect("7z failed");
    assert!(output.status.success(), "7z creation failed: stdout={} stderr={}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));

    // 无密码打开应失败（或列出为空）
    // 有密码打开应成功
    // 7z 创建分卷后生成 .001 文件
    let volume_path = dir.join("encrypted.zip.001");
    assert!(volume_path.exists(), "volume file not created");

    let mut archive = Archive::open_with_password(&volume_path, Some(password))
        .expect("open with password failed");

    let entries = archive.get_entries().expect("get_entries failed");
    assert_eq!(entries.len(), 2, "expected 2 entries");

    // 解压
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    archive.extract(&out).expect("extract failed");

    // 验证内容
    let content = std::fs::read_to_string(out.join("file1.txt")).unwrap();
    assert_eq!(content, "test content 123");

    println!("AES 加密 zip 解压测试通过: {} 个文件", entries.len());

    let _ = std::fs::remove_dir_all(&dir);
}
