use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header::VARY},
    middleware::Next,
    response::{IntoResponse, Response},
};

const CROSS_ORIGIN_RESOURCE_POLICY: HeaderName =
    HeaderName::from_static("cross-origin-resource-policy");

fn is_cross_site_request(headers: &HeaderMap) -> bool {
    headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("cross-site"))
}

fn add_resource_policy(response: &mut Response) {
    response.headers_mut().insert(
        CROSS_ORIGIN_RESOURCE_POLICY,
        HeaderValue::from_static("same-site"),
    );
    response
        .headers_mut()
        .append(VARY, HeaderValue::from_static("Sec-Fetch-Site"));
}

pub async fn prevent_hotlinking(request: Request, next: Next) -> Response {
    if is_cross_site_request(request.headers()) {
        tracing::debug!(
            path = %request.uri().path(),
            "blocked cross-site asset request"
        );
        let mut response = (
            StatusCode::FORBIDDEN,
            "Cross-site asset embedding is not allowed",
        )
            .into_response();
        add_resource_policy(&mut response);
        return response;
    }

    let mut response = next.run(request).await;
    add_resource_policy(&mut response);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cross_site_browser_requests() {
        let mut headers = HeaderMap::new();
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));

        assert!(is_cross_site_request(&headers));
    }

    #[test]
    fn permits_same_site_and_direct_requests() {
        let mut headers = HeaderMap::new();
        assert!(!is_cross_site_request(&headers));

        headers.insert("sec-fetch-site", HeaderValue::from_static("same-site"));
        assert!(!is_cross_site_request(&headers));

        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        assert!(!is_cross_site_request(&headers));
    }

    #[test]
    fn adds_same_site_resource_policy() {
        let mut response = StatusCode::OK.into_response();
        add_resource_policy(&mut response);

        assert_eq!(
            response
                .headers()
                .get(CROSS_ORIGIN_RESOURCE_POLICY)
                .and_then(|value| value.to_str().ok()),
            Some("same-site")
        );
        assert_eq!(
            response
                .headers()
                .get(VARY)
                .and_then(|value| value.to_str().ok()),
            Some("Sec-Fetch-Site")
        );
    }
}
