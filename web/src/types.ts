import { z } from "zod";

const mode = z.enum(["live", "replay"]);
export const modelBackendSchema = z.enum(["nvidia_hosted", "brev_hosted"]);
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
const observationSchema = z.object({
  speech_act: z.enum([
    "request",
    "statement",
    "negation",
    "quotation",
    "unclear",
  ]),
  requested_object: z.enum([
    "authentication_secret",
    "appointment_reference",
    "payment",
    "payment_diversion",
    "remote_access",
    "routine_information",
    "unknown",
  ]),
  direction: z.enum([
    "caller_requests_from_sarah",
    "caller_supplies",
    "none",
    "unclear",
  ]),
});
export const assessmentSchema = z.object({
  turn_version: z.number().int().nonnegative(),
  generation_id: z.number().int().nonnegative(),
  observation: observationSchema,
  evidence_refs: z.array(z.object({ turn_id: z.string(), quote: z.string() })),
});
const acceptedUnderstandingSchema = z.object({
  generation_id: z.number().int().nonnegative(),
  turn_version: z.number().int().nonnegative(),
  policy_epoch: z.number().int().nonnegative(),
  understanding: z.object({
    version: z.literal(1),
    claimed_identity: z.string().max(120).nullable(),
    purpose: z.string().max(180),
    requested_action: z.string().max(180),
    supplied_facts: z
      .array(
        z.object({
          text: z.string().max(180),
          evidence_refs: z.array(
            z.object({ turn_id: z.string(), quote: z.string() }),
          ),
        }),
      )
      .max(4),
    missing_information: z.array(z.string().max(160)).max(3),
    readiness: z.enum([
      "needs_information",
      "ready",
      "caller_declined",
      "no_message",
    ]),
  }),
});
export const sessionSchema = z.object({
  id: z.string(),
  household_id: z.string(),
  state: z.string(),
  policy_epoch: z.number(),
  generation_id: z.number(),
  speech_pending: z.boolean(),
  inbound_ended: z.boolean(),
  closing_reason: z.string().nullable().optional(),
  current_action: z.string(),
  evidence_quote: z.string().nullable(),
  turn_version: z.number().int().nonnegative().optional(),
  assessed_version: z.number().int().nonnegative().optional(),
  assessments: z.array(assessmentSchema).optional(),
  understanding: acceptedUnderstandingSchema.nullable().optional(),
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
const proofSchema = z.object({
  status: z.enum(["pending", "passed", "failed"]),
  detail: z.string(),
  recorded_at: z.string().nullable().optional(),
  source: z.string().optional(),
});
export const stateSchema = z.object({
  execution_mode: mode,
  data_origin: z.string(),
  transport: z.string(),
  phone_display: z.string(),
  selected_household: z.string(),
  model_backend: z
    .object({
      backend: modelBackendSchema,
      label: z.string(),
      requested_model: z.string(),
      configured: z.boolean(),
      protocol_verified: z.boolean(),
      detail: z.string(),
    })
    .optional(),
  speech: z
    .object({
      source: z.literal("configured"),
      transport: z.string(),
      provider: z.string(),
      voice: z.string(),
      voice_label: z.string(),
      closing_provider: z.string(),
      closing_voice: z.string(),
      transcription_provider: z.string().nullable(),
      transcription_model: z.string().nullable(),
    })
    .optional(),
  evidence: z
    .object({
      model_protocol: proofSchema,
      inbound_phone: proofSchema,
      relay_phone: proofSchema,
    })
    .optional(),
  phone_b: z
    .object({
      enrolled: z.boolean(),
      enabled: z.boolean(),
      authorized: z.boolean(),
      remaining_attempts: z.number().int().nonnegative(),
      can_request: z.boolean(),
      reason: z.string(),
    })
    .optional(),
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
