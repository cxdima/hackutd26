# Private permission call (handoff): what the console reads

The backend already implements the flow:
1. The model's `offer_handoff` proposal.
2. Rust's checks: `HANDOFF_CALLS_ENABLED`, the current assessment and the policy epoch.
3. A reserved outbound leg to Sarah's server-enrolled endpoint.
4. A private whisper summary on answer.
5. Keypad choice: 1 to connect, 2 to decline.
6. Conference bridge.

Revocation and stale-key rejection happen if the caller's request changes. The console only **reads** these receipts. No page load, refresh, reconnect, navigation or test places a call. The only console-initiated outbound call is the existing one-time Phone B test in Operator detail, which is gated by the server's `phone_b.can_request` and needs confirmation.

## Receipts and how they are shown

| Receipt | Console stage | Notes |
|---|---|---|
| `agent.tool.proposed` `offer_handoff` | Model proposed a handoff | A proposal alone creates nothing. |
| `policy.denied` `{tool:"offer_handoff"}` | "Rust refused it" + detail | e.g. "Private handoff calls are not authorized yet". |
| `handoff.offered` `{offer_id, summary, expires_at}` | Permission requested | `summary` is private: household dashboard and evidence drawer only, never community views. |
| `outbound.requested` (handoff leg) | Calling Sarah's phone | Provider accepted the request. **Not** ringing. |
| `outbound.ringing` | Sarah's phone is ringing | |
| `outbound.answered` | Sarah picked up · private summary requested | Hearing is not verified. Only a keypad choice shows someone is there. |
| `handoff.choice` `accept` / `decline_or_empty` | Sarah pressed 1 / declined | A `1` followed by `handoff.stale_rejected` is shown as "pressed 1 after the offer was withdrawn". |
| `bridge.connected` | Connected | The only receipt that produces "Connected". |
| `handoff.revoked` | Permission withdrawn | |
| `handoff.stale_rejected` | Old keypad approval rejected | |
| `outbound.unknown` / `dependency.failed` / failed leg status | Waiting for the provider / call failed | No retry is implied. |
| `offer.status == "expired"` | Offer expired | From the snapshot. |

Handoff legs are identified from the snapshot (`legs[].kind == "handoff"`, `offer.leg_id`), never from model output. Logic: `handoffView()` and `liveStages()` in `src/app/core/live.ts`. Tests: `live.spec.ts`, `demo-staged.spec.ts`, `e2e/present.spec.ts`.

## One backend addition requested

`/api/state` does not say whether permission calls are armed. Settings and the presentation readiness list therefore show "The server does not report it yet". A one-line capability would let the console show the server's own answer:

```rust
{"id":"handoff","status":if c.handoff_enabled && c.outbound_configured() && c.contacts.contains_key("sarah") {"configured"} else {"disabled"},"detail":"Private permission calls to the enrolled household phone; configured is not tested"},
```

The console reads `capabilities[id=handoff]` when present. No frontend change is needed after adding it. Arming remains a server setting (`HANDOFF_CALLS_ENABLED`), with no frontend switch, and the enrolled number is never sent to the browser.
