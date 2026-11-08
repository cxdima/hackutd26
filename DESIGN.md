---
name: FirstRing
description: A calm local station that goes on air before you pick up — captions, lower thirds and a plain-language bulletin for every unfamiliar call.
colors:
  paper: "#f7f7f2"
  paper-2: "#efefe7"
  paper-3: "#e3e4da"
  rule: "#d2d4c8"
  ink: "#171c18"
  ink-2: "#222923"
  ink-3: "#303832"
  muted: "#59615b"
  muted-on-ink: "#b8bfb6"
  lime: "#76b900"
  lime-hover: "#86cc0b"
  lime-ink: "#3a6300"
  lime-wash: "#e6f1cf"
  magenta: "#e20074"
  magenta-ink: "#b3005c"
  magenta-on-ink: "#ff74b5"
  magenta-wash: "#fbe4ef"
  water: "#b9e2d4"
  amber-ink: "#7a4a00"
  amber-wash: "#fbefd6"
typography:
  display:
    fontFamily: "'Manrope Variable', 'Manrope', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "clamp(2.75rem, 1.1rem + 4.1vw, 5.25rem)"
    fontWeight: 800
    lineHeight: 0.98
    letterSpacing: "-0.035em"
  headline:
    fontFamily: "'Manrope Variable', 'Manrope', ui-sans-serif, system-ui, sans-serif"
    fontSize: "clamp(2.25rem, 1.55rem + 2.6vw, 3.9rem)"
    fontWeight: 800
    lineHeight: 1.03
    letterSpacing: "-0.03em"
  page-title:
    fontFamily: "'Manrope Variable', 'Manrope', ui-sans-serif, system-ui, sans-serif"
    fontSize: "clamp(2rem, 1.6rem + 1.6vw, 2.9rem)"
    fontWeight: 800
    lineHeight: 1.05
    letterSpacing: "-0.03em"
  title:
    fontFamily: "'Manrope Variable', 'Manrope', ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.75rem"
    fontWeight: 800
    lineHeight: 1.25
    letterSpacing: "-0.02em"
  lede:
    fontFamily: "'Manrope Variable', 'Manrope', ui-sans-serif, system-ui, sans-serif"
    fontSize: "clamp(1.2rem, 1.1rem + 0.35vw, 1.4rem)"
    fontWeight: 400
    lineHeight: 1.55
  body:
    fontFamily: "'Manrope Variable', 'Manrope', ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.1875rem"
    fontWeight: 400
    lineHeight: 1.65
    fontFeature: "'ss01' on"
  body-ui:
    fontFamily: "'Manrope Variable', 'Manrope', ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.0625rem"
    fontWeight: 400
    lineHeight: 1.6
    fontFeature: "'ss01' on"
  slate:
    fontFamily: "'Manrope Variable', 'Manrope', ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 800
    lineHeight: 1.2
    letterSpacing: "0.08em"
rounded:
  bug: "0.5rem"
  row: "0.75rem"
  band: "0.875rem"
  field: "0.875rem"
  card: "1.25rem"
  pill: "999px"
spacing:
  gutter-sm: "1rem"
  gutter-md: "1.5rem"
  gutter-lg: "2.5rem"
  section: "5rem"
  section-lg: "7rem"
