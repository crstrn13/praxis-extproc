# IPP → Praxis-ExtProc Parity Validation

Living tracker for validating that praxis-extproc is a drop-in replacement for
the Go IPP data plane, per the RHOAI 3.6 migration plan. This is the
"what is missing" ledger that E1 (port IPP E2E) and E3 (goldens) fill in.

> **Status: draft, `main` branch.** Verified against the code on
> 2026-09-08. Numbers/coverage will drift — re-derive before trusting.

## Sources of truth

- **Behavioral oracle:** `opendatahub-io/ai-gateway-payload-processing`
  `test/e2e/e2e_test.go` — Ginkgo suite, 7 per-provider specs × 5 providers +
  1 global, all against the **llm-katan simulator** (`3.147.232.199`, the same
  endpoint this repo's e2e uses). No real provider keys required.
- **Must-port ledger:** the plan's Appendix A (downstream IPP fixes classified
  Port / Covered / Obsolete).
- **Plan bar:** "IPP E2E green" is the *floor*; true parity = E1 + all
  Appendix-A must-ports + E3 goldens (streaming, 401/403 ordering, fail-closed,
  redaction).

## 0. The blocker under everything: the ai pin — ✅ DONE (2026-09-08)

**Bumped.** `praxis-ai-*` now pins `rev = 359559ab` (**v0.3.0**), and
`praxis-core`/`praxis-filter` moved from the git tag `v0.5.2` to the **crates.io
registry `0.5.4`** (`Cargo.toml:26-30`). The registry switch matters: ai v0.3.0
depends on the *registry* `praxis-proxy-core`, so keeping praxis on a git source
resolved a second, incompatible copy → `PipelineExtension` trait-bound mismatch.
Using the registry version unifies them. Build + 91 lib + 39 gRPC tests green,
clippy clean.

What the bump unlocked (was pin-blocked at `e9fb521`):

