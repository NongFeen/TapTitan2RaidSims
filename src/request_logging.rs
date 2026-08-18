use std::{fmt, time::Duration};

use axum::http::{Request, Response};
use tower_http::trace::{MakeSpan, OnFailure, OnRequest, OnResponse};
use tracing::Span;

#[derive(Clone, Copy, Debug, Default)]
pub struct FilteredHttpTrace;

fn suppress_request_log(path: &str) -> bool {
    path.starts_with("/api/players/")
}

impl<B> MakeSpan<B> for FilteredHttpTrace {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        if suppress_request_log(request.uri().path()) {
            Span::none()
        } else {
            tracing::debug_span!(
                "request",
                method = %request.method(),
                uri = %request.uri(),
                version = ?request.version(),
            )
        }
    }
}

impl<B> OnRequest<B> for FilteredHttpTrace {
    fn on_request(&mut self, _request: &Request<B>, span: &Span) {
        if !span.is_disabled() {
            span.in_scope(|| tracing::debug!("started processing request"));
        }
    }
}

impl<B> OnResponse<B> for FilteredHttpTrace {
    fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
        if !span.is_disabled() {
            span.in_scope(|| {
                tracing::debug!(
                    latency_ms = latency.as_millis(),
                    status = response.status().as_u16(),
                    "finished processing request"
                )
            });
        }
    }
}

impl<FailureClass> OnFailure<FailureClass> for FilteredHttpTrace
where
    FailureClass: fmt::Display,
{
    fn on_failure(&mut self, failure_classification: FailureClass, latency: Duration, span: &Span) {
        if !span.is_disabled() {
            span.in_scope(|| {
                tracing::error!(
                    classification = %failure_classification,
                    latency_ms = latency.as_millis(),
                    "response failed"
                )
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_player_descendants_only() {
        assert!(suppress_request_log("/api/players/933qd64"));
        assert!(suppress_request_log("/api/players/933qd64/simulation-jobs"));
        assert!(!suppress_request_log("/api/players"));
        assert!(!suppress_request_log("/api/live-current-boss"));
        assert!(!suppress_request_log("/api/players-extra/example"));
    }
}
