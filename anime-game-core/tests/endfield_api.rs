//! 验证终末地 batch_proxy API 客户端（需要网络，默认忽略）
use anime_game_core::endfield::api;
use anime_game_core::endfield::consts::GameEdition;

#[test]
#[ignore = "requires network access"]
fn request_latest_game_info() {
    let response = api::request(GameEdition::Official, String::new()).expect("API request failed");

    let version = response.version.expect("no version");
    println!("终末地官服最新版本: {version}");

    let pkg = response.pkg.expect("no pkg");
    let packs = pkg.packs.unwrap_or_default();
    println!("全量包数量: {}", packs.len());
    assert!(!packs.is_empty(), "no packs");
}