components:
  button-primary:
    backgroundColor: "{colors.lime}"
    textColor: "{colors.ink}"
    rounded: "{rounded.pill}"
    padding: "0.8rem 1.4rem"
    height: "3.25rem"
  button-primary-hover:
    backgroundColor: "{colors.lime-hover}"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.pill}"
    padding: "0.8rem 1.4rem"
    height: "3.25rem"
  button-secondary-hover:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
  button-ink:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.pill}"
    padding: "0.8rem 1.4rem"
    height: "3.25rem"
  button-ink-hover:
    backgroundColor: "{colors.ink-3}"
  button-quiet:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.pill}"
    padding: "0.55rem 1rem"
    height: "2.75rem"
  bug-live:
    backgroundColor: "{colors.lime}"
    textColor: "{colors.ink}"
    typography: "{typography.slate}"
    rounded: "{rounded.bug}"
    padding: "0.3rem 0.75rem 0.3rem 0.55rem"
  bug-outline:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    typography: "{typography.slate}"
    rounded: "{rounded.bug}"
    padding: "0.3rem 0.75rem 0.3rem 0.55rem"
  bug-lost:
    backgroundColor: "{colors.amber-wash}"
    textColor: "{colors.amber-ink}"
    typography: "{typography.slate}"
    rounded: "{rounded.bug}"
    padding: "0.3rem 0.75rem 0.3rem 0.55rem"
  caption-band:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.band}"
    padding: "1rem"
  chip-saved:
    backgroundColor: "{colors.lime-wash}"
    textColor: "{colors.lime-ink}"
    rounded: "{rounded.bug}"
    padding: "0.25rem 0.65rem 0.25rem 0.5rem"
  chip-blocked:
    backgroundColor: "{colors.magenta-wash}"
    textColor: "{colors.magenta-ink}"
    rounded: "{rounded.bug}"
    padding: "0.25rem 0.65rem 0.25rem 0.5rem"
  chip-review:
    backgroundColor: "{colors.amber-wash}"
    textColor: "{colors.amber-ink}"
    rounded: "{rounded.bug}"
    padding: "0.25rem 0.65rem 0.25rem 0.5rem"
  chip-none:
    backgroundColor: "{colors.paper-3}"
    textColor: "{colors.ink-3}"
    rounded: "{rounded.bug}"
    padding: "0.25rem 0.65rem 0.25rem 0.5rem"
  chip-live:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.lime}"
    rounded: "{rounded.bug}"
    padding: "0.25rem 0.65rem 0.25rem 0.5rem"
  field:
    backgroundColor: "#ffffff"
    textColor: "{colors.ink}"
    rounded: "{rounded.field}"
    padding: "0.7rem 0.95rem"
    height: "3.25rem"
---

# Design System: FirstRing

## Overview

**Creative North Star: "The Calm Local Station"**

FirstRing looks like a small, trustworthy local TV station that goes on air before you pick up the phone. The page is a warm-white studio ground; information arrives as broadcast graphics: charcoal closed-caption bands with the real `>>` speaker-change convention, lower thirds with a lime tally tab, a single station bug that says where the data comes from (Live, Replay, Demo, Signal lost), and a rundown sheet of calls with tabular times. The dashboard is the same station's rundown and caption archive, with the decision trace drawn as an exposed vertical spine of labeled stations.

Hierarchy comes from scale contrast in one family (Manrope, 400 to 800), not from decoration. The only ornament is concentric signal arcs radiating from Ringo's lime ring, with one contiguous magenta segment. Density is generous: older adults are a primary audience, so body copy runs 17 to 19px, controls are 44 to 56px tall, and every state is carried by an icon plus words, never colour alone. Surfaces are flat and opaque; the few shadows are soft lifts under floating broadcast graphics.

Confirmed rejections from the build: no shield-icon hero, no six-feature card grid, no KPI-card dashboard or invented scores, no glass or backdrop blur, no lime-to-magenta gradient, no eyebrows or kickers above headings.

**Key Characteristics:**
- Warm-white studio ground, charcoal broadcast graphics, lime as the on-air/tally colour.
- Closed-caption bands with `>> SPEAKER` labels for every transcript or quoted caller line.
- One station bug per view naming the data source.
- Rundown rows and ruled lists instead of nested cards.
- Concentric signal arcs as the sole ornament; Ringo at monumental scale, cropped by the frame.
- Opaque, flat surfaces; soft lift only under floating lower thirds and the operator drawer.

## Colors

A brief-pinned palette of warm white, charcoal, NVIDIA-adjacent lime and telecom magenta, with each accent split into a brand fill, a text-safe ink and a pale wash.

### Primary
- **Tally Lime** (lime): the on-air colour. Primary button fill, LIVE bug, lower-third tally tabs, How-it-works step rings, signal arcs, text selection. Dark ink text on lime only.
- **Tally Lime Bright** (lime-hover): primary button hover fill.
- **Field Lime Ink** (lime-ink): lime-family text on paper (Done/saved chips, ok spine nodes).
- **Lime Wash** (lime-wash): background of saved/Done chips and ok spine nodes.

