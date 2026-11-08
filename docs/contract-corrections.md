# Recorded contract corrections

## Payment meaning (2026-11-07)

The master guide's single `requested_object=payment` value cannot distinguish “Please bring your copay to the clinic” from “Buy a gift card and read its number” while preserving their actual request speech act and direction. A regression test demonstrated that treating every directed payment request as diversion blocks the benign copay case.

The shared `Observation` enum therefore adds **`payment_diversion`**. `payment` retains ordinary on-site payment/reminder meaning. Rust protects a directed request for `payment_diversion`, `authentication_secret`, or `remote_access`; it does not flag ordinary payment merely because it is a request. Both assessment and handoff tools use the same schema, and the model prompt/evaluator use the same distinction.

This is a categorical contract repair, not proof of Nemotron's extraction accuracy. The frozen ordinary-payment and diversion cases still require real model evaluation. Other observation fields and tool arguments are unchanged; original guide copies are preserved.

## Gather retry identity

The fallback adds a server-owned `turn_id` nonce to `/voice/turn` action URLs. Signed duplicate callbacks claim work once and receive the cached TwiML response. This avoids treating a webhook retry as a new caller turn without assuming a universal Twilio event ID. The route and product behavior remain the guide's Gather fallback.
