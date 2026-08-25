//! hdiff 应用链路测试：构造 base + diff，验证 hpatchz::patch 应用
use std::process::Command;

#[test]
fn hdiff_apply_roundtrip() {
    let dir = std::env::temp_dir().join("hdiff-test");
    let _ = std::fs::create_dir_all(&dir);

    // 1. 构造 base 文件（模拟 75.0.0 的游戏文件）
    let base_path = dir.join("base.ab");
    let base_content: Vec<u8> = (0..4096).map(|i| (i % 251) as u8).collect();
    std::fs::write(&base_path, &base_content).unwrap();

    // 2. 构造 new 文件（模拟 76.0.0 的文件，部分内容变化）
    let new_path = dir.join("new.ab");
    let mut new_content = base_content.clone();
    new_content[100] = 0xAB;
    new_content[2000] = 0xCD;
    new_content.extend_from_slice(b"new data block");
    std::fs::write(&new_path, &new_content).unwrap();

    // 3. 用 hdiffz 生成 diff（HDiffPatch 的 diff 创建工具）
    let diff_path = dir.join("test.hdiff");
    let hdiffz = "/home/sour/文档/YJ-Lunch/hdiffpatch_tools/linux64/hdiffz";
    let status = Command::new(hdiffz)
        .arg(&base_path)
        .arg(&new_path)
        .arg(&diff_path)
        .status()
        .expect("hdiffz failed");
    assert!(status.success(), "hdiffz diff creation failed");

    // 4. 用 anime_game_core 的 hpatchz::patch 应用 diff
    let output_path = dir.join("output.ab");
    anime_game_core::external::hpatchz::patch(&base_path, &diff_path, &output_path)
        .expect("patch application failed");

    // 5. 验证输出 == new
    let output = std::fs::read(&output_path).unwrap();
    assert_eq!(output, new_content, "patched output doesn't match expected");

    println!("hdiff 应用链路测试通过: base {}B -> new {}B", base_content.len(), new_content.len());

    let _ = std::fs::remove_dir_all(&dir);
}
