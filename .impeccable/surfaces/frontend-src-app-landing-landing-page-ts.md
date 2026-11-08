---
version: 1
slug: "frontend-src-app-landing-landing-page-ts"
primary_target: "frontend/src/app/landing/landing-page.ts"
related_targets: []
---

## Scope

Public landing page at `/` of the Angular app in `frontend/`. Visitor mode: **Persuade**. Audience: older adults (65+) and the family members or caregivers who set FirstRing up. Job: understand in ten seconds what FirstRing does, then see a screened call, the community idea, and the privacy answer. Actions: "See how it works" (in-page), "Explore the demo" (interactive example and explicitly labeled demo dashboard), "Open dashboard".

Constraints: no fabricated proof, numbers, testimonials, partnerships; simulated examples labeled; respectful, non-fear language; 18–20px body; reduced-motion parity; GSAP limited to hero entrance, hero layer depth, the personal→community ring transition and the staged screening explanation.

## Direction contract

THESIS: FirstRing is a calm local station that goes on air before you pick up: it answers, captions what the caller wants, and posts a plain-language bulletin. The page refuses the category default of a shield icon, centered hero and six feature cards; every proof is shown as a broadcast caption or bulletin of a real-shaped call.

OWN-WORLD: Warm-white studio ground (#F7F7F2). Charcoal (#171C18) caption bands and lower thirds with warm-white text and a lime (#76B900) tally tab; dark text on lime. Magenta (#E20074, deepened for text) marks a suspected request, always paired with an icon and the words "Suspected". Closed-caption transcripts with the real ">>" speaker-change convention. Status bugs (EXAMPLE, LIVE, REPLAY, DEMO, SIGNAL LOST) used once per view. Manrope from 400 to 800, hierarchy by scale contrast, tabular figures for times. Concentric signal rings as the only ornament.

STORY: Visitor sees Ringo and the headline, reads one example caption, understands that ordinary messages get through and code requests get stopped, scrolls the single axis from "your phone" outward to "your neighborhood", reads the voice-privacy answer, and opens the demo.

FIRST VIEWPORT: Desktop: left 6/12 columns, headline "Before you answer, FirstRing checks." at about 5.5rem/800, supporting copy at 1.25rem, primary lime action and secondary charcoal-outline action. Right 6/12 columns plus right bleed: Ringo hero loop (poster still) at monumental scale, SVG signal rings radiating from his ring, and a charcoal lower-third caption of a screened call overlapping Ringo's lower left with a small "Example" bug. Mobile: copy and actions first, then a Ringo crop with the lower third beneath.

FORM: Local TV station graphics (weather desk, lower thirds, closed captions, the rundown), candidate 7 of 7 on my grounded list; seed key e8bca7fd. Raises kept from declined challengers: drum-machine step row → the staged screening explanation always shows where "now" is; variable-font specimen → hierarchy by scale contrast alone, no eyebrows; Versailles vistas → one reading axis from phone to neighborhood carried by the expanding ring; Coptic binding → the decision chain drawn as an exposed spine (evidence → check → rule → outcome); Taller Gráfica linocut → Ringo at monumental scale cropped by the frame. Competitive challenger kept as alternate: Brodovitch spread (caller line poses, FirstRing answers across a central seam in the interactive example).

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Unresolved

- Real voice preview samples are not exposed by the API; only labeled text examples ship.
- Number reputation is a labeled concept; backend contract documented in `frontend/docs/reputation-contract.md`.
