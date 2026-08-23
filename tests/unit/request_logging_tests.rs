use super::*;

#[test]
fn hides_player_descendants_only() {
    assert!(suppress_request_log("/api/players/933qd64"));
    assert!(suppress_request_log("/api/players/933qd64/simulation-jobs"));
    assert!(!suppress_request_log("/api/players"));
    assert!(!suppress_request_log("/api/live-current-boss"));
    assert!(!suppress_request_log("/api/players-extra/example"));
}