### Secondary
- **Signal Magenta** (magenta): brand fill only — the single magenta arc segment in the hero rings and the wordmark detail. Never body text.
- **Suspect Magenta Ink** (magenta-ink): the "Suspected" voice. Blocked chips, protect spine nodes, the suspected-tactic strip, error borders, error banners, text caret.
- **Magenta on Ink** (magenta-on-ink): magenta text on charcoal grounds.
- **Magenta Wash** (magenta-wash): background for blocked chips, tactic matches and error notices.

### Tertiary
- **Caution Amber Ink** (amber-ink) and **Amber Wash** (amber-wash): needs-review chips, SIGNAL LOST bug, paused-stream banner, "Proposed · not executed" tags.
- **Ripple Water** (water): the ground of the community water/ripple illustration panel only.

### Neutral
- **Studio Paper** (paper): page ground and light surfaces.
- **Paper 2** (paper-2): row hover, inset panels, demo-mode banner.
- **Paper 3** (paper-3): selected row, neutral chips, internal hairlines, spine rail.
- **Rule** (rule): field borders, header border on scroll, quiet button outline.
- **Broadcast Charcoal** (ink): text, caption bands, console top bar, ink buttons, focus outline.
- **Ink 2** (ink-2): landing footer ground.
- **Ink 3** (ink-3): secondary text on paper, ink-button hover, neutral proposal outlines.
- **Muted** (muted): help text, metadata, slate labels on paper.
- **Muted on Ink** (muted-on-ink): caption speaker labels and secondary text on charcoal.

### Named Rules
**The Ink-Variant Rule.** Lime and magenta fills keep their brand value; any lime or magenta *text* on paper uses the `-ink` variant, and on charcoal the `-on-ink` variant. Dark text on lime, never white.

**The Lime Means Done Rule.** Lime washes and lime-ink text mark things that actually happened (saved, Done, accepted). Proposals the model merely suggested are neutral ink-outline ("Proposed · executed below") or amber ("Proposed · not executed"), never lime.

**The Suspected-Needs-Words Rule.** Magenta always travels with an icon and words ("Suspected", "blocked"); it is never the only signal.

## Typography

**Display Font:** Manrope Variable (with Manrope, ui-sans-serif, system-ui)
**Body Font:** Manrope Variable (same family)

**Character:** One humanist-geometric family carrying everything; extrabold (800) headings with tight negative tracking against regular-weight, generously leaded body. Stylistic set `ss01` is on globally; times and counts use tabular figures.

### Hierarchy
- **Display** (800, clamp 2.75–5.25rem, 0.98): the landing hero headline only.
- **Headline** (800, clamp 2.25–3.9rem, 1.03): landing section headings.
- **Page title** (800, clamp 2–2.9rem, 1.05): console page h1s; the overview h1 and connect panel run slightly larger clamps.
- **Title** (800, 1.75rem, 1.25, -0.02em): step and sub-section h3s; bulletin headlines use a 1.4–1.875rem clamp.
- **Lede** (400, clamp 1.2–1.4rem, 1.55): hero supporting copy, capped near 33ch.
- **Body** (400, 1.1875rem, 1.65): landing prose, 60–64ch measure.
- **Body UI** (400–600, 1.0625rem, 1.6): console text, rows, notices; metadata at 0.95–1rem.
- **Slate** (800, 0.8125rem, 0.08em, uppercase): broadcast slates only — bugs, lower-third tally tabs, caption speaker labels, decision-spine station names, small standalone label headings in the call detail.

### Named Rules
**The Scale-Not-Eyebrow Rule.** Hierarchy is carried by scale contrast alone. Nothing sits above a heading to announce it; slate caps are broadcast graphics or self-standing labels, never a kicker over a larger title.

**The Big-Enough Rule.** No reading text below 0.95rem; landing body stays at 19px.

## Layout

Landing content sits in a 12-column grid inside an 86rem container (console: 90rem) with 1rem / 1.5rem / 2.5rem side padding at base / sm / lg. Sections breathe at 5rem vertical padding, 7rem from sm. Copy usually takes 5 columns with a 6–7 column companion offset by one column.

