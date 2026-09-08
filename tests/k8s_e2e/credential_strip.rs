// SPDX-License-Identifier: Apache-2.0

//! P2 · #422 — client credentials must never cross the trust boundary.
//!
//! The [`crate::trust_boundary`] test only checks that a bogus caller token
//! still yields 200 — it cannot see whether the caller's credential *also*
//! leaked to the upstream. This suite proves the **negative** using the
//! in-cluster `echo-backend`, which reflects the headers Envoy actually
//! forwarded (post-ext-proc mutation) back in the response body.
//!
//! On the `/echo` prefix the IPP `headers` filter strips the caller's
//! `x-api-key` and replaces `Authorization` with the real upstream token, so a
//! request bearing bogus client credentials reaches the upstream carrying only
//! the injected credential.
//!
//! Scope note: this exercises the *exact-name* strip mechanism. Prefix-based
//! stripping (`x-maas-*`) is not expressible via praxis `headers`
//! `request_remove` — the same gap PR #52's native module works around.

use crate::fixtures::{ensure_gateway_ready, gateway_url};

const BOGUS_TOKEN: &str = "Bearer bogus-caller-token";
const CLIENT_API_KEY: &str = "client-secret-key";
const INJECTED_AUTH: &str = "Bearer echo-upstream-secret";

/// POST a benign chat completion to `/echo/v1/...` carrying bogus client
/// credentials, and return the headers the echo-backend saw upstream.
async fn upstream_headers() -> serde_json::Map<String, serde_json::Value> {
    let url = format!("{}/echo/v1/chat/completions", gateway_url());
    let resp = reqwest::Client::builder()
        .timeout(crate::fixtures::REQUEST_TIMEOUT)
        .build()
        .expect("failed to build HTTP client")
        .post(&url)
        .header("Authorization", BOGUS_TOKEN)
        .header("x-api-key", CLIENT_API_KEY)
        .json(&serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "hello there"}]
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 200, "echo-backend should return 200");

    let body: serde_json::Value = resp.json().await.expect("echo response was not JSON");
    body.get("headers")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .expect("echo response missing `headers` object")
}

#[tokio::test]
async fn client_api_key_stripped_before_upstream() {
    ensure_gateway_ready().await;

    let headers = upstream_headers().await;

    assert!(
        !headers.contains_key("x-api-key"),
        "caller's x-api-key must be stripped before the upstream, but the \
         echo-backend saw it: {headers:?}"
    );
}

#[tokio::test]
async fn caller_authorization_replaced_not_forwarded() {
    ensure_gateway_ready().await;

    let headers = upstream_headers().await;
    let auth = headers
        .get("authorization")
        .and_then(serde_json::Value::as_str)
        .expect("upstream request had no Authorization header");

    assert_eq!(
        auth, INJECTED_AUTH,
        "upstream should carry the injected credential, got {auth:?}"
    );
    assert_ne!(
        auth, BOGUS_TOKEN,
        "caller's bogus token must not reach the upstream"
    );
}
