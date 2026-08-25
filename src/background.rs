use std::path::Path;
use std::process::Command;

use anime_launcher_sdk::anime_game_core::installer::downloader::Downloader;
use anime_launcher_sdk::is_available;
use anime_launcher_sdk::anime_game_core::minreq;
use anyhow::Context;
use md5::{Digest, Md5};

pub fn download_background(with_video: bool, index: u8) -> anyhow::Result<()> {
    tracing::debug!("Downloading background picture");

    let info = get_background_info(index)?;

    let regenerate_image = info.download(with_video)?;

    if regenerate_image {
        if gtk_webp_image_supported() {
            std::fs::copy(&*crate::BACKGROUND_FILE, &*crate::PROCESSED_BACKGROUND_FILE)
                .context("Copying background file")?;
            if matches!(info, BackgroundSpec::Video { .. }) {
                std::fs::copy(
                    &*crate::BACKGROUND_OVERLAY_FILE,
                    &*crate::PROCESSED_BACKGROUND_OVERLAY_FILE
                )
                .context("Copying background overlay file")?;
            }
        }
        else {
            tracing::info!("WebP GDK Pixbuf Loader is not installed, converting images to PNG");
            info.convert_and_copy()?;
        }

        if matches!(info, BackgroundSpec::Normal { .. }) {
            // Remove the overlay and video file if it's normal variant
            // Ignore error, if file is already missing for example
            let _ = std::fs::remove_file(&*crate::PROCESSED_BACKGROUND_OVERLAY_FILE);
            let _ = std::fs::remove_file(&*crate::BACKGROUND_VIDEO_FILE);
        }
    }
    else {
        tracing::debug!("Not re-generating the background image, already latest")
    }

    Ok(())
}

#[cached::proc_macro::cached(result)]
pub fn get_background_info_multiple() -> anyhow::Result<Vec<BackgroundSpec>> {
    let uri = get_uri();

    if uri.is_empty() {
        return Ok(Vec::new());
    }

    Ok(vec![BackgroundSpec::Normal {
        background: Background::from_uri(uri)
    }])
}

#[cached::proc_macro::cached(result)]
pub fn get_background_info(index: u8) -> anyhow::Result<BackgroundSpec> {
    let uri = get_uri();

    if uri.is_empty() {
        anyhow::bail!("Failed to get background URI");
    }

    Ok(BackgroundSpec::Normal {
        background: Background::from_uri(uri)
    })
}

