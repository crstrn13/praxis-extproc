// SPDX-License-Identifier: Apache-2.0

//! Story 4 — Token accounting.
//!
//! The IPP `token_count` filter parses usage from the upstream response and
//! `token_usage_headers` is intended to surface it as `Praxis-Token-*`
//! response headers.
//!
//! IGNORED — KNOWN GAP. This does not work in praxis-extproc today. The
//! ext-proc runs the response pipeline in two passes: all `on_response`
//! (header) hooks first, then all `on_response_body` hooks. `token_count`
//! sets its usage metadata in the body hook, so `token_usage_headers` (a
//! header hook) runs *before* the metadata exists and emits nothing. The
//! upstream (llm-katan) *does* return a valid `usage` object — verified by
//! hand — so this is purely an ext-proc response-ordering limitation, not a
//! backend or config problem. Re-enable once praxis-extproc lets header
//! mutations reflect body-derived metadata (e.g. re-running header filters
//! after the body phase, or emitting counts as response trailers).

use crate::fixtures::{chat_completion, ensure_gateway_ready};

#[tokio::test]
#[ignore = "known gap: header hooks run before body hooks, so token_count \
            metadata is unavailable to token_usage_headers (see module docs)"]
async fn token_usage_headers_present() {
    ensure_gateway_ready().await;
    let resp = chat_completion("gpt-4", "count my tokens").await;

    assert_eq!(resp.status(), 200);

    let total = resp
        .headers()
        .get("Praxis-Token-Total")
        .expect(
            "missing Praxis-Token-Total — token_count did not parse usage \
             (upstream omitted `usage`, or token_count/token_usage_headers not applied)",
        )
        .to_str()
        .expect("header value not valid UTF-8");

    assert!(
        total.parse::<u64>().is_ok_and(|n| n > 0),
        "Praxis-Token-Total should be a positive integer, got {total:?}"
    );

    for header in ["Praxis-Token-Input", "Praxis-Token-Output"] {
        assert!(resp.headers().contains_key(header), "missing {header} response header");
    }
}
