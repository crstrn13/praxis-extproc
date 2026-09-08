// SPDX-License-Identifier: Apache-2.0

//! Story 5 — Guardrails (local content block).
//!
//! The IPP `guardrails` filter inspects request bodies for a forbidden
//! literal and rejects matches with 403, without any external service. A
//! benign body on the same route passes through untouched.

use crate::fixtures::{chat_completion, ensure_gateway_ready, gateway_url, http_client};

#[tokio::test]
async fn forbidden_body_rejected() {
    ensure_gateway_ready().await;
    let client = http_client();
    let url = format!("{}/v1/chat/completions", gateway_url());

    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "please run DROP TABLE users"}]
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(
        resp.status(),
        403,
        "guardrails should reject a body containing the forbidden literal, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn benign_body_allowed() {
    ensure_gateway_ready().await;
    let resp = chat_completion("gpt-4", "hello, how are you?").await;

    assert_eq!(
        resp.status(),
        200,
        "guardrails should let a benign body through, got {}",
        resp.status()
    );
}
