---
version: 1
slug: "frontend-src-app-console-present-present-page-ts"
primary_target: "frontend/src/app/console/present/present-page.ts"
related_targets:
  - "frontend/src/app/console/present/community-story.ts"
  - "frontend/src/app/console/present/stage-rail.ts"
---

## Scope

The projected screen at `/console/present`, inside the authenticated console, shown while a judge phones FirstRing. Visitor mode: **Experience** for the room (the live call is the artifact), with **Operate** controls tucked to the edges for the presenter: pin or follow a call, reset the display, open the evidence drawer. It must also work on the presenter's phone at 390px.

Readable from a few feet away, the screen shows three things: what the caller is asking for right now, FirstRing's current assessment or next step, and the actual outcome. A rundown of evidence-backed stages connects them ("Caller speaking" → "Checking the request" → "Asking a follow-up" → "Message saved" / "Permission requested" / "Suspicious request blocked"). The branch moment, when the caller switches from an ordinary appointment to asking for a code, must be visually obvious. A second scene, "Sarah spotted it. Maya was ready.", shows the community story with two households and one shared warning, completing only the stages the receipts prove.

States: locked, waiting for a call (connected), service unavailable or signal lost (distinct from waiting), following newest call, pinned call, a new call arriving while pinned, a historical call that must not take over, DEMO rehearsal playback (labeled), handoff stages, model not configured, reduced motion.

## Direction contract

THESIS: A local station's live broadcast of one phone call. The caller's words arrive as a giant closed caption, FirstRing's assessment as the lower third, and the outcome as the single colored graphic on screen. It refuses a dashboard-on-a-projector: no card grids, no metrics, no JSON on the main stage.

OWN-WORLD: Same tokens. The full-bleed charcoal studio is the only charcoal-ground surface in the console, so paper text and lime tally carry from a distance. The `>> CALLER` caption is set at display scale. The lower third has a lime tally strip, or magenta-ink when a request is suspected. The rundown rail of stages uses slate caps. The station bug (LIVE / DEMO / SIGNAL LOST) and a quiet "Decision model: NVIDIA Nemotron" credit sit in the corners.

STORY: Waiting (the number to call, large, with an optional tel: QR) → caption appears as the caller speaks → lower third shows "Checking the request" → follow-up question → the request changes and the earlier turn stays visible as the struck-through "before" → the outcome graphic lands. Evidence lives behind one drawer button.

FIRST VIEWPORT: The waiting state is the cover: the phone number at display scale, "Call FirstRing", with Ringo's rest pose small and calm and the connection bug top-right. A live call replaces it with caption, lower third and outcome.

MOTION: GSAP only on evidence arriving. A new caption line wipes in left to right (≤400ms), the lower third tally strip slides, and the outcome graphic scales from 96% with a single ring pulse. Reduced motion swaps to instant changes. Nothing loops while a call is live.

FINISH: finish review with one verdict pass, the detector run, and DESIGN.md updated with the presentation components.
