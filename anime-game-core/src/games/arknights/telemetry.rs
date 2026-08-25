use super::consts::GameEdition;

/// Check if the game telemetry is disabled
///
/// Returns `Ok(Some(server))` if the given telemetry server is still
/// reachable (i.e. not blocked), and `Ok(None)` if all servers are
/// unreachable (i.e. telemetry is disabled).
pub fn is_disabled(game_edition: GameEdition) -> anyhow::Result<Option<String>> {
    for server in game_edition.telemetry_servers() {
        if crate::check_domain::available(server)? {
            return Ok(Some(server.to_string()));
        }
    }

    Ok(None)
}
