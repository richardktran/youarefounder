# You Are Founder — Planning Documents

This folder contains the end-to-end plan for a product where a **customer acts as the human founder** of a virtual company. AI roles run autonomously using configurable models—**phase 1 ships Ollama only**, but the **architecture is multi-provider** (OpenAI, Anthropic, Gemini, …) via adapters; see [12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md).

## How to read these docs

| Order | Document | Purpose |
|-------|----------|---------|
| 1 | [00-vision-and-scope.md](./00-vision-and-scope.md) | Product intent, personas, in/out of scope |
| 2 | [01-system-architecture.md](./01-system-architecture.md) | Rust backend + Next.js frontend, deployment shape |
| 3 | [02-domain-model.md](./02-domain-model.md) | Companies, people, workspaces, tickets, contracts |
| 4 | [03-backend-rust.md](./03-backend-rust.md) | Services, APIs, persistence, background jobs |
| 5 | [04-frontend-next.md](./04-frontend-next.md) | Routes, UX flows, state, components |
| 6 | [05-ai-runtime.md](./05-ai-runtime.md) | Provider-agnostic runtime, agent loop, prompts |
| 7 | [06-workspaces-and-tickets.md](./06-workspaces-and-tickets.md) | Jira-like structure and automation rules |
| 8 | [07-hiring-and-approvals.md](./07-hiring-and-approvals.md) | Proposals, contracts, accept/decline |
| 9 | [08-notification-and-escalation.md](./08-notification-and-escalation.md) | When agents pause for the founder |
| 10 | [09-security-and-compliance.md](./09-security-and-compliance.md) | Secrets, tenancy, abuse |
| 11 | [10-implementation-phases.md](./10-implementation-phases.md) | MVP → v1 milestones and risks |
| 12 | [11-embedded-runtime-data.md](./11-embedded-runtime-data.md) | App-managed Postgres, PG-backed queue, no Redis |
| 13 | [12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md) | Multi-provider design (Ollama, OpenAI, Claude, Gemini) |
| 14 | [13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md) | Git credentials in onboarding, private repos, code/knowledge RAG |
| 15 | [14-implementation-phases-design.md](./14-implementation-phases-design.md) | Phase dependencies, gates, parallel tracks, effort bands |

Start with vision and architecture, then dive into domain + AI runtime before implementation phases. For **zero pre-setup installs**, read [11-embedded-runtime-data.md](./11-embedded-runtime-data.md) with [01-system-architecture.md](./01-system-architecture.md). For **adding cloud LLMs later**, read [12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md) alongside [05-ai-runtime.md](./05-ai-runtime.md). For **Git org repos + indexed codebase**, read [13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md). For **how phases connect and when to start each**, read [14-implementation-phases-design.md](./14-implementation-phases-design.md) with [10-implementation-phases.md](./10-implementation-phases.md).