The hero splits copy left (6 of 12) from a 16:9 art stage anchored right that bleeds off the right edge by 12vw, sized to min(60vw, viewport height minus header, 64rem). The lower-third caption overlaps the art's lower left. Below lg (1024px) the art becomes a square crop under the copy, feathered on all four edges, with the lower third pulled up over its bottom edge.

Navigation collapses below lg on both surfaces: landing to a Menu button with a full-width list; console to a five-up icon tab row under a slim top bar. From lg the console is a 16.5rem rail plus a content column (max 96rem, 3rem side padding). Tables that cannot fit a phone switch to stacked rundown rows below md instead of scrolling the page sideways.

The rundown is the default list shape: rows of time · outcome chip · bulletin with a chevron, hairline-separated or hover-tinted, never boxed individually. Progressions and comparisons render as ruled rows (top rule 2px ink, dl grids) rather than cards inside cards.

## Elevation & Depth

Flat, opaque, tonal by default: depth comes from paper / paper-2 / paper-3 steps, 2px ink borders and charcoal bands. Shadows appear only under elements that float over other content as broadcast graphics or drawers. No backdrop blur, no translucent glass; overlays are fully opaque.

### Shadow Vocabulary
- **Lift** (`box-shadow: 0 18px 40px -22px rgb(23 28 24 / 0.45), 0 2px 6px -2px rgb(23 28 24 / 0.18)`): the hero lower third, the screening stage caption band, the operator detail drawer.
- **Soft** (`box-shadow: 0 10px 30px -18px rgb(23 28 24 / 0.35)`): small floating notes over imagery, the video play control, the selected call-example option.

### Named Rules
**The Opaque Air Rule.** Anything that overlaps something else is solid paper or solid ink. No blur, no glass.

## Shapes

Soft, friendly but not bubbly. Actions are full pills (999px). Large containers and featured bulletins use 1.25rem corners; caption bands and fields 0.875rem; rows and inset panels 0.75rem; bugs and chips a tight 0.5rem so they read as broadcast tags. When a caption band joins a lime or magenta tally strip, the band's own radius is dropped and the pair is clipped by the outer container. Borders are 2px (fields, outline buttons, featured bulletin, ruled sections); the spine rail is a 2px paper-3 line with 2.25rem circular nodes; dashed borders mark concept or proposed-only content.

The signal-arc motif: concentric arcs centred on Ringo's ring, decreasing in stroke width and opacity outward, round caps, one contiguous magenta segment. The wordmark repeats it: a lime ring with one magenta segment.

## Components

### Buttons
Large, plain, confident pills.
- **Shape:** full pill (999px), 3.25rem minimum height (hero actions 3.5rem), 2px border slot, weight 750.
- **Primary:** flat Tally Lime with charcoal text — no bevel, gradient or glow. Hover brightens to lime-hover and lifts 1px.
- **Secondary:** transparent with a 2px charcoal outline; hover inverts to charcoal fill with paper text.
- **Ink:** charcoal fill, paper text; hover to ink-3. Used for "Open dashboard" and bulletin actions.
- **Quiet:** 2.75rem, paper fill with a rule outline, darkening to ink on hover; on charcoal it turns transparent with a faint paper outline that goes lime on hover.
- **Focus:** 3px charcoal outline at 3px offset; lime outline on charcoal grounds. Disabled at 55% opacity.

### Station Bug
The single data-source tag of a view: slate type, 0.5rem corners, icon plus word. LIVE is lime fill; REPLAY / DEMO / EXAMPLE are paper with a 2px inset ink ring (inverted on charcoal); SIGNAL LOST is amber wash with an amber-ink ring; Connecting / Locked is a faint outline on charcoal. The same tag shape labels examples and concepts in content ("Example", "Concept · not live yet").

### Caption Band and Lower Third
A charcoal band, paper text, 0.875rem corners. Speaker changes use `>> CALLER` / `>> FIRSTRING` in slate caps in muted-on-ink. A lower third is a caption band topped by a lime tally strip (or a magenta-ink strip for a suspected tactic) carrying slate-caps labels, with a hairline divider before FirstRing's outcome line and its icon.

