//! 验证多分卷预下载链路:download_to 应下载全部 segments,且已下载的分卷被跳过
//!
//! 使用本地 HTTP 服务器(支持 Range 请求)模拟 CDN 分卷,不依赖外网。
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use anime_game_core::prelude::*;
use anime_game_core::endfield::prelude::*;

/// 极简 HTTP 服务器:支持 HEAD + GET(Range)请求,从内存 map 提供文件
fn start_server(files: Arc<HashMap<String, Vec<u8>>>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let files = Arc::clone(&files);
                thread::spawn(move || handle(stream, &files));
            }
        }
    });

    port
}

fn handle(mut stream: TcpStream, files: &HashMap<String, Vec<u8>>) {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).unwrap_or(0);
    if n == 0 {
        return;
    }

    let request = String::from_utf8_lossy(&buf[..n]);
    let mut lines = request.lines();

    let mut parts = lines.next().unwrap_or_default().split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    // 去掉 query 参数,文件名匹配
    let clean_path = path.split('?').next().unwrap_or(&path).to_string();

    let Some(file) = files.get(&clean_path) else {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
        return;
    };

    let content_length = file.len() as u64;

    // 解析 Range: bytes=N-
    let range_header = lines
        .map(|line| line.to_ascii_lowercase())
        .find(|line| line.starts_with("range:"));

    let start = range_header
        .and_then(|line| line.split_once(':').map(|(_, v)| v.trim().to_string()))
        .and_then(|range| range.strip_prefix("bytes=").map(|r| r.to_string()))
        .and_then(|range| range.split('-').next().map(|s| s.parse::<u64>().ok()))
        .flatten()
        .unwrap_or(0);

    if method == "HEAD" {
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {content_length}\r\n\r\n"
        );
        return;
    }

    // GET
    let data = &file[start as usize..];

    if start == 0 {
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
            data.len()
        );
    }
    else {
        let _ = write!(
            stream,
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\n\r\n",
            start,
            content_length - 1,
            content_length,
            data.len()
        );
    }

    let _ = stream.write_all(data);
}

fn make_predownload(
    uris: Vec<String>,
    downloaded_size: u64
) -> VersionDiff {
    VersionDiff::Predownload {
        current: Version::from_str("1.4.4").unwrap(),
        latest: Version::from_str("1.5.3").unwrap(),
        uris,
        edition: GameEdition::Official,
        downloaded_size,
        unpacked_size: downloaded_size,
        installation_path: None,
        version_file_path: None,
        temp_folder: None
    }
}

fn progress_stub() -> impl Fn(u64, u64) + Send + 'static {
    |_curr, _total| {}
}

