import { sessionSchema, type CallEvent, type Session } from '../app/core/schemas';

/** Small, explicit synthetic receipts for unit tests. */
let seq = 0;
export function receipt(
  type: string,
  payload: Record<string, unknown> = {},
  overrides: Partial<CallEvent> = {},
): CallEvent {
  seq += 1;
  return {
    id: `evt-${seq}`,
    sequence: seq,
    session_id: 's1',
    leg_role: 'system',
    occurred_at: new Date(Date.UTC(2026, 10, 8, 14, 0, seq)).toISOString(),
    received_at: new Date(Date.UTC(2026, 10, 8, 14, 0, seq)).toISOString(),
    policy_epoch: 0,
    execution_mode: 'live',
    data_origin: 'synthetic',
    type,
    payload,
    ...overrides,
  };
}

export function session(overrides: Record<string, unknown> = {}): Session {
  return sessionSchema.parse({
    id: 's1',
    household_id: 'sarah',
    state: 'ended',
    policy_epoch: 0,
    generation_id: 3,
    speech_pending: false,
    inbound_ended: true,
    current_action: 'Call ended',
    evidence_quote: null,
    offer: null,
    legs: [],
    execution_mode: 'live',
    data_origin: 'synthetic',
    turns: [],
    ...overrides,
  });
}

export const NOW = Date.UTC(2026, 10, 8, 14, 5);
