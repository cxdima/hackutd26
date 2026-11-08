# FirstRing campaign result — Nov 8

Work: **07:09:50–09:09:50UTC /01:09:50–03:09:50Central**. GPU absence verified09:13:19UTC, within the cleanup window.

| Area | Before | Verified result |
|---|---|---|
| Exact appointment | User heard failure recovery | Real calls saved tomorrow at four without invented AM/PM and closed |
| Correction during goodbye | Wrong time or redundant AM/PM question | Attempt18 saved tomorrow at five, not four; finished goodbye and hung up |
| Normal speech path | Mixed preset/closing voices | Jessica/ElevenLabs v4 plus cached same-voice closing; subjective naturalness still needs listening |
| Reply delay | Matched1000ms finalization:4.873s | Matched600ms:4.553s; one pair, not a general latency guarantee |
| General dialogue | Initial review4/7 | Latest review5/7; human requests and pickup questions remain awkward |
| Family→login request | Development schema contradiction prevented protection | Bounded correction completed protection in development; actual call19 still failed on truncated assessments |

**Actual phone evidence:**15 campaign attempts,19 total reservations,7 remain. Eleven campaign scenarios passed their narrow stated gates, one was partial and three failed; all legs are terminal. Automated-call receipts include44 completed model HTTP observations:42 parsed native responses and2 truncated/rejected responses, not42 successful actions. Recordings and transcripts are retained. Sarah's separate attempt is unused; handoff/family dialing stayed disabled. The new distinct Photon origin worked; repetitive automated traffic remains a carrier risk, so direct inference was the main iteration method.

**NVIDIA:** actual Nemotron3.5Lightning30B-A3B NVFP4 weights ran on the Brev H100 with pinned vLLM0.27.1. Native assessment/result/action cycles were observed. The final six-case comparison still failed warning-summary persistence and produced an unhelpful delivery question. The final phone test received both family and login-secret turns, but both assessments hit512tokens (2305/2458ms); neither was accepted and no incident was saved. Live turn IDs/profile/context differ from development and need exact-context reproduction before another promotion. No weight or adapter training occurred; frozen held-out sets were untouched.

**Checks:**324Rusttests passed,2explicitignored; fmt, Clippy and build passed.53frontendtests,lint/typecheck/build and the focused saved-message browser check passed. These checks do not override failed phone evidence.

**Cost/resource state:** prior$2.60 + currentposted$6.28 = **$8.88**; reportedbalance **$51.12**, auto-rechargeoff. Billing may still settle. The exactGPU returns404 and the account lists0instances. Modelweights were not exported; pinned deployment recipes and local binaries/database/audio were preserved.

**What is available now:** console, historical evidence and signed greeting/keypad transport probe. **Live AI is off after scheduled GPU cleanup**, and hosted inference is not a verified fallback. The last successful phone binary is `.tools/firstring-campaign-flux-600`; the later experimental binary is `.tools/firstring-campaign-native-repair`. Both hashes and the database snapshot are recorded locally.

**Best demonstration:** show the recorded appointment→time-correction call alongside transcript, accepted understanding and actual saved result, then the earlier recorded clinic→login protective call. Label playback as recorded. For a new live demo, restore a budget-checked Nemotron instance and reproduce the actual family-call context before claiming general-purpose reliability. The next development task is precise live-context token/termination diagnosis, not another broad dataset or training project.