### Chips (outcomes)
- **Style:** 0.5rem corners, weight 750 at 0.9375rem, always a 16px icon plus words.
- **States:** saved = lime wash / lime ink; blocked = magenta wash / magenta ink; needs review = amber wash / amber ink; incomplete and no message = paper-3 / ink-3; in progress = charcoal with lime text.
- **Proposal tags:** slate caps; executed proposals are a 1.5px ink-3 inset outline, unexecuted ones amber.

### Cards / Containers
- **Corner Style:** 1.25rem.
- **Background:** paper with a 2px ink border for featured bulletins (with a charcoal header strip); paper-2 for quiet inset panels.
- **Shadow Strategy:** none at rest (see Elevation).
- **Internal Padding:** 1.25–2rem.

### Inputs / Fields
- **Style:** white fill, 2px rule border, 0.875rem corners, 3.25rem tall, 1.0625rem text; labels 750 weight above, muted help text below.
- **Focus:** border goes charcoal and a 3px lime outline sits 1px outside.
- **Error:** magenta-ink border, with a magenta-wash notice carrying words.

### Navigation
Landing: sticky paper header, 4.5rem tall, a rule border appears after scroll; links are pill-hover text links in ink-3 at 1.0625rem; "Open dashboard" is an ink button. Console: see Console Rail below. Below 1024px both collapse (Menu list with 3.5rem rows on landing; a five-up icon-over-label tab row with a lime underline on the active tab in the console).

### Rundown Row
One link per call: a 8.5rem time column (bold time, muted age and duration), then the outcome chip, "for {name}", and a two-line bulletin, then a chevron that nudges right on hover. Hover tints paper-2; selected is paper-3.

### Decision Spine
A vertical rail of numbered stations per caller turn: slate station name and tabular time in a right-aligned left column (inline on mobile), a 2.25rem circular icon node coloured by tone (ok lime, protect magenta, warn amber, fail charcoal, proposed dashed ink-3), then title, caption-band quote for heard lines, a key/value list and the raw receipt link.

### Presentation Studio (`/console/present`)
The only charcoal-ground surface in the console. It is the projected broadcast of one call, chrome-less, with its own top strip:
- **Top strip:** wordmark, a Call/Community scene switch, the "Showing" select (Follow newest call / pinned call), the station bug, Evidence, Reset display and Exit.
- **Caption:** `>> CALLER` slate, then the caller's words at `clamp(1.875rem, 1.1rem + 2.9vw, 4.25rem)`, weight 800, measure 28ch. Partial speech shows at 80% opacity with "…".
- **Branch strip:** two ruled rows. A lime disc marks the earlier ordinary request; a magenta-on-ink square marks "Then the request changed."
- **Lower third:** paper band with a tally strip. Lime for active and done, magenta-ink for protect, amber-wash for warn. "FirstRing · now", the step at `clamp(1.5rem, 1.1rem + 1.5vw, 2.5rem)`, and FirstRing's last reply as a `>> FIRSTRING` caption.
- **Outcome graphic:** a 1.25rem-radius block. Fill by outcome:
  - lime fill with ink text for things that happened (connected, message saved);
  - magenta-wash with magenta-ink for blocked or withdrawn;
  - amber-wash for permission pending;
  - a dashed outline while no outcome exists.
  It always carries an icon and words.
- **Waiting cover:** "Call FirstRing", the inbound number in lime at display scale, the readiness rows (check, or dashed circle with amber), an optional tel: QR on white, and Ringo's rest pose. One slow lime beacon, removed under reduced motion.
- **Service unavailable:** a separate cover with the SIGNAL LOST bug. It is never shown as waiting.

### Stage Chip
A rundown of evidence-backed stages: 0.5rem corners, 1rem/750 type, a 0.7rem shape dot plus words and tabular time.
- Dot shapes: circle for neutral and active; square for protect, in magenta-on-ink text; rotated amber diamond for warn and fail.
- On ink the chips are ink-3. The newest ("now") chip inverts to paper, lime for ok, or magenta-wash for protect.
- Only the last seven show, behind "+N earlier".

### Community Story
The presentation screen keeps the large version below. The dashboard uses a compact warning trail instead: the headline, a 320×72 line of two 22px-radius rings joined to the shared-warning capsule (same solid-when-receipted rule), and the five stages as a ruled rundown with times.

