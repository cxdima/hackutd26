---
version: 1
slug: "frontend-src-app-console-console-shell-ts"
primary_target: "frontend/src/app/console/console-shell.ts"
related_targets: []
---

## Scope

Authenticated dashboard at `/console` (Overview, Calls, Community, Settings) plus progressive-disclosure operator detail. Visitor mode: **Operate**. Users: a family member/caregiver reading what happened on calls, and an operator who needs full evidence. Inherits the landing page's world (same tokens, caption bands, status bug, rundown) with expression restrained to precise details.

Tasks: see service state truthfully; read the most important recent call; scan and filter calls by outcome; open a call and read summary, caller request, outcome, closed-caption transcript and the full decision trace (what the AI heard, how it classified it, which rule allowed or held it, what it proposed, what Rust executed, what was spoken, model latency and tokens); review shared tactics; edit the call profile for new calls; access raw events, provider readiness and test controls.

States: no token (connect form), connecting, connected LIVE, REPLAY server, explicitly selected DEMO fixtures, stale/paused stream, signal lost, empty calls, long transcripts, failed profile save. Proposed actions never display as completed. No Ringo animation during urgent decisions; the rest pose appears only in calm empty states.

## Direction contract

THESIS: The dashboard is the station's rundown and caption archive: every call is a segment with a plain-language bulletin and an exposed decision spine you can open to see everything the AI saw and did. It refuses the generic KPI-card dashboard and invented "protected" scores.

OWN-WORLD: Same tokens as the landing page. Charcoal top bar with the station bug (LIVE / REPLAY / DEMO / SIGNAL LOST) at its right; rundown table rows on warm white with tabular times; outcome chips always icon + words; caption-band transcripts; decision spine as a vertical rail with labeled stations.

STORY: Open, see whether the service is answering, read the latest bulletin, open a call, follow the spine from caller words to outcome, adjust the greeting for new calls.

FIRST VIEWPORT: Top bar (wordmark, nav, status bug, lock). Overview: service state line, latest important call bulletin as the largest element, recent rundown below, a quiet community update aside.

FORM: Inherits the landing direction (seed key e8bca7fd) translated to Operate: rundown sheet, captions, lower-third bulletins, station bug.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