#[test]
fn download_all_segments() {
    let dir = std::env::temp_dir().join("endfield-multiseg-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // 模拟 3 个分卷(刻意给 URL 加 query 参数验证文件名解析)
    let seg1: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let seg2: Vec<u8> = (0..512).map(|i| (i % 251) as u8).collect();
    let seg3: Vec<u8> = (0..128).map(|i| (255 - i as u8)).collect();

    let files = Arc::new(HashMap::from([
        ("/seg.zip.001".to_string(), seg1.clone()),
        ("/seg.zip.002".to_string(), seg2.clone()),
        ("/seg.zip.003".to_string(), seg3.clone())
    ]));

    let port = start_server(files);
    let base = format!("http://127.0.0.1:{port}");

    let uris = vec![
        format!("{base}/seg.zip.001?auth_key=test"),
        format!("{base}/seg.zip.002?auth_key=test"),
        format!("{base}/seg.zip.003?auth_key=test")
    ];

    let total = (seg1.len() + seg2.len() + seg3.len()) as u64;

    let mut diff = make_predownload(uris, total);

    // 初始状态:未下载任何分卷
    assert!(!diff.is_downloaded(&dir), "is_downloaded should be false initially");

    // 下载全部分卷
    let last_progress = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    diff.download_to(&dir, {
        let last_progress = std::sync::Arc::clone(&last_progress);
        move |curr, _total| {
            last_progress.store(curr, std::sync::atomic::Ordering::Relaxed);
        }
    })
    .expect("download_to failed");

    // 进度应累计到总大小
    assert_eq!(
        last_progress.load(std::sync::atomic::Ordering::Relaxed),
        total,
        "progress should reach total size"
    );

    // 3 个分卷都应存在且内容正确(文件名来自 URL,去掉了 query)
    for (name, content) in [("seg.zip.001", &seg1), ("seg.zip.002", &seg2), ("seg.zip.003", &seg3)] {
        let path = dir.join(name);
        assert!(path.exists(), "{name} should exist");
        assert_eq!(&std::fs::read(&path).unwrap(), content, "{name} content mismatch");
    }

    // 现在应认为已全部下载
    assert!(diff.is_downloaded(&dir), "is_downloaded should be true after download");

    // 再次调用应幂等(直接成功,不重新下载)
    diff.download_to(&dir, progress_stub()).expect("second download_to failed");

    // 文件内容不应被改动
    assert_eq!(std::fs::read(dir.join("seg.zip.001")).unwrap(), seg1);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn partial_download_resumes() {
    let dir = std::env::temp_dir().join("endfield-multiseg-test-2");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // 第一个分卷已完整下载(模拟用户中断后再继续)
    let seg1: Vec<u8> = (0..1024).map(|i| (i % 128) as u8).collect();
    let seg2: Vec<u8> = (0..512).map(|i| i as u8).collect();

    std::fs::write(dir.join("seg.zip.001"), &seg1).unwrap();

    let files = Arc::new(HashMap::from([
        ("/seg.zip.001".to_string(), seg1.clone()),
        ("/seg.zip.002".to_string(), seg2.clone())
    ]));

    let port = start_server(files);
    let base = format!("http://127.0.0.1:{port}");

    let total = (seg1.len() + seg2.len()) as u64;

    let mut diff = make_predownload(
        vec![
            format!("{base}/seg.zip.001"),
            format!("{base}/seg.zip.002")
        ],
        total
    );

    // 部分完成:第一个在,第二个不在
    assert!(!diff.is_downloaded(&dir));

    diff.download_to(&dir, progress_stub()).expect("download_to failed");

    // 两个分卷都应存在
    assert!(dir.join("seg.zip.001").exists());
    assert_eq!(std::fs::read(dir.join("seg.zip.002")).unwrap(), seg2);

    assert!(diff.is_downloaded(&dir));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn legacy_query_filename_migrated() {
    let dir = std::env::temp_dir().join("endfield-multiseg-test-3");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let seg1: Vec<u8> = (0..1024).map(|i| (i % 200) as u8).collect();
    let seg2: Vec<u8> = (0..256).map(|i| i as u8).collect();

    // 用户已有的旧格式文件:名字带 ?auth_key=... (模拟 fad0262 修复前的行为)
    std::fs::write(dir.join("seg.zip.001?auth_key=old-token"), &seg1).unwrap();

    let files = Arc::new(HashMap::from([
        ("/seg.zip.001".to_string(), seg1.clone()),
        ("/seg.zip.002".to_string(), seg2.clone())
    ]));

    let port = start_server(files);
    let base = format!("http://127.0.0.1:{port}");

    let total = (seg1.len() + seg2.len()) as u64;

    let mut diff = make_predownload(
        vec![
            format!("{base}/seg.zip.001"),
            format!("{base}/seg.zip.002")
        ],
        total
    );

    diff.download_to(&dir, progress_stub()).expect("download_to failed");

    // 旧文件应被迁移为干净文件名(而不是重新下载)
    assert!(
        !dir.join("seg.zip.001?auth_key=old-token").exists(),
        "legacy file should be renamed"
    );
    assert_eq!(std::fs::read(dir.join("seg.zip.001")).unwrap(), seg1);
    assert_eq!(std::fs::read(dir.join("seg.zip.002")).unwrap(), seg2);

    assert!(diff.is_downloaded(&dir));

    let _ = std::fs::remove_dir_all(&dir);
}

// 辅助:确保 PathBuf 引用在测试中可用(避免未使用警告)
#[allow(dead_code)]
fn _unused(_: &Path) {}
#[allow(dead_code)]
fn _unused2(_: PathBuf) {}
