// SPDX-License-Identifier: Apache-2.0

//! Story 2 — Trust boundary (credential replacement).
//!
//! On the `/trust` prefix, the IPP ext-proc replaces the caller-supplied
//! `Authorization` header with the real upstream credential. A request that
//! carries a bogus client token therefore still reaches llm-katan with a
//! valid key and succeeds.
//!
//! Contrast with [`crate::errors::invalid_api_key_rejected`], which sends the
//! same bogus token to a non-`/trust` path and gets a 401 — proving the swap
//! is scoped to the trust boundary and does not leak onto other routes.

use crate::fixtures::{ensure_gateway_ready, http_client_with_auth, post_chat};

#[tokio::test]
async fn credential_replaced_at_trust_boundary() {
    ensure_gateway_ready().await;
    let client = http_client_with_auth("bogus-caller-token");

    let resp = post_chat(&client, "/trust/v1/chat/completions", "gpt-4", "hello").await;

    assert_eq!(
        resp.status(),
        200,
        "trust-boundary route should replace the caller token with the real \
         upstream credential and succeed, got {}",
        resp.status()
    );
}
