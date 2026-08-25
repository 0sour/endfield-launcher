//! 验证 batch_proxy API 客户端（需要网络，默认忽略）
use anime_game_core::arknights::api;
use anime_game_core::arknights::consts::GameEdition;

#[test]
#[ignore = "requires network access"]
fn request_latest_game_info() {
    let response = api::request(GameEdition::Official).expect("API request failed");

    let version = response.version.expect("no version");
    println!("方舟官服最新版本: {version}");

    let pkg = response.pkg.expect("no pkg");
    let packs = pkg.packs.unwrap_or_default();
    println!("全量包数量: {}", packs.len());
    assert!(!packs.is_empty(), "no packs");

    let file_path = pkg.file_path.expect("no file_path");
    println!("资源基础 URL: {file_path}");
}