/// Get the background image URI from the Hypergryph web API
///
/// Uses the `batch_proxy` endpoint with the `get_main_bg_image` kind.
pub fn get_uri() -> String {
    let body = serde_json::json!({
        "seq": "5",
        "proxy_reqs": [{
            "kind": "get_main_bg_image",
            "get_main_bg_image_req": {
                "appcode": "6LL0KJuqHBVz33WK",
                "language": "zh-cn",
                "channel": "1",
                "sub_channel": "1",
                "platform": "Windows",
                "source": "launcher"
            }
        }]
    });

    let response = minreq::post(concat!(
        "https://launcher.",
        "hypergryph",
        ".com/api/proxy/web/batch_proxy"
    ))
    .with_timeout(15)
    .with_header("Content-Type", "application/json")
    .with_header("User-Agent", "XelLauncher/0.2.5")
    .with_body(body.to_string())
    .send();

    match response {
        Ok(response) => {
            let json: serde_json::Value = serde_json::from_slice(response.as_bytes()).unwrap_or_default();

            json["proxy_rsps"][0]["get_main_bg_image_rsp"]["main_bg_image"]["url"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        }
        Err(err) => {
            tracing::error!("Failed to get background URI: {err}");
            String::new()
        }
    }
}

#[derive(Debug, Clone)]
pub enum BackgroundSpec {
    Normal {
        background: Background
    },
    Video {
        background: Background,
        video: Background,
        overlay: Background
    }
}

impl BackgroundSpec {
    fn background(&self) -> &Background {
        match self {
            Self::Normal {
                background
            }
            | Self::Video {
                background, ..
            } => background
        }
    }

    /// Returns true if the background needs to be re-generated
    fn download(&self, with_video: bool) -> anyhow::Result<bool> {
        let mut regenerate_image = false;

        regenerate_image |= self.background().download(&crate::BACKGROUND_FILE)?;

        if let Self::Video {
            video,
            overlay,
            ..
        } = self
        {
            regenerate_image |= overlay.download(&crate::BACKGROUND_OVERLAY_FILE)?;
            if with_video {
                regenerate_image |= video.download(&crate::BACKGROUND_VIDEO_FILE)?;
            }
        }

        Ok(regenerate_image)
    }

    fn convert_and_copy(&self) -> anyhow::Result<()> {
        finalize_file(
            self.background(),
            &crate::BACKGROUND_FILE,
            &crate::PROCESSED_BACKGROUND_FILE
        )?;
        if let Self::Video {
            overlay, ..
        } = self
        {
            finalize_file(
                overlay,
                &crate::BACKGROUND_OVERLAY_FILE,
                &crate::PROCESSED_BACKGROUND_OVERLAY_FILE
            )?;
        }
        Ok(())
    }
}

fn finalize_file(bg_info: &Background, from: &Path, to: &Path) -> anyhow::Result<()> {
    if bg_info.uri.ends_with(".webp") {
        convert_image(from, to).context(format!("Converting image {to:?}"))?;
    }

    // If it failed to re-code the file - just copy it
    // Will happen with HSR because devs apparently named
    // their background image ".webp" while it's JPEG
    if !to.exists() {
        std::fs::copy(from, to).context(format!("Copying {to:?}"))?;
    }

    Ok(())
}

fn convert_image(from: &Path, to: &Path) -> anyhow::Result<()> {
    if is_available("dwebp") {
        Command::new("dwebp")
            .arg(from)
            .arg("-o")
            .arg(to)
            .spawn()?
            .wait()?;
    }
    else if is_available("magick") {
        Command::new("magick")
            .arg(from)
            .arg(format!("PNG:{}", to.display()))
            .spawn()?
            .wait()?;
    }
    else {
        tracing::warn!("Could not find `dwebp` or `magick` to convert the image file.");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Background {
    pub uri: String,
    pub hash: String
}

impl Background {
    fn from_uri(uri: String) -> Self {
        let hash = get_img_hash_from_uri(&uri);
        Self {
            uri,
            hash
        }
    }

    /// Return true if the background needs to be re-generated
    fn download(&self, path: &Path) -> anyhow::Result<bool> {
        if !check_img_file(path, &self.hash)? {
            download_img_file(path, &self.uri)?;
            return Ok(true);
        }
        Ok(false)
    }
}

/// Returns true if image exists and is correct
fn check_img_file(path: &Path, expected_hash: &str) -> anyhow::Result<bool> {
    if path.exists() {
        let hash = Md5::digest(std::fs::read(path)?);

        if format!("{hash:x}").eq_ignore_ascii_case(expected_hash) {
            tracing::debug!("Background picture {path:?} already downloaded. Skipping");

            return Ok(true);
        }
    }

    Ok(false)
}


fn get_img_hash_from_uri(uri: &str) -> String {
    uri.split('/')
        .next_back()
        .unwrap_or_default()
        .split('_')
        .next()
        .unwrap_or_default()
        .to_owned()
}

#[cached::proc_macro::once()]
fn gtk_webp_image_supported() -> bool {
    let supported_pixbuf_formats = gtk::gdk_pixbuf::Pixbuf::formats();
    supported_pixbuf_formats.into_iter().any(|format| {
        format
            .name()
            .map(|name| name.eq_ignore_ascii_case("webp"))
            .unwrap_or(false)
            || format
                .extensions()
                .iter()
                .any(|ext| ext.eq_ignore_ascii_case("webp"))
    })
}

fn download_img_file(path: &Path, uri: &str) -> anyhow::Result<()> {
    let mut downloader = Downloader::new(uri)?;

    downloader.continue_downloading = false;

    if let Err(err) = downloader.download(path, |_, _| {}) {
        anyhow::bail!(err);
    }

    Ok(())
}
