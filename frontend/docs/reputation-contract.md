# Reported-number reputation: proposed backend contract

Status: **not implemented.** The landing page and the dashboard show reputation only as a labeled concept. This document describes what the Rust server would need so the Community page can show real data. It does not authorize building it.

## Principles

- **Match tactics, not people.** A number warning never claims who placed a call. Caller ID can be spoofed or reassigned.
- **Independent evidence.** Corroboration counts distinct participating households, not reports.
- **Minimized data.** No names, transcripts, recordings or quotes. The number is stored as a keyed hash (HMAC with a server secret) plus the last two digits, for display.
- **Expiry by default.** Every level decays unless renewed by new, independent reports.
- **Correctable.** A correction request freezes escalation, and an accepted correction removes the warning everywhere.
- **Scope.** Effects stay inside participating FirstRing services. There are no carrier actions and no shutdown claims.

## Data

```text
number_reputation
  number_key        TEXT PRIMARY KEY   -- HMAC-SHA256(server_secret, E.164)
  display_suffix    TEXT               -- e.g. "34"
  level             TEXT               -- none | reported | corroborated | warning | screened
  household_count   INTEGER            -- distinct reporting households (dedupe key below)
  tactic_ids        TEXT[]             -- capsule ids that support the reports
  first_reported_at TIMESTAMP
  last_reported_at  TIMESTAMP
  expires_at        TIMESTAMP          -- recomputed on every accepted report
  correction        TEXT               -- none | requested | upheld | removed
  version           INTEGER            -- monotonic, for idempotent clients

number_report
  report_id         TEXT PRIMARY KEY
  number_key        TEXT
  household_id      TEXT
  session_id        TEXT               -- the blocked call that produced it
  tactic_id         TEXT NULL
  created_at        TIMESTAMP
  UNIQUE(number_key, household_id)     -- one household counts once
  UNIQUE(session_id)                   -- one call reports once
```

Suggested thresholds (tunable): one household → `reported` (private to that household); two independent households within 7 days → `corroborated`; three or more → `warning`. `screened` (stronger screening) requires `warning` plus a matching shared tactic capsule. Levels expire after 14 days without new independent reports. A correction in `requested` state holds the level where it is.

## API (bearer-authenticated, same as `/api/*`)

| Method | Path | Body / result |
|---|---|---|
| `GET` | `/api/reputation?limit=50` | `{ items: [{ number_key, display_suffix, level, household_count, expires_at, correction, version, tactic_ids }] }` |
| `GET` | `/api/reputation/:number_key` | One item plus `reports: [{ household_id, created_at, tactic_id }]` (household-scoped for non-operators) |
| `POST` | `/api/reputation/reports` | `{ session_id }` → `202 { report_id, level, version }`. The server derives the number from the session, so a client never sends raw numbers. `409` if it is a duplicate. |
| `POST` | `/api/reputation/:number_key/corrections` | `{ reason: "mistaken" \| "spoofed" \| "legitimate_business", note?: string ≤ 280 }` → `202 { correction: "requested", version }` |
| `POST` | `/api/reputation/:number_key/corrections/resolve` | operator only, `{ outcome: "upheld" \| "removed" }` |

Events for the existing SSE stream (same envelope as other receipts):

- `reputation.reported` `{ number_key, level, version, household_count }`
- `reputation.level_changed` `{ number_key, from, to, version, reason: "corroborated" | "expired" | "corrected" }`
- `reputation.correction` `{ number_key, state, version }`

Duplicate and out-of-order events must not regress `version`. This is the same rule the console already applies to phone receipts.

## Frontend wiring once available

1. Add Zod schemas next to `src/app/core/schemas.ts` and a loader in `ConsoleStore`.
2. Replace the "Not connected" panel in `src/app/console/community-page.ts` with the list, and add a "Report a mistake" form posting to `/corrections`.
3. In a call's detail, show the number's level only as "reported number", never as an identity.
4. Keep the landing page's concept section labeled until real data exists in production.

## If built on AWS instead

The Terraform scaffold in `infra/terraform` can host this as API Gateway + Lambda + DynamoDB, with the Rust server calling it server-side. The browser contract above stays the same, with the Rust server proxying it so tokens never reach the browser.