| Capability | e9fb521 | v0.3.0 (now) | Notes |
|---|---|---|---|
| `model_to_header`, routing | ✅ | ✅ | BBR works |
| Anthropic translation (`anthropic_to_openai`, stream events, responses↔chat) | ✅ | ✅ | translation testable |
| `credential_inject` | ❌ | ✅ (`register.rs:148`) | trust/cred parity now testable |
| `aws_sigv4_sign`, `azure_ad` | ❌ | ✅ (`register.rs:87,93`) | SigV4/Entra now present |
| `aws`/`azure`/`gcp`/`callout`/`token_rate_limit`/`opentelemetry` modules | ❌ | ✅ | new filter surface |
| NeMo allow status | `"passed"` (wrong) | `"success"` (correct) | stub flipped back to `"success"` |
| NeMo SSRF guard | none | `allow_private_endpoint` (default deny) | e2e config sets it `true` for the in-cluster stub |
| `external_metering`, `identity_header_guard` | ❌ | ❌ (open PRs ai#581/#709) | metering **still not merged anywhere** |

**Follow-ups from the bump:** `adapter.rs` gained 9 new `HttpFilterContext`
fields (retry/endpoint-reselection state — advisory in ExtProc, all unset).
NeMo unknown status still `Err`→500, not the IPP fail-closed 503 (C4 gap, §2).

## 1. IPP E2E spec → praxis test coverage

Matrix = 7 specs × {openai, anthropic, azure-openai, bedrock-openai,
vertex-openai} + 1 global = 36 instances. Current praxis coverage: **OpenAI
column only ≈ 7/36 ≈ 19%.**

| IPP spec | praxis test | openai | anthropic | azure | bedrock | vertex-oai |
|---|---|:--:|:--:|:--:|:--:|:--:|
| return 200 | `completions::smoke_200` | ✅ | ❌ | ❌ | ❌ | ❌ |
| OpenAI-format response | `completions::response_has_openai_structure` | ✅ | ❌ | ❌ | ❌ | ❌ |
| tool_calls | `completions::tool_call_passthrough` | ✅ | ❌ | ❌ | ❌ | ❌ |
| image content | `completions::image_content_passthrough` | ✅ | ❌ | ❌ | ❌ | ❌ |
| valid JSON content | *(missing)* | ❌ | ❌ | ❌ | ❌ | ❌ |
| system prompt | `completions::system_prompt_passthrough` | ✅ | ❌ | ❌ | ❌ | ❌ |
| multi-turn | `completions::multi_turn_conversation` | ✅ | ❌ | ❌ | ❌ | ❌ |
| reject invalid API key (global) | `errors::invalid_api_key_rejected` | ✅ | — | — | — | — |

**Gaps:** (a) the 4 non-OpenAI columns = the whole translation surface;
(b) the JSON-content row; (c) no test drives ExternalModel/ExternalProvider CRs
(WS-A reconciler doesn't exist for praxis — routes are hand-written here).

praxis-only tests with **no IPP counterpart** (mechanics, not parity):
`streaming`, `large_body_passthrough`, `direct_*`, `routing_*`, `filters`,
`guardrails_*` (local rule), `ai_guardrails_*`, `observability`,
`trust_boundary`, `token_usage` (ignored).

## 2. Appendix-A must-ports → praxis status

| IPP fix | Guarantees | praxis mechanism | Testable now? | Test |
|---|---|---|---|---|
| #422 strip authorization/x-api-key | client creds never reach 3P | `headers` request_remove (exact) + `credential_inject` (Authorization only, metadata-driven); `x-maas-*` prefix strip still a gap | ⚠️ partial | `credential_strip::*` (exact-name slice) |
| #367 stream-usage-enforcer scope | don't 400 Anthropic streaming | filter **doesn't exist** (praxis-extproc#44) | ❌ | *missing (expect-fail)* |
| #389 usage reassembly across chunks | metering not missed | `external_metering` (open PR) | ❌ pin/PR | *missing* |
| #393 provider errors as metering events | failed reqs metered | `external_metering` | ❌ | *missing* |
| #359 resolve model by body name | single-URL selection | `intelligent_route` overlay | ⚠️ overlay setup | *missing* |
| #425 HTTPRoute match on CR name | header-based BBR identity | BBR + routing | ⚠️ WS-A | partial (`routing::*`) |
| #440 /v1/embeddings shape | embeddings route | translation (C1/C2) | ⚠️ verify filter | *missing* |
| #403 none auth (hub-spoke mTLS) | grid flows | `credential_inject` none-type | ❌ pin | *missing* |
| #438 secret read from event-source | no cred-store race | secret watch (missing) | ❌ | *missing* |
| NeMo fail-closed (unknown → 503) | safe guardrail default | `ai_guardrails` (does `Err`→500) | ✅ | *missing (expect-fail)* |

## 3. Phased validation plan

**P0 — unblock (prereq for anything credential/guardrail-real): ✅ DONE.**
ai pin → v0.3.0 (`359559ab`); praxis-core/filter → registry `0.5.4`;
nemo-stub → `"success"`; e2e provider gets `allow_private_endpoint: true`.
Next: re-run the k8s e2e suite against the bumped build to confirm at runtime.

**P1 — port the IPP suite shape (E1, non-blocked for translation):**
1. Parametrize `completions.rs` over a `providers` table (mirror `e2e_test.go`).
2. Add the missing **valid-JSON-content** spec.
3. Add the **anthropic** column: `anthropic_to_openai` in a route-scoped chain
   → llm-katan anthropic wire format. (Filter exists in the pinned rev.)
4. Add azure/bedrock/vertex-openai columns as their filters/routes land.

**P2 — E3 goldens for the parity-hard items (highest security value):**
1. **#422 credential-strip** — ✅ *exact-name slice scaffolded* (`echo-backend.yaml`
   + `/echo` route + `credential_strip::*`): a header-echo backend proves the
   *negative* — caller `x-api-key` stripped, `Authorization` replaced, neither
   reaches upstream. Verified compiling + kustomize-rendering; **not yet run in
   cluster**. Two follow-ups: (a) upgrade the Authorization path from the
   `headers` overwrite to real `credential_inject` (fail-closed 503, zeroized) —
   needs an `intelligent_route` overlay to write credential metadata; (b)
   `x-maas-*` **prefix** strip is blocked — praxis `headers` `request_remove` is
   exact-name only (same gap as PR #52); file a praxis issue for
   `request_remove_prefix`.
2. **NeMo fail-closed** — provider unreachable/unknown-status → 503 (currently
   500; documents the gap).
3. **Streaming usage** — SSE final chunk has `usage` (expect-fail until
   stream-usage-enforcer lands).
4. **401/403 ordering** (plan D5) — post-auth stamp vs Authorino.

**P3 — real control plane (E1 full, blocked on WS-A):**
run the ported suite through the ai-gateway-controller ExternalModel reconciler
once it exists; until then hand-render HTTPRoutes/overlay.

**P4 — non-functional:** latency parity, redaction checks, production
shadow/mirror against Go IPP.

## Definition of "drop-in ready"

Not one checkbox. All of: EA2 execution flow (reconciler + core chain serving a
MaaS request) · every Appendix-A must-port landed · IPP E2E green against praxis
routes (E1) · E3 goldens pass (streaming, 401/403, fail-closed, redaction).
