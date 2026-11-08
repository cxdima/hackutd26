# Evaluation ledger

Status: **first real hosted language evaluation FAILED** on Nov 7 at 20:05 Central. All 24 NVIDIA requests completed; 3/12 cases passed the available composite rubric. Phone rehearsals remain unrun. These inputs are synthetic, but provider responses and latencies are real. [Complete original results](evidence/nvidia-held-out-2026-11-07.json) and [verification](verification.md) retain the failures. No prompt or frozen fixture was changed after scoring.

Local checks: all 113 Rust tests passed in the current checkpoint, including seven community tests covering publication privacy/idempotency, subscriber receipts, mode isolation, expiry, startup recovery and concurrent publication. Consent tests inject structured observations; they do not test Nemotron's extraction from speech. See [verification](verification.md) for the complete executed checks and their limits.

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
| Per-class confusion | Expected class × observed class, with error column | benign→benign 2, benign→clarify 4; clarify→risky 1; risky→risky 4; injection→clarify 1 (rejection class unscored); errors 0 |
| Authentication-request recall | Correct authentication-risk flags ÷ authentication-positive cases |3/3 flags; all 3 nevertheless chose a disallowed protective next step, so this is not completed protection |
| Benign false-positive rate | Benign cases classified risky ÷ all benign cases |0/6; 4/6 instead received unnecessary clarification |
| Clarification rate | Cases proposing clarification ÷ completed cases; relevance separately reviewed |12/12 ask_caller; required subset 1/1. Relevance unscored; all 4 risky cases fail next-tool allowlist |
| Evidence validity | Initial assessments with valid committed-turn quote references ÷ attempted cases |12/12; does not establish correct meaning |
| Stale offer rejection | Rejected obsolete choices ÷ obsolete choices submitted | Not run on phones |
| Unenrolled destinations executed | Count of actual external calls outside the enrolled directory |0 calls/actions executed; model-only evaluation |
| Telephone completion | Fully verified scenario successes ÷ all attempts, including failures | Not run |
| Response latency | HTTP request through response/tool parse; audio separately |24 samples, 629–1979ms, median 1049.5ms. No audio measurement |

The composite 3/12 includes the injection case with its classification explicitly unscored; it is not three fully verified safety scenarios. Classification was correct in 6/11 scored cases; next tool allowed in 8/12. All failures are retained, including four benign→clarify errors, ambiguous→risky, and four risky calls that stayed at clarification. Any prompt tuning based on these failures makes these cases regression data; author a new frozen set before another held-out claim.

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
