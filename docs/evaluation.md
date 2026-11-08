# Evaluation ledger

Status: language evaluation and phone rehearsals **not run**. No provider accuracy, latency or call-success claim is supported yet. Local state/protocol test results belong in the verification log, separate from language understanding and real audio.

Local checks: all 91 Rust tests passed in the final local checkpoint, including seven community tests covering publication privacy/idempotency, subscriber receipts, mode isolation, expiry, startup recovery and concurrent publication. Consent tests inject structured observations; they do not test Nemotron's extraction from speech. See [verification](verification.md) for the complete executed checks and their limits.

## Authored cases

- `fixtures/language-development.json`: 24 authored cases, two examples in each of 12 language/context categories.
- `fixtures/language-held-out.json`: 12 authored paraphrases, one per category. Keep out of prompts and development replay. Run after freezing the prompt/policy; if used to tune a fix, report it as a regression set and author a new held-out case.
- `fixtures/protocol-scenarios.json`: 14 race/failure acceptance sequences. These are specifications, not executed test results.
- `fixtures/profiles.json`: fictional household labels and roles; these do not enroll or authorize a telephone endpoint.

All scenarios contain synthetic data. Expected outcomes were authored before model evaluation. They are reviewable expectations, not the evaluated model's own grades. Sponsor data and a sponsor baseline have not been obtained.

The language set distinguishes benign reminders, supplied references, ambiguous codes, explicit authentication requests, paraphrases, negation, quoted warnings, ordinary payment, payment diversion, prompt injection, cross-number tactic retrieval and the benign new-number control. Expected tools allow multiple reasonable responses. `record_assessment` is required for every committed turn before a decision; an offered handoff is never a bridge.

## Execution and scoring

Run language evaluation through the real Nemotron adapter using a non-dialing executor. Never execute the held-out set against a live calling executor. Existing family authorization is absent; protective policy can be tested locally without claiming a family phone rang. Model/API requests are separately measured from phone calls.

For each case record: case ID, build revision, model ID, transport (`model-only`, Relay or Gather), date/time, actual structured observation, tool proposals/results, evidence validity, resulting policy state, latency and reviewer verdict. Review against the authored expectation; do not score from keyword matching or ask the same model to certify itself. Store sanitized outcomes without keys, full telephone numbers, hidden model reasoning or real personal data.

Count `benign`, `risky`, `clarify`, and `reject_instruction` as separate expected classes. An empty/malformed/unassessed response is an error, not a benign prediction. A provider error is a dependency failure; report it separately and in the end-to-end failure denominator. Do not remove failed requests from a success percentage.

| Metric | Calculation | Actual result |
|---|---|---|
| Per-class confusion | Expected class × observed class, with error column | Not run |
| Authentication-request recall | Correctly protective explicit/paraphrased authentication cases ÷ all authentication-positive cases | Not run |
| Benign false-positive rate | Benign cases incorrectly treated as risky ÷ all benign cases | Not run |
| Clarification rate | Cases with a relevant clarification ÷ all completed cases; report ambiguous-code subset separately | Not run |
| Evidence validity | Tool decisions with valid committed-turn quote references ÷ decisions requiring evidence | Not run |
| Stale offer rejection | Rejected obsolete choices ÷ obsolete choices submitted | Not run on phones |
| Unenrolled destinations executed | Count of actual external calls outside the enrolled directory | No live evaluation executed |
| Telephone completion | Fully verified scenario successes ÷ all attempts, including failures | Not run |
| Response latency | Final prompt to model response/audio request; separately measure first audible sound | Not run |

The held-out set has 3 authentication-positive cases, 6 benign cases, 1 ambiguous case, 1 payment-diversion case and 1 injection case. Report counts alongside percentages; this small synthetic set cannot establish production accuracy.

## Result record

```text
Run ID / build revision:
Timestamp / model / transport:
Case IDs attempted:
Passed / failed / dependency failures:
Expected-to-observed confusion counts:
Representative errors and actual consequence:
Evidence validity:
Stale choices submitted / rejected / unauthorized bridge commits:
Calls submitted / answered / acknowledged:
Latency sample count and measured boundaries:
Phone test authorization and endpoints by role only:
```

A local generated sequence test can demonstrate a state invariant under the tested implementation. It cannot establish speech recognition, semantic extraction, carrier routing, first audible latency, summary privacy or two-way audio. Retain those as separate phone gates.
