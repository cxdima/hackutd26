# Archived text training candidates — not ready for training

**Training is paused.** These exports preserve the prior prompt/schema and are stale against the current caller fixes. They were not regenerated. Only `frozen_training_exports_match_current_prompt` is explicitly ignored for this reason; the active tests still execute every authored trajectory through the current replay actor and check evidence, outcomes, family isolation and no side effects. The ignored equality check is not a passing compatibility result. Root review and deliberate re-export are required before training.

These are **12 manually authored synthetic candidates**, not a trained model or a measured accuracy result. Seven purpose families were assigned to five training families and two development families in `family-split.json` before any scenario wording was written. `split-freeze.json` records that earlier file hash. All variants and conversation prefixes stay in their assigned family. No existing six-case comparison text, its expectations, or held-out data was used.

`train.jsonl` contains ten rows and `dev.jsonl` two. Each row uses standard `messages`, `tools`, and `parallel_tool_calls: false` fields. Its final assistant message is a native action proposal. The original system/user context comes from the shared runtime builder; an authored assessment, the **actual replay actor's assessment result**, and the authored next action follow it. Earlier caller turns and actual assistant questions are preserved in the current-prefix context. UUIDs are consistently replaced with stable turn aliases; function call IDs match their corresponding tool results.

The earlier input is deliberately not rewritten with the future assessment result. This avoids teaching an assessment while its answer is already present in its input. The dataset carries the full tool schema because each row spans both phases; production still enforces its separate per-request tool masks. These exports have not yet been checked against a particular LoRA trainer's tokenizer or loss masking.

`authored-trajectories.json` contains all source steps. `review-labels.json` holds the independent expected state, stored summary, rationale and **pending root review** status; these fields never enter the model input. `validation-receipts.json` records real in-memory replay-actor results for all 17 caller turns and 34 native tool operations. Only the final prefix of each conversation is selected for the 12 exported rows; earlier questions are validated and supplied as context, not additional training rows. Final targets are ten messages, one question and one ordinary no-message ending, so this small set is not balanced coverage of the task.

Validation checks current evidence, the compact `{turn_id, quote}` fact format, readiness, blocked-question rejection, actual persisted summary, closing state and zero action reservations. Negative controls reject invented evidence and a question after a committed risky assessment. The actor uses an in-memory database, replay mode, no provider keys, no enrolled contacts and disabled outbound controls. No result claims delivery, provider hangup or verified caller identity.

Run offline validation:

```bash
bash scripts/cargo.sh test --test training_candidates
```

After an intentional contract change and review, regenerate only these candidate exports with:

```bash
FIRSTRING_EXPORT_TRAINING_CANDIDATES=1 bash scripts/cargo.sh test --test training_candidates
```

Regeneration changes the candidate files; refresh and review their manifest hashes before use. The separately ignored equality test compares archived exports against current Rust messages and schemas; ordinary test runs retain the real actor trajectory checks. No optimizer, inference request, audio generation, phone call or model update has run. `manifest.json` records provenance and hashes; root must approve the authored gold before any training.
