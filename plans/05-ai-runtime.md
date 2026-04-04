# AI Runtime (Provider-agnostic agents, prompts)

## Design principle

Agents are **stateless functions** over a **context pack** plus **tool schema**. Persistence lives in Postgres; the model proposes **declarative actions** the backend validates and applies.

**Inference is provider-agnostic:** the worker speaks a single internal **`ChatCompletionRequest` / `ChatCompletionResponse`** contract; **Ollama, OpenAI, Anthropic, Gemini**, etc. are **adapters** behind one trait ([12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md)).

## Provider abstraction (worker-facing)

- **Registry** maps `AIProfile.provider_kind` → `Arc<dyn InferenceProvider>` (or enum of boxed adapters).
- Each adapter implements:
  - `complete(request)` — normalized messages + model + sampling params → assistant text (and later tool calls).
  - `health_check()` — lightweight call for “Test connection” in onboarding.
- **Phase 1:** only **Ollama** adapter is registered in production builds / enable-list; others are stubbed or feature-gated until implemented.

### Ollama adapter (first implementation)

- HTTP: typically `POST /api/chat` or `POST /api/generate` depending on API version—pin a version in code and document it.
- Maps internal `Message` list to Ollama’s chat format; maps `model_id` to Ollama model name.
- **Streaming:** optional for UX; MVP can use non-streaming in worker for simpler parsing.

### Future adapters (same contract)

- **OpenAI / Azure OpenAI:** Chat Completions or Responses API; map `model_id` to model or deployment name.
- **Anthropic:** Messages API; map system vs user blocks from internal messages.
- **Gemini:** Generative Language API; handle different content parts as needed.

Use **capability flags** per adapter (`json_mode`, streaming, approximate context limit) so the worker can adjust behavior ([12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md)).

## Roles and prompts

Each `role_type` maps to:

- **System prompt** (values, boundaries, formatting)
- **Tool/action schema** (what the agent is allowed to propose)
- **Default model parameters**

Provider choice does not change **role prompts** or **action JSON schema**—only which model answers.

### Co-founder vs CEO vs CTO (behavioral split)

| Role | Emphasis |
|------|----------|
| Co-founder | Product problem framing, roadmap narratives, early discovery tickets |
| CEO | Prioritization across functions, stakeholder narrative, hiring proposals |
| CTO | Technical breakdown, architecture tasks, R&D workspace structure |

Even with similar models, **prompt + allowed actions** create separation.

## Context pack contents (input)

Keep under a token budget with summarization (budgeting may use **per-adapter context hints** later):

1. Company + product one-pagers (trimmed)
2. Workspace charter the ticket belongs to
3. Ticket thread (comments + last N run summaries)
4. Open decisions relevant to assignee
5. List of existing people and specialties (for hiring proposals)
6. **Retrieved repository knowledge (RAG):** top-k chunks from **`knowledge_chunk`** / pgvector search using a query derived from the ticket; include **file path + line range**; require model to treat as authoritative for “what the codebase says” ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md))
7. **Policy block**: “You may not claim external facts without labeling as hypothesis” (reduces false certainty)

## Agent output contract (machine-parseable)

Require JSON (schema versioned), e.g.:

```json
{
  "schema_version": 1,
  "reasoning_summary": "string",
  "actions": [
    { "type": "add_comment", "ticket_id": "uuid", "body": "markdown" },
    { "type": "update_ticket", "ticket_id": "uuid", "fields": { "status": "in_progress" } },
    { "type": "create_ticket", "workspace_id": "uuid", "title": "...", "description": "..." },
    { "type": "create_workspace", "name": "Finance", "purpose": "..." },
    { "type": "open_decision", "title": "...", "context": "...", "blocks": ["ticket_uuid"] },
    { "type": "propose_hire", "display_name": "...", "specialty": "...", "ai_profile_id": "uuid", "rationale": "..." }
  ]
}
```

**Validation:** reject unknown types; cap action count; enforce IDs belong to company.

## Autonomy vs escalation

Default policy:

- Agents may auto-create **child tickets** under their workspace up to depth N.
- Creating a **new workspace** may require CEO/CTO role or a **soft threshold** (e.g. max 5 workspaces) then escalate.
- **Hiring** always produces a `propose_hire` action that becomes a **pending contract**—never direct `Person` insert from model alone.

## Safety and quality controls

- **Second-pass summarizer** (optional): small model summarizes run for activity feed—may use a **cheaper profile** or same profile.
- **Repeat detection:** if model proposes duplicate tickets, merge or ignore with comment.
- **Rate limits** per company and **per provider** (cloud APIs need stricter defaults than local Ollama).

## Testing prompts

- Golden tests with **recorded fixtures** (no network) for parser and validator.
- **Fake `InferenceProvider`** returning deterministic JSON for worker E2E tests.
- Live **Ollama** (and later other vendors) smoke tests behind env flags.
