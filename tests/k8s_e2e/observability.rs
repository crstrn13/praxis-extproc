// SPDX-License-Identifier: Apache-2.0

//! Story 6 — Observability (request-ID correlation).
//!
//! The IPP `request_id` filter echoes a client-supplied `X-Correlation-ID`
//! back on the response so callers can correlate their request with its
//! response.
//!
//! Note: a custom header is used deliberately. Envoy generates and overwrites
//! the reserved `x-request-id` before the ext-proc sees it, so a caller's own
//! `x-request-id` value can never round-trip; a custom correlation header does.

use crate::fixtures::{ensure_gateway_ready, gateway_url, http_client};

#[tokio::test]
async fn correlation_id_echoed() {
    ensure_gateway_ready().await;
    let client = http_client();
    let url = format!("{}/v1/chat/completions", gateway_url());
    let correlation_id = "e2e-corr-1234567890";

    let resp = client
        .post(&url)
        .header("X-Correlation-ID", correlation_id)
        .json(&serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 200);

    let echoed = resp
        .headers()
        .get("X-Correlation-ID")
        .expect("missing X-Correlation-ID — request_id filter did not echo the client ID")
        .to_str()
        .expect("header value not valid UTF-8");
    assert_eq!(
        echoed, correlation_id,
        "echoed correlation ID should match the client's"
    );
}
