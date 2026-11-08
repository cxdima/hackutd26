# Console contract v1

Authoritative backend owner: root. Frontend consumes this contract; do not invent provider receipts.

All `/api/*` routes require `Authorization: Bearer <DEMO_ADMIN_TOKEN>`. Browser keeps token in memory only. Same-origin Vite proxy `/api` → `http://127.0.0.1:8080`; production serves web/dist. SSE uses fetch streaming to carry the authorization header. Resume with `Last-Event-ID` (decimal event sequence). Database events are replayed on reconnect, then live stream continues. Dedupe by `id`.

`GET /api/state` returns:
```json
{"execution_mode":"replay","data_origin":"synthetic","transport":"probe","phone_display":"Not configured","selected_household":"sarah","capabilities":[{"id":"inbound","status":"pending","detail":"Twilio credentials and HTTPS origin absent"},{"id":"outbound","status":"pending","detail":"Separate Photon SIP setup absent"}],"sessions":[],"community":[{"household_id":"sarah","display_name":"Sarah","last_applied_version":0},{"household_id":"maya","display_name":"Maya","last_applied_version":0}],"tactics":[]}
```
Capability statuses: `pending|configured|verified|disabled`; configured never means phone tested. Session shape:
```json
{"id":"uuid","household_id":"sarah","state":"screening","policy_epoch":0,"generation_id":0,"speech_pending":false,"inbound_ended":false,"turns":[{"id":"uuid","text":"...","speaker":"caller"}],"current_action":"Waiting for a call","evidence_quote":null,"offer":null,"legs":[],"execution_mode":"replay","data_origin":"synthetic"}
```
Offer: `{id,summary,status:pending|revoked|accepted|declined|expired,policy_epoch,expires_at}`. Leg: `{action_id,role:sarah|family|clinic, status:reserved|unknown|queued|initiated|ringing|in-progress|completed|busy|failed|no-answer|canceled, acknowledged:boolean}`. No phone numbers in leg data. `GET /api/session/:id` returns that same session object.

Additive session `message` defaults to null for older snapshots: `{summary,recorded_at,turn_version,evidence_turn_ids}`. It contains the minimized private message only after a successful durable save. `message.recorded` carries metadata only, never the summary; the authenticated incident snapshot supplies its private display. A saved message is not a delivered-message receipt and never a community capsule. Readiness IDs now include `inbound`, `outbound`, `relay`, `nemotron`; each is configuration only until separately observed live evidence exists.

`GET /api/events` SSE data is one JSON object: `{id,session_id,leg_role,occurred_at,received_at,sequence,policy_epoch,execution_mode,data_origin,type,payload}`. Times are ISO 8601. Session_id may be null for community events, leg_role `caller|sarah|family|clinic|system`. Event type names are those in guide section 7; additional honest events include `action.reserved`, `outbound.unknown`, `audio.requested`, `transport.keypad`, `session.updated`. Snapshot refresh on event is valid; no UI inferred acknowledgements. `/api/state` is the source for current snapshot; events supply timeline/evidence.

Controls (JSON):
- POST `/api/demo/household` `{household_id:"sarah"|"maya"}` selects ONLY the next inbound session.
- POST `/api/demo/probe` `{session_id,contact_id:"sarah"}` requests M1 outbound Phone B. Backend owns destination, uniqueness and mode guards.
- POST `/api/demo/end-call` `{session_id}` ends owned test legs.
- POST `/api/demo/reset` `{}` requests test-session termination; keeps audit events.
- POST `/api/demo/replay` `{scenario:"benign"|"revocation"|"benign_new_number"}` runs authored fixtures, only in replay execution mode. This never creates calls or inference. Show REPLAY prominently.

Community capsule: `{id,version,tactic,requested_action,pretext_category,pressure_category,expires_at}`; version receipt belongs to household. No names, numbers, transcripts or quotes in capsule. Show shared memory as applied only from actual stored receipt. No fictitious live call, inferred audio heard, fake risk score, clone claim, or subscriber success.
