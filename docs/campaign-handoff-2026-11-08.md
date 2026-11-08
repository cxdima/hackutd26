# Campaign handoff — 2026-11-08

**Final state09:13UTC:** exact campaign GPU404 and organization inventory0. Reportedcredits$51.12; cumulativepostedspend$8.88; noauto-recharge. App/publicingresshealthy in explicit live/probe mode with modelconfiguredfalse.19/26attemptsused,7remain; all terminal. Attempt19family→secret failed both assessments at512tokens and is not a liveprotectionpass. Bestknownphonebinary and experimentalbinary+DBexport retained; see [final report](campaign-report-2026-11-08.md). Historical preparation details below remain useful for a fresh rehearsal; do not execute the old instance pointer.

This is a prepared handoff, **not a cleanup receipt**. No resource, application configuration, account setting or phone call was changed while preparing it.

## Verified state

- At **09:00 UTC**, exactly one bound Brev H100 remained RUNNING/HEALTHY at the **$3/hour** quote. Billing reported **$53.17 remaining**, auto-recharge off, and $2.60 prior + $4.23 current compute posted. Both guards still target the correct deadlines; the remote timer is active, the container is running, and authenticated model discovery/health returned 200. GPU memory was 74,585 / 81,559MiB. Usage unit/time discrepancies and unposted costs remain unresolved. [Cost/control receipt](evidence/brev-cost-control-2026-11-08-0900.json).
- Final precleanup export at **09:02 UTC** preserves loaded image/model pins, package/EOS configuration, script hashes and 237 retained HTTP-200 chat request records summarized by minute. These are transport counts, **not 237 semantic successes**. Model weights were not copied locally. [Runtime export](evidence/brev-final-runtime-export-2026-11-08.json).
- Work ends **09:09:50 UTC / 03:09:50 Central**. The GPU must be deleted by **09:39:50 UTC / 03:39:50 Central**. Both guards target the exact resource; neither deadline is extended here.
- Hosted NVIDIA inference is **not a verified replacement**. The current-schema assessment timed out at both 3.5s and 10s. An older-schema assessment success at 4,532ms is not a current full-cycle pass. See [current-schema receipt](../fixtures/results/2026-11-08-hosted-current-thinking-budget64-assessment.json).
- Root's **08:27 UTC read-only Twilio inventory** found **zero IncomingPhoneNumbers and zero OutgoingCallerIds**; there is no existing independent Twilio origin. That audit added no number, verification call or purchase.
- **Distinct Photon origin:** readback at 08:32 found two active voice-capable lines following default profile v4, with local MD5/TLS credentials and SRTP matched. Root subsequently verified the distinct-origin phone path in **attempt 16**. This does not create a Twilio verified caller ID. Private configuration receipt: `.tools/photon-second-origin-readonly.json`.
- **Final phone checkpoint supplied by root:** **18/26 attempts consumed, 8 unused**, preserving failed and historical reservations. Attempt 16's complete-message closing gap was 4.873s; matched attempt 17 with Flux `speechTimeout=600` measured 4.553s, versus native request totals 1.722s and 1.760s. The 320ms reduction is **one matched pair**, not a general latency guarantee. Correction attempt 18 passed. Keep these real synthetic phone results separate from human/noisy-room validation and hosted fallback readiness.
- **Provider clarification:** Photon support subsequently confirmed self-calls are permitted, while repeated automated traffic can trigger carrier spam flags. Missing self-call history is not proof that no call connected. Use sparse, reviewed distinct-line canaries for phone evidence and direct GPU tests for iteration; do not resume a rapid automated call loop. This updates the earlier temporary hold rationale without erasing reservations or previous results.

Photon's `GET /v1/projects/{id}/platforms/voice/lines/{resourceId}/profile` returns HTTP 200 **without `profileId` when following the project default**. That is not an unassigned-line blocker; the current default must not be assigned explicitly.

## Cleanup and truthful availability

