# Fictional evaluation fixtures

All people, appointment details and conversations here are synthetic. Role labels do not enroll contacts or authorize calls.

`language-development.json` contains 24 examples for development. `language-held-out.json` contains 12 distinct paraphrases reserved for evaluation after prompt/policy freeze. Keep expected labels and held-out utterances out of model prompts and replay initialization. `protocol-scenarios.json` specifies 14 independent race/failure cases; it is not a result ledger. `profiles.json` identifies two fictional households.

Language cases test meaning, direction, quotation and negation. Match tactics by requested action and context; `caller_endpoint_label` is test metadata and must not become an input to tactic retrieval. Cross-number cases require an actual unexpired applied capsule; the benign control must not match merely because that capsule exists.

The expected independent-contact action means a policy-approved proposal resolved against the enrolled directory. With family calling disabled it must remain isolated/pending or report unavailable; a fixture must never override live authorization. Use an inert executor for language evaluation. See `docs/evaluation.md` for scoring and `docs/demo-runbook.md` for the separate real-phone gates.
