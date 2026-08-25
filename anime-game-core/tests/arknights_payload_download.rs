//! 真实切服文件下载验证（需要网络，默认忽略）
use anime_game_core::arknights::consts::GameEdition;
use anime_game_core::arknights::payload;

#[test]
#[ignore = "requires network access"]
fn download_official_payload() {
    let dest = std::env::temp_dir().join("arknights-payload-test");
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(&dest).unwrap();

    let files = payload::download_payload(GameEdition::Official, &dest, |done, total| {
        println!("下载进度: {done}/{total}");
    }).expect("payload download failed");

    println!("下载了 {} 个切服文件:", files.len());
    for f in &files {
        println!("  {}", f.display());
    }

    assert!(!files.is_empty(), "no payload files downloaded");

    // 验证关键文件存在
    assert!(dest.join("hgsdk.dll").exists(), "hgsdk.dll missing");
    assert!(dest.join("U8CoreUI.dll").exists(), "U8CoreUI.dll missing");
    assert!(dest.join("U8Data/config/config.bin").exists(), "config.bin missing");

    let _ = std::fs::remove_dir_all(&dest);
}
