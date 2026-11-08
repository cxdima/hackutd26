# Optional voice enrollment: proposed contract

Status: **not available.** `/api/state` reports `{"id":"voice_clone","status":"disabled","detail":"Preset assistant voice; no clone endpoint verified"}`.

Settings → "Your own voice (optional)" therefore does two things:
- It shows the preset voice actually in use (`state.speech.voice_label` and provider) and the server's disabled status.
- It offers the steps as an explicitly labeled **in-browser example**. Nothing is uploaded, no voice is created, and the real profile is not changed.

Cloning is optional and must never delay the working phone demonstration.

## Product rules the UI already follows

- A preset synthetic voice is the default. The person's own voice is opt-in.
- Explicit consent covers three statements: it is my own voice; a voice can be recorded and imitated, and resemblance never proves identity; I can turn it off and delete it. Consent also records the speaker's name.
- Requirements come from the server. The example uses 20 to 120 s and WAV, MP3, M4A, WebM or OGG, labeled as example values.
- Processing and verification states are shown as states, never as a completed voice.
- Preview only plays audio the server produced. In the example, preview is disabled with an explanation.
- Applying a voice changes a named profile for **new** calls only (like the profile editor).
- Turning it off revokes and deletes the sample and voice.
- No provider key or provider voice ID secret ever reaches the browser.

## Proposed endpoints (operator-authenticated, like `/api/profile/{household}`)

| Method | Path | Body / result |
|---|---|---|
| `GET` | `/api/voice/{household}` | `{ state, requirements: {min_seconds, max_seconds, formats[]}, consent?: {statement_version, recorded_at}, voice?: {voice_id, label, preview_url?}, updated_at }` |
| `POST` | `/api/voice/{household}/consent` | `{ statement_version, speaker_name, accepted: true }` → `{ state: "consented" }` |
| `POST` | `/api/voice/{household}/sample` | `multipart/form-data` audio → `202 { state: "processing" }` |
| `POST` | `/api/voice/{household}/apply` | `{ expected_version }` → the updated profile (same shape as `/api/profile`) |
| `DELETE` | `/api/voice/{household}` | `202 { state: "revoked" }`. Deletes the sample and the provider voice. |

`state` is one of `none | consented | processing | verifying | ready | applied | failed | revoked`. The console's `EnrollmentState` in `src/app/core/voice.ts` matches it. Progress events on the SSE stream would be `voice.enrollment.*`, carrying the same `state`.

`preview_url` should be a short-lived capability URL like the existing `/media/speech/{capability}` (private, `no-store`).

When the capability becomes `configured`, the console needs three changes:
- Replace the example adapter with fetches to these routes.
- Keep the consent copy.
- Show the server's requirements instead of the example values.
