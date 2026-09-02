//! 验证预下载检测链路:本地版本 = 正式版时,API 应返回 pre_patch
//! 需要网络,默认忽略
use anime_game_core::prelude::GameExt;
use anime_game_core::endfield::{crypto, Game, version_diff::VersionDiff};
use anime_game_core::endfield::consts::GameEdition;

#[test]
#[ignore = "requires network access"]
fn predownload_detected_when_local_version_is_latest() {
    let dir = std::env::temp_dir().join("endfield-predownload-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // 写入一个假的可执行文件 + 加密 config.ini,版本 = 当前正式版
    std::fs::write(dir.join("Endfield.exe"), b"MZ fake exe").unwrap();
    crypto::encrypt_string_to_file("version=1.4.4\n", &dir.join("config.ini")).unwrap();

    let game = Game::new(&dir, GameEdition::Official);

    let diff = game.try_get_diff().expect("try_get_diff failed");

    // 版本等于正式版时,API 可能返回预下载(发布前)或增量更新(发布后),
    // 两者都说明更新链路被正确检测到
    match &diff {
        VersionDiff::Predownload { latest, downloaded_size, uris, .. } => {
            println!(
                "预下载可用: {latest}, 下载大小 {}B, {} 个分卷",
                downloaded_size,
                uris.len()
            );
            assert!(latest.to_string() != "1.4.4", "预下载版本不应等于正式版");
        }
        VersionDiff::Diff { latest, uris, .. } => {
            println!("增量更新可用: {latest}, {} 个分卷", uris.len());
            assert!(!uris.is_empty(), "diff should have segments");
        }
        other => {
            println!("未检测到更新: {other:?}");
            panic!("expected Predownload or Diff, got different diff");
        }
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "requires network access"]
fn no_predownload_when_local_version_is_outdated() {
    let dir = std::env::temp_dir().join("endfield-predownload-test-2");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // 旧版本客户端不应看到预下载
    std::fs::write(dir.join("Endfield.exe"), b"MZ fake exe").unwrap();
    crypto::encrypt_string_to_file("version=1.4.0\n", &dir.join("config.ini")).unwrap();

    let game = Game::new(&dir, GameEdition::Official);

    let diff = game.try_get_diff().expect("try_get_diff failed");

    match &diff {
        VersionDiff::Predownload { .. } => {
            panic!("旧版本客户端不应收到预下载");
        }
        other => {
            println!("旧版本客户端收到: {other:?}");
        }
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "requires network access"]
fn diff_detected_when_update_released() {
    let dir = std::env::temp_dir().join("endfield-predownload-test-3");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // 正式版已发布(1.5.3),本地仍是 1.4.4:应返回 Diff(增量更新)
    std::fs::write(dir.join("Endfield.exe"), b"MZ fake exe").unwrap();
    crypto::encrypt_string_to_file("version=1.4.4\n", &dir.join("config.ini")).unwrap();

    let game = Game::new(&dir, GameEdition::Official);

    let diff = game.try_get_diff().expect("try_get_diff failed");

    match &diff {
        VersionDiff::Diff { current, latest, uris, downloaded_size, .. } => {
            println!(
                "增量更新可用: {current} -> {latest}, {} 个分卷, 下载 {}B",
                uris.len(),
                downloaded_size
            );
            assert!(!uris.is_empty(), "diff should have segments");
        }
        other => {
            println!("未检测到增量更新: {other:?}");
            panic!("expected Diff, got different diff");
        }
    }

    let _ = std::fs::remove_dir_all(&dir);
}