1. At the work cutoff, stop starting experiments and calls. Settle existing calls and reconcile unknown actions. Preserve the working binary/provenance, database, results, recordings and cached audio.
2. Before GPU deletion, root must deploy an explicit **transport-only** configuration: `VOICE_TRANSPORT=probe`, `DEMO_EXECUTION_MODE=live`, and `TEST_CALLS_AUTHORIZED`, `HANDOFF_CALLS_ENABLED`, `FAMILY_CALLS_ENABLED` all `false`. Select `NEMOTRON_BACKEND=nvidia_hosted` with an empty `NVIDIA_API_KEY` **only in this fallback process's environment**, preserving credentials and deployment files on disk. This marks inference unconfigured.
3. Update the application supervisor's recorded launcher/identity as part of that deployment. The current wrapper forces Brev. Keep ingress/signing unchanged; check local/public health and reported transport without placing a call.
4. The fallback keeps the console, historical evidence and signed fictional greeting/keypad path available while server and ingress remain running. **It does not provide AI conversation.** Gather also requires inference and is not a solution to the hosted-model outage. Historical protocol evidence may still show passed; current `configured=false` must remain clear.
5. Delete the exact bound GPU by the hard deadline. Verify authenticated exact-resource HTTP 404, resource inventory and billing; save a cleanup receipt. A stopped container, active timer or missing list entry is not deletion proof. Keep the first experiment’s unresolved deletion timing intact.

After deletion, the prepared **local, GET-only** helper verifies the fixed campaign identity and writes a sanitized dated receipt:

```bash
python3 .tools/brev-nemotron/verify-cleanup-readonly.py
```

It returns success only when exact-resource HTTP 404 and the organization inventory agree. It never deletes, provisions, calls or runs inference. It reports observed credits/usage without claiming billing has fully settled. The helper lives in the ignored local deployment artifacts; preserve it for this handoff. Its help and syntax checks passed; post-deletion account readback remains pending until root deletes the resource.

The prior campaign supervisor also expires at the hard deadline; leaving the server process running does not imply continued automatic recovery or durable tunnel availability.

## Next rehearsal: fresh deployment, not a warm restart

The selected H100 offering is non-stoppable. Deletion removes its Docker and `/ephemeral` model cache. Local serving/model/container pins, working binary, configuration and audio survive. The 21.56GB checkpoint needs another download unless a separate cache is verified. [Brev lifecycle and persistence](https://docs.nvidia.com/brev/latest/concepts/gpu-instances).

Before allocation, recheck credit, existing resources and the eligible quote. At the observed rate, 60 minutes plus $2 cleanup allowance estimates **$5**, subject to remaining credit and the $60 lifetime ceiling. These commands were not executed; no top-up is planned.

The existing exact-type command is:

```bash
.tools/brev/brev-main create "$rehearsal_name" \
  --type hyperstack_H100 --count 1 --parallel 1 \
  --jupyter=false --detached
```

This is **not a turnkey safe restore command**. First prepare a fresh name, budget/deadline receipt and local guard; bind the returned exact ID without retrying ambiguous creation. Only after the private current-run pointer names the new resource, install the remote guard and pinned runtime:

```bash
python3 .tools/brev-nemotron/bootstrap_current.py --start-runtime
```

Do not run bootstrap against the old pointer or reuse expired identity/deadline files. Verify both new timers, GPU health, API authentication and current-schema native assessment → tool result → next tool before restoring Relay. Refresh the private SSH forward/inference key and the app supervisor's launcher together. [Brev creation reference](https://docs.nvidia.com/brev/latest/cli/instance-creation).

## Guard policy review — proposed only

`watchdog.py::expiry` still subtracts a hard-coded **$50** from `balance_usd` for non-campaign receipts. That historical reserve can reject an otherwise affordable later rehearsal. The campaign branch already accounts for prior spending and the $60 total ceiling; do not mislabel a new rehearsal as the old campaign to bypass the restriction.

The smallest future change is an explicit, immutable budget-policy version for new receipts with `total_budget_usd`, `prior_spend_usd`, `reserve_usd` and `max_spend_usd`. Compute the usable ceiling from the minimum of verified remaining balance minus the explicit reserve, total budget minus prior spending, and the per-experiment cap. Keep the existing $2 cleanup allowance and time bounds. Legacy receipts must retain their original $50 rule; absent or malformed new fields must not silently relax it. Add the new policy fields to the trusted immutable comparison so edits cannot extend the running guard's budget or deadline.

Suggested focused tests: an explicit zero-reserve $5 rehearsal below $50 balance passes; the same legacy receipt remains rejected; insufficient total remaining funds or cleanup allowance fails; negative/nonfinite/bool budget fields fail; and mutation of reserve, prior spending or total ceiling retains the original trusted cleanup deadline. This review did not alter either running guard.
