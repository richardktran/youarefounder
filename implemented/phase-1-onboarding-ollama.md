# Phase 1 — Onboarding + Ollama (completed)

Reference: [plans/10-implementation-phases.md](../plans/10-implementation-phases.md) (Phase 1), [plans/12-ai-provider-extensibility.md](../plans/12-ai-provider-extensibility.md).

## Summary

Onboarding now includes **company, product, and AI setup** (Ollama only in the product). The backend stores **`AIProfile`** rows (`provider_kind`, `model_id`, **`provider_config` JSONB**), exposes **provider metadata** and **test connection**, and creates an **AI co-founder `Person`** linked to that profile. Inference is abstracted behind **`InferenceProvider`** with a **registry** and an **Ollama** adapter. **Git** is intentionally not in this phase (planned for **Phase 9** in [10-implementation-phases.md](../plans/10-implementation-phases.md)—post–MVP).

## Deliverables (as implemented)

### Onboarding wizard (UI + API)

- **`apps/web/src/app/onboarding/page.tsx`** — Steps: **Company → Product → AI setup → Confirm**.
- **AI step** collects: co-founder display name, Ollama base URL, model id; optional **Test connection** before launch.
- **Launch sequence** (client): `POST /companies` (with inline product) → `POST .../ai-profiles` → `POST .../people` (AI agent, `co_founder`, linked to profile) → `POST .../complete-onboarding`.

### `AIProfile` CRUD

- **Migration:** `crates/db/migrations/002_ai_profiles.sql` — `ai_profiles` table + FK from `people.ai_profile_id` → `ai_profiles`.
- **Domain:** `crates/domain/src/ai_profile.rs` — `AiProfile`, `CreateAiProfileInput`, `UpdateAiProfileInput`.
- **DB:** `crates/db/src/ai_profile.rs` — list, get, create, update.
- **API:** `GET/POST /v1/companies/:id/ai-profiles`, `GET/PATCH /v1/companies/:id/ai-profiles/:profile_id` (`crates/api/src/routes/ai_profiles.rs`).

### Enabled providers API (Ollama only)

- **`GET /v1/ai-providers`** — Returns enabled providers and **config field descriptors** for the UI (`crates/api/src/routes/providers.rs`, `ai_providers::ProviderRegistry::enabled_providers()`).
- **Phase 1 product:** only **`ollama`** is registered and validated on create.

### `InferenceProvider` + registry + Ollama adapter

- **`crates/ai-core`** — `Message`, `ChatCompletionRequest` / `ChatCompletionResponse`, `AiError`, **`InferenceProvider`** trait (`async` complete + health check).
- **`crates/ai-providers`** — **`ProviderRegistry`** (`build_adapter` from `provider_kind` + JSON config), **`OllamaAdapter`** (`POST /api/chat` for completion, `GET /api/tags` for health).
- **`AppState`** holds `ProviderRegistry` (`crates/api/src/state.rs`).

### Test connection

- **`POST /v1/ai-providers/test-connection`** — Body: `provider_kind` + `provider_config`. Runs **`health_check()`** on the resolved adapter (no `AIProfile` row required). Used during onboarding before saving a profile.

### Co-founder person linked to profile

- **`POST /v1/companies/:id/people`** — Creates a `Person` with optional `ai_profile_id` (`crates/api/src/routes/people.rs`).
- **DB:** `crates/db/src/person.rs` — `create_person`, `get_person`, plus existing `list_people`.
- **Onboarding** creates an **`ai_agent`** with **`role_type: co_founder`** and sets **`ai_profile_id`** to the new profile.

### Complete-onboarding gate (Phase 1)

- **`POST /v1/companies/:id/complete-onboarding`** (`crates/api/src/routes/companies.rs`) requires:
  1. At least one **product**.
  2. At least one **`ai_agent`** **`Person`** with a non-null **`ai_profile_id`** (AI co-founder configured).

### Frontend API client

- **`apps/web/src/lib/api.ts`** — Types and functions for AI profiles, people, providers, test connection.

## Exit criteria (Phase 1)

| Criterion | How it is met |
|-----------|----------------|
| Fresh user completes onboarding | Wizard + API sequence creates company, product, profile, co-founder person, then completes onboarding. |
| Test connection works against local Ollama | Health check hits Ollama `/api/tags` via registry-built adapter. |
| Adding a vendor later is adapter + enable-list, not a redesign | Trait + registry + JSONB `provider_config`; new kind = new adapter + registry arm + provider list entry. |
| Git + index | **Not in Phase 1** — tracked in Phase 9 per plan. |

## Intentional deferrals

- **Git onboarding, PAT, org, indexing, pgvector** — Phase 9.
- **Cloud LLM providers in the enable-list** — Phase 10+; schema and trait are ready for extension.
