# Security and Compliance

## Threat model (realistic for phase 1)

- Founder runs stack locally; risks include **credential leakage** (cloud API keys, optional Ollama keys, **Git PATs with repo/org scope**), **prompt injection** via ticket content, and **accidental exfil** if inference endpoints or plugins are misconfigured. **Git tokens are high impact:** they can create/delete private repositories—store encrypted, document minimal scopes, never log ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md)).
- With **app-managed PostgreSQL**, the **data directory** on disk is sensitive: other OS users or backups must not get world-readable files; follow OS conventions for app data dirs (see [11-embedded-runtime-data.md](./11-embedded-runtime-data.md)).

## Secrets handling

- Store **per-profile** API secrets (Ollama optional, OpenAI/Anthropic/Gemini, …) and **per-company Git credentials** **encrypted** (KMS or libsodium with env-derived key for local dev); isolate by `company_id`.
- Never log raw prompts containing secrets; redact headers.
- API returns **masked** secrets (`****last4`) or boolean configured only.

## Authorization

- **Phase 1:** no session/JWT. Rely on **API bound to loopback** and a **single user per machine** mental model; document that running the API on `0.0.0.0` without auth would be unsafe.
- **Later:** session-bound founder ID; all queries filter by `company_id` ownership.
- Worker uses DB role or internal token **not** derivable from browser (unchanged).

## Prompt injection mitigation (practical, not perfect)

- Separate **instructions** from **user content** with clear delimiters.
- Down-weight ability to override system rules; instruct model to treat ticket text as untrusted data.
- Validate actions against **allowlists** and **ownership** of IDs.

## Data retention

- Define policy for **transcript retention** (30/90 days) and implement purge job later.
- Export/delete account path (GDPR-minded) even for MVP if you ship publicly.

## Dependency and supply chain

- Pin Rust and npm dependencies in CI; audit in release process.

## Compliance copy for users

Product is **simulation / assistance**, not legal or investment advice; surface disclaimers in onboarding footer.
