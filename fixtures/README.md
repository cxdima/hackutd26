# Fictional evaluation fixtures

All people, appointment details and conversations here are synthetic. Role labels do not enroll contacts or authorize calls.

`language-development.json` contains 24 development examples. `language-held-out.json` preserves the original 12 evaluated cases unchanged; it is now regression data. `language-held-out-v2.json` contains 12 new cases frozen before this prompt revision or model execution, with SHA-256 and timestamp in `language-held-out-v2.freeze.json`. Keep expected labels out of model inputs and held-out utterances out of development replay. Evaluation supplies only caller turns, shared runtime context and explicit fictional capabilities. `protocol-scenarios.json` specifies 14 independent race/failure cases; it is not a result ledger. `profiles.json` identifies two fictional households.

Language cases test meaning, direction, quotation and negation. Match tactics by requested action and context; `caller_endpoint_label` is test metadata and must not become an input to tactic retrieval. Cross-number cases require an actual unexpired applied capsule; the benign control must not match merely because that capsule exists.

The expected independent-contact action means a policy-approved proposal resolved against the enrolled directory. With family calling disabled it must remain isolated/pending or report unavailable; a fixture must never override live authorization. Use an inert executor for language evaluation. See `docs/evaluation.md` for scoring and `docs/demo-runbook.md` for the separate real-phone gates.

`language-a-only-development.json` contains three development-only missing-detail/message/risky-shift checks with all calling capabilities false. `results/` retains the actual revised development, original regression and single v2 held-out reports, including failed runs. `model-evaluation-freeze.json` records source hashes before final regression/v2 execution. These results are model-only; completed external actions are always zero.
