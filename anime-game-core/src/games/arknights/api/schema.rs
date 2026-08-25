use serde::{Deserialize, Serialize};

// ==========================================
// Request structures
// ==========================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchRequest {
    #[serde(rename = "seq")]
    pub seq: String,

    #[serde(rename = "proxy_reqs")]
    pub proxy_reqs: Vec<ProxyRequest>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyRequest {
    #[serde(rename = "kind")]
    pub kind: String,

    #[serde(rename = "get_latest_game_req", skip_serializing_if = "Option::is_none")]
    pub get_latest_game_req: Option<GetLatestGameReq>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetLatestGameReq {
    #[serde(rename = "appcode")]
    pub app_code: String,

    #[serde(rename = "launcher_appcode")]
    pub launcher_app_code: String,

    #[serde(rename = "channel")]
    pub channel: String,

    #[serde(rename = "sub_channel")]
    pub sub_channel: String,

    #[serde(rename = "version")]
    pub version: String
}

// ==========================================
// Response structures
// ==========================================

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BatchResponse {
    #[serde(rename = "proxy_rsps")]
    pub proxy_rsps: Vec<ProxyResponse>
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProxyResponse {
    #[serde(rename = "kind")]
    pub kind: Option<String>,

    #[serde(rename = "get_latest_game_rsp")]
    pub get_latest_game_rsp: Option<GetLatestGameRsp>
}

/// Response of the `get_latest_game` request
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GetLatestGameRsp {
    #[serde(rename = "action")]
    pub action: Option<i32>,

    #[serde(rename = "version")]
    pub version: Option<String>,

    #[serde(rename = "pkg")]
    pub pkg: Option<PkgInfo>,

    #[serde(rename = "patch")]
    pub patch: Option<PatchInfo>,

    #[serde(rename = "pre_patch")]
    pub pre_patch: Option<PatchInfo>
}

/// Full package info
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PkgInfo {
    #[serde(rename = "packs")]
    pub packs: Option<Vec<Pack>>,

    #[serde(rename = "file_path")]
    pub file_path: Option<String>
}

/// Incremental update / predownload info
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PatchInfo {
    #[serde(rename = "url")]
    pub url: Option<String>,

    #[serde(rename = "md5")]
    pub md5: Option<String>,

    #[serde(rename = "package_size")]
    pub package_size: Option<String>,

    #[serde(rename = "total_size")]
    pub total_size: Option<String>,

    #[serde(rename = "patches")]
    pub patches: Option<Vec<Pack>>,

    #[serde(rename = "cd_key")]
    pub cd_key: Option<String>,

    #[serde(rename = "version")]
    pub version: Option<String>,

    #[serde(rename = "v2_patch_info_url")]
    pub v2_patch_info_url: Option<String>,

    #[serde(rename = "v2_patch_info_md5")]
    pub v2_patch_info_md5: Option<String>
}

/// A single downloadable pack
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Pack {
    #[serde(rename = "url")]
    pub url: String,

    #[serde(rename = "md5")]
    pub md5: Option<String>,

    #[serde(rename = "package_size")]
    pub package_size: Option<String>
}

/// V2 delta patch manifest (patch.json)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PatchManifest {
    #[serde(rename = "version")]
    pub version: Option<String>,

    #[serde(rename = "vfs_base_path")]
    pub vfs_base_path: Option<String>,

    #[serde(rename = "files")]
    pub files: Option<Vec<PatchFile>>
}

/// A single file entry in the delta patch manifest
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PatchFile {
    #[serde(rename = "name")]
    pub name: Option<String>,

    #[serde(rename = "md5")]
    pub md5: Option<String>,

    #[serde(rename = "size")]
    pub size: Option<i64>,

    /// 0 = static copy, 1 = hdiff patch, 2 = verify only
    #[serde(rename = "diffType")]
    pub diff_type: Option<i32>,

    #[serde(rename = "local_path")]
    pub local_path: Option<String>,

    #[serde(rename = "patch")]
    pub patch: Option<Vec<PatchNode>>
}

/// A single hdiff patch node
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PatchNode {
    #[serde(rename = "base_file")]
    pub base_file: Option<String>,

    #[serde(rename = "base_md5")]
    pub base_md5: Option<String>,

    #[serde(rename = "patch")]
    pub patch: Option<String>
}

/// Game files manifest entry (game_files, JSONL)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GameFileEntry {
    #[serde(rename = "path")]
    pub path: String,

    #[serde(rename = "md5")]
    pub md5: String,

    #[serde(rename = "size")]
    pub size: i64
}