Two household rings (lime, 20px stroke) and a capsule-shaped shared warning (magenta-ink outline, magenta triangle). Concentric arcs sit behind each ring.
- A connector is drawn solid lime only when its stage has a receipt; it is dotted otherwise.
- A magenta ring segment marks "spotted" on the source ring and "matched" on the receiving ring.
- Below it, a ruled list of five stages, each marked receipt observed, not yet, or not available.
- GSAP draws a connector, or pulses the ring once, only when a stage newly completes.

### Console Rail
The console's navigation wireframe, top to bottom: wordmark; the line it answers for ("Sarah’s line", the number, and the live state in words: "Live, updated 11:20 AM." plus "View only." on the public copy); three groups split by hairlines (Overview · Calls · Callers, then Community, then Settings) with tabular counts at the right; Present (lime) and Operator detail (quiet) at the foot; then the rail art. The ground runs from ink to ink-deep (#0f1612) so the art joins without a seam. The art is a Higgsfield illustration of Ringo floating in night water with the lime ring balanced on his nose (`public/media/console-sidebar-*`), masked in from the top, with three thin lime ripple ellipses that expand every 9s (removed under reduced motion, hidden below 760px of height). Active item: paper fill, ink text, soft lift.

### Dashboard Patterns
- **Overview hierarchy:** one bordered bulletin (the most important call) is the largest element; beside it a calm outcome summary; below, a six-row recent-calls table; everything else (callers to watch, shared warnings, trusted callers, how the line runs) sits behind one tab row. Never all at once.
- **Outcome proportion bar:** 0.875rem tall, 2px paper gaps, 4px rounded ends, fixed order blocked (#b3005c) · needs review (#d39b2a) · put through (#3a6300) · message saved (#76b900) · no message or incomplete (#b8bfb6), validated for adjacent CVD separation. It always sits on a ruled table with icon, words and count, so colour is never alone.
- **Segmented switch:** paper-3 pill track, ink thumb, 2.25rem buttons (Today / All).
- **Tabs:** 3rem tabs on a 2px ink rule; the selected tab is an ink tab with paper text; arrow keys move between tabs.
- **Disclosure:** a 2px ink top rule, a 3.25rem summary row in 800 weight, and a chevron that turns on open. Used for details one click away (comparison tables, every receipt, technical details).
- **Settings rows:** a sticky pill index of sections, then label-left (20rem) / content-right rows separated by 2px ink rules.
- **Trusted callers:** a ruled list with a lime-wash person-check disc, name, relation and "ending 1234"; removing needs a second, named confirmation. The caller-ID warning is always shown in amber-ink words.
- **Read-only public view:** off localhost (or with `?view=public`) every write control is removed, never merely greyed, and one muted line says "View only. Changes are made on the presenting laptop."

### Disabled Buttons
Disabled controls keep full-contrast text: a paper-3 fill with muted text (on ink: transparent with muted-on-ink text). They no longer fade with opacity.

## Do's and Don'ts

### Do:
- **Do** show every transcript or quoted caller line as a charcoal caption band with `>> SPEAKER` slate labels.
- **Do** pair every outcome colour with an icon and words; magenta always says "Suspected" or "blocked".
- **Do** keep exactly one data-source station bug per view, in the header on the console.
- **Do** use the `-ink` variants (lime-ink #3a6300, magenta-ink #b3005c) for coloured text on paper.
- **Do** reserve lime washes for things that happened; render proposals as neutral outline or amber.
- **Do** keep primary actions flat lime pills with charcoal text.
- **Do** render progressions and comparisons as ruled rows and rundown lines.
- **Do** keep overlays and floating graphics opaque, using the Lift or Soft shadow only.
- **Do** derive every presentation stage, outcome and community stage from a receipt; show missing ones as "not yet" or "not available", never as done.

### Don't:
- **Don't** put eyebrows or kickers above headings; carry hierarchy with scale.
- **Don't** use backdrop blur, glass or translucent overlays.
- **Don't** add a bevel, gradient or glow to the primary button, and never draw a lime-to-magenta gradient.
- **Don't** nest cards inside cards.
- **Don't** show a model proposal in lime or as completed.
- **Don't** add ornaments beyond the concentric signal arcs and Ringo.
- **Don't** set reading text below 0.95rem or rely on hover for essential information.
