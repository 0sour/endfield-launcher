//! 切服白名单规则验证
use anime_game_core::arknights::consts::GameEdition;

#[test]
fn official_channel_rules() {
    let edition = GameEdition::Official;

    // 官服 SDK 文件应命中
    assert!(edition.is_payload_file("hgsdk.dll"));
    assert!(edition.is_payload_file("PlatformProcess.dll"));
    assert!(edition.is_payload_file("webviewsdk.dll"));

    // 公共文件应命中
    assert!(edition.is_payload_file("U8CoreUI.dll"));
    assert!(edition.is_payload_file("U8SDK.dll"));
    assert!(edition.is_payload_file("u8_channel.dll"));

    // 目录前缀应命中
    assert!(edition.is_payload_file("sdkdata/xxx.dat"));
    assert!(edition.is_payload_file("U8Data/config/config.bin"));

    // B服 SDK 不应命中官服规则
    assert!(!edition.is_payload_file("PCGameSDK.dll"));
    assert!(!edition.is_payload_file("BLPlatform64/PCGamePlatform.exe"));
}

#[test]
fn bilibili_channel_rules() {
    let edition = GameEdition::Bilibili;

    // B服 SDK 文件应命中
    assert!(edition.is_payload_file("PCGameSDK.dll"));
    assert!(edition.is_payload_file("BLPlatform64/PCGamePlatform.exe"));

    // 官服 SDK 不应命中 B服规则
    assert!(!edition.is_payload_file("hgsdk.dll"));
    assert!(!edition.is_payload_file("sdkdata/xxx.dat"));
}

#[test]
fn excluded_files() {
    let edition = GameEdition::Official;

    // 游戏本体文件不应被部署
    assert!(!edition.is_payload_file("config.ini"));
    assert!(!edition.is_payload_file("Arknights.exe"));
    assert!(!edition.is_payload_file("Arknights_Data/globalgamemanagers"));
    assert!(!edition.is_payload_file("GameAssembly.dll"));
    assert!(!edition.is_payload_file("UnityPlayer.dll"));
    assert!(!edition.is_payload_file("game_files"));
}
