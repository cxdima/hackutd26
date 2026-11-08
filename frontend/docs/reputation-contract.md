# Reported-number history: contract between console and server

Status (Nov 8, 07:45 CT): **in development on the backend.** The backend terminal's uncommitted work in the main checkout adds the following:

- `server/src/reputation.rs` and `server/src/reputation_http.rs`.
- A `reputation` capability in `/api/state`.
- The routes below.

The Angular console is already wired to that contract. It shows the server's data only when `/api/state` reports `{"id":"reputation","status":"configured"}`. Until then it says "Not connected" and makes no reputation requests. The running server (`.tools/firstring-campaign-transport-only`) predates this work, so live calls show "Not connected" today.

## Principles (unchanged, and kept by the backend implementation)

- **History about a number, not a person.** Caller ID can be spoofed or reassigned. Every response carries `identity_verified: false`.
- **No history means "No reports yet", never "safe".** The console never words it as safety.
- **The browser computes nothing.** The band, label, explanation and optional score come from the server. The console displays them and never derives a percentage.
- **Independence is honest.** Demo-scope reports come from simulated household profiles, so `verified_independent_households` is 0. The console shows both numbers.
- **Correctable and expiring.** A correction request freezes escalation. Levels expire without new reports.
- **Inside FirstRing only.** No carrier actions and no claim that a number can be shut down.

## Routes (all behind the operator bearer token)

| Method | Path | Used by the console |
|---|---|---|
| `GET` | `/api/session/{id}/reputation` | **Yes.** Call page and presentation evidence drawer, only when the capability is configured. |
| `GET` | `/api/reputation?limit=…` | Not yet. Reserved for a list on the Community page. |
| `GET` | `/api/reputation/{number_key}` | Not yet. |
| `POST` | `/api/reputation/reports` `{session_id}` | No. Reports come from the server's own protective flow. |
| `POST` | `/api/reputation/{number_key}/corrections` `{reason, expected_version}` | Not yet. Needs a decision on who may request corrections from the console. |
| `POST` | `/api/reputation/{number_key}/corrections/resolve` | No. Operator-only, out of scope for the household view. |

## `GET /api/session/{id}/reputation` response (validated by `reputationSchema` in `src/app/core/reputation.ts`)

```json
{
  "version": 2,
  "scope": "demo",
  "number_label": "Presented number ending 34",
  "band": "no_reports | reported | corroborated | warning",
  "band_label": "Demo reported; simulated profiles, no verified independence",
  "explanation": "Server-written sentence shown verbatim.",
  "score": null,
  "independent_households": 0,
  "verified_independent_households": 0,
  "demo_household_count": 1,
  "reports": 1,
  "first_reported_at": "RFC 3339 or null",
  "last_reported_at": "RFC 3339 or null",
  "expires_at": "RFC 3339 or null",
  "correction": { "state": "none | requested | upheld | removed" },
  "related_tactic_ids": [],
  "identity_verified": false
}
```

Additive fields are tolerated. A missing `explanation` or any other required field makes the console show "The reputation format changed; nothing is shown rather than a guess." It never falls back to a computed value.

How the console handles each status:
- `404` means no history for this call.
- `503` means reputation is not configured on the server.
- Anything else is shown as an error.

## Receipts on the event stream

These are rendered in the decision trace as plain-language steps:
- `reputation.lookup` (screening context only)
- `reputation.reported`
- `reputation.level_changed`
- `reputation.correction`

## Demo and rehearsal data

`demoReputation()` supplies fictional, demo-scope history for demo calls only, labeled "Example · fictional reports". Fictional numbers are shown only as "Fictional number ending 07/41". No real or judge number is ever reported or displayed as reported.
