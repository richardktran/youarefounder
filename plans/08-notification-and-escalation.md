# Notifications and Escalation to the Founder

## When to interrupt the human

Use escalations sparingly. Suggested triggers:

1. **Decision request** created by CEO/CTO with `blocks` tickets.
2. **Hiring contract** pending approval.
3. **Policy violation** attempt (agent proposed disallowed action)—optional silent fix + comment.
4. **Repeated inference failures** (any `provider_kind`) crossing threshold—surface as company health banner with provider hint.

## Channels (phase 1)

- **In-app inbox** only (decisions + hiring tabs).
- Email/push: defer to post-MVP.

## UX requirements

- **SLA copy** is product tone, not real legal SLA: e.g. “2 decisions unblock 5 tickets.”
- Show **blocked ticket list** on each decision card.
- Allow **defer** / **snooze** later; MVP: only answer or leave open.

## Agent behavior when blocked

- Ticket in `blocked` with link to `decision_id`.
- Scheduler skips blocked tickets until decision `answered`.

## Activity feed semantics

Event types for transparency:

- `decision.opened`, `decision.answered`
- `contract.pending`, `contract.accepted`, `contract.declined`
- `agent.run.failed` with user-safe message

## Anti-spam

- Coalesce multiple hires into one digest **optional**; default immediate for MVP accountability.
- Cap open decisions per company; force agent to **merge questions** if over cap (soft policy in prompt).
