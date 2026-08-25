pub mod schema;

use cached::proc_macro::cached;

use super::consts::GameEdition;

/// Request the latest game info from the Hypergryph launcher API
///
/// Uses the `batch_proxy` endpoint with the `get_latest_game` kind.
/// The result is cached per edition.
#[cached(key = "GameEdition", convert = r#"{ game_edition }"#, result)]
pub fn request(game_edition: GameEdition) -> anyhow::Result<schema::GetLatestGameRsp> {
    tracing::trace!("Requesting latest game info for {game_edition:?}");

    let body = schema::BatchRequest {
        seq: game_edition.seq().to_string(),
        proxy_reqs: vec![schema::ProxyRequest {
            kind: "get_latest_game".to_string(),
            get_latest_game_req: Some(schema::GetLatestGameReq {
                app_code: game_edition.app_code().to_string(),
                launcher_app_code: game_edition.launcher_app_code().to_string(),
                channel: game_edition.channel().to_string(),
                sub_channel: game_edition.sub_channel().to_string(),
                version: String::new()
            })
        }]
    };

    let response: schema::BatchResponse = minreq::post(game_edition.api_uri())
        .with_timeout(*crate::REQUESTS_TIMEOUT)
        .with_header("Content-Type", "application/json")
        .with_header("User-Agent", "XelLauncher/0.2.5")
        .with_body(serde_json::to_string(&body)?)
        .send()?
        .json()?;

    response
        .proxy_rsps
        .into_iter()
        .find(|rsp| rsp.kind.as_deref() == Some("get_latest_game"))
        .and_then(|rsp| rsp.get_latest_game_rsp)
        .ok_or_else(|| anyhow::anyhow!("Failed to find the game in the API"))
}
