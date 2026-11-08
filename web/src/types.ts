import { z } from "zod";

const mode = z.enum(["live", "replay"]);
export type ExecutionMode = z.infer<typeof mode>;
export const legSchema = z.object({
  action_id: z.string(),
  kind: z.string().optional(),
  role: z.enum(["sarah", "family", "clinic"]),
  status: z.string(),
  acknowledged: z.boolean(),
});
export type Leg = z.infer<typeof legSchema>;
export const eventSchema = z.object({
  id: z.string(),
  sequence: z.number().int().nonnegative(),
  session_id: z.string().nullable(),
  leg_role: z.string(),
  occurred_at: z.string(),
  received_at: z.string(),
  policy_epoch: z.number(),
  execution_mode: mode,
  data_origin: z.string(),
  type: z.string(),
  payload: z.record(z.string(), z.unknown()),
});
export type CallEvent = z.infer<typeof eventSchema>;
export const sessionSchema = z.object({
  id: z.string(),
  household_id: z.string(),
  state: z.string(),
  policy_epoch: z.number(),
  generation_id: z.number(),
  speech_pending: z.boolean(),
  inbound_ended: z.boolean(),
  current_action: z.string(),
  evidence_quote: z.string().nullable(),
  message: z
    .object({
      summary: z.string(),
      recorded_at: z.string(),
      turn_version: z.number().int().positive(),
      evidence_turn_ids: z.array(z.string()),
    })
    .nullable()
    .optional(),
  turns: z.array(
    z.object({ id: z.string(), text: z.string(), speaker: z.string() }),
  ),
  offer: z
    .object({
      id: z.string(),
      summary: z.string(),
      status: z.string(),
      policy_epoch: z.number(),
      expires_at: z.string(),
    })
    .nullable(),
  legs: z.array(legSchema),
  execution_mode: mode,
  data_origin: z.string(),
});
export type Session = z.infer<typeof sessionSchema>;
export const stateSchema = z.object({
  execution_mode: mode,
  data_origin: z.string(),
  transport: z.string(),
  phone_display: z.string(),
  selected_household: z.string(),
  capabilities: z.array(
    z.object({
      id: z.string(),
      status: z.enum(["pending", "configured", "verified", "disabled"]),
      detail: z.string(),
    }),
  ),
  sessions: z.array(sessionSchema),
  community: z.array(
    z.object({
      household_id: z.string(),
      display_name: z.string(),
      last_applied_version: z.number().int().nonnegative(),
    }),
  ),
  tactics: z.array(
    z.object({
      id: z.string(),
      version: z.number(),
      tactic: z.string(),
      requested_action: z.string(),
      pretext_category: z.string(),
      pressure_category: z.string(),
      expires_at: z.string(),
    }),
  ),
});
export type ConsoleState = z.infer<typeof stateSchema>;

export const matchSchema = z.object({
  id: z.string(),
  version: z.number().int().positive(),
  requested_action: z.string(),
  pretext_category: z.string(),
  expires_at: z.string(),
  household_applied_version: z.number().int().nonnegative(),
  matched_fields: z.array(z.enum(["requested_action", "pretext_category"])),
});
export type TacticMatch = z.infer<typeof matchSchema>;
