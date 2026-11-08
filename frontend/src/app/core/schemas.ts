import { z } from 'zod';

/**
 * Runtime validation of the Rust console contract (docs/event-contract.md).
 * Ported from web/src/types.ts. Unknown additive fields are tolerated; a missing
 * or mistyped required field is treated as an incompatible server, never guessed.
 */
export const executionModeSchema = z.enum(['live', 'replay']);
export type ExecutionMode = z.infer<typeof executionModeSchema>;
/** Known backends get friendly labels; an unknown future backend stays a readable string. */
export const modelBackendSchema = z.string();

export const householdIdSchema = z.enum(['sarah', 'maya']);
export type HouseholdId = z.infer<typeof householdIdSchema>;

export const profileSchema = z.object({
  household_id: householdIdSchema,
  version: z.number().int().nonnegative(),
  display_name: z.string(),
  greeting: z.string(),
  tts_provider: z.string(),
  voice_id: z.string(),
  locale: z.string(),
  timezone: z.string(),
  source: z.string().optional(),
  applies_to: z.string().optional(),
});
export type Profile = z.infer<typeof profileSchema>;

export const legSchema = z.object({
  action_id: z.string(),
  kind: z.string().optional(),
  role: z.string(),
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
  execution_mode: executionModeSchema,
  data_origin: z.string(),
  type: z.string(),
  payload: z.record(z.string(), z.unknown()),
});
export type CallEvent = z.infer<typeof eventSchema>;

export const requestedObjectSchema = z.enum([
  'authentication_secret',
  'appointment_reference',
  'payment',
  'payment_diversion',
  'remote_access',
  'routine_information',
  'unknown',
]);
export type RequestedObject = z.infer<typeof requestedObjectSchema>;

export const observationSchema = z.object({
  speech_act: z.enum(['request', 'statement', 'negation', 'quotation', 'unclear']),
  requested_object: requestedObjectSchema,
  direction: z.enum(['caller_requests_from_sarah', 'caller_supplies', 'none', 'unclear']),
});
export type Observation = z.infer<typeof observationSchema>;

const evidenceRefSchema = z.object({ turn_id: z.string(), quote: z.string() });

export const assessmentSchema = z.object({
  turn_version: z.number().int().nonnegative(),
  generation_id: z.number().int().nonnegative(),
  observation: observationSchema,
  evidence_refs: z.array(evidenceRefSchema),
});
export type Assessment = z.infer<typeof assessmentSchema>;

export const readinessSchema = z.enum([
  'needs_information',
  'ready',
  'caller_declined',
  'no_message',
]);

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
      .array(z.object({ text: z.string().max(180), evidence_refs: z.array(evidenceRefSchema) }))
      .max(4),
    missing_information: z.array(z.string().max(160)).max(3),
    readiness: readinessSchema,
  }),
});

/** A private handoff offer. Its summary is private to the household and never shown in shared views. */
export const offerSchema = z.object({
  id: z.string(),
  summary: z.string(),
  status: z.string(),
  policy_epoch: z.number(),
  expires_at: z.string(),
  leg_id: z.string().optional(),
});
export type Offer = z.infer<typeof offerSchema>;

export const sessionSchema = z.object({
  id: z.string(),
  household_id: z.string(),
  profile: profileSchema.nullable().optional(),
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
  blocked: z.boolean().optional(),
  ambiguous: z.boolean().optional(),
  message: z
    .object({
      summary: z.string(),
      recorded_at: z.string(),
      turn_version: z.number().int().positive(),
      evidence_turn_ids: z.array(z.string()),
    })
    .nullable()
    .optional(),
  turns: z.array(z.object({ id: z.string(), text: z.string(), speaker: z.string() })),
  offer: offerSchema.nullable(),
  offer_history: z.array(offerSchema).optional(),
  bridge_committed: z.boolean().optional(),
  end_requested: z.boolean().optional(),
  legs: z.array(legSchema),
  execution_mode: executionModeSchema,
  data_origin: z.string(),
});
export type Session = z.infer<typeof sessionSchema>;

const proofSchema = z.object({
  status: z.enum(['pending', 'passed', 'failed']),
  detail: z.string(),
  recorded_at: z.string().nullable().optional(),
  source: z.string().optional(),
});
export type Proof = z.infer<typeof proofSchema>;

export const tacticSchema = z.object({
  id: z.string(),
  version: z.number(),
  tactic: z.string(),
  requested_action: z.string(),
  pretext_category: z.string(),
  pressure_category: z.string(),
  expires_at: z.string(),
  recommended_action: z.string().optional(),
  provenance: z.string().optional(),
  ttl_hours: z.number().optional(),
  schema_version: z.number().optional(),
});
export type Tactic = z.infer<typeof tacticSchema>;

export const stateSchema = z.object({
  execution_mode: executionModeSchema,
  data_origin: z.string(),
  transport: z.string(),
  phone_display: z.string(),
  selected_household: z.string(),
  profile: profileSchema.optional(),
  model_backend: z
    .object({
      backend: modelBackendSchema,
      label: z.string(),
      requested_model: z.string(),
      configured: z.boolean(),
      protocol_verified: z.boolean(),
      detail: z.string(),
      /** Live availability check (backend in progress): additive and optional. */
      current: z
        .object({
          available: z.boolean(),
          reason: z.string().nullable().optional(),
          checked_at: z.string().nullable().optional(),
          available_until: z.string().nullable().optional(),
          endpoint_reachable: z.boolean().optional(),
          protocol_current: z.boolean().optional(),
        })
        .optional(),
    })
    .optional(),
  speech: z
    .object({
      source: z.literal('configured'),
      transport: z.string(),
      provider: z.string(),
      voice: z.string(),
      voice_label: z.string(),
      closing_provider: z.string(),
      closing_voice: z.string(),
      transcription_provider: z.string().nullable(),
      transcription_model: z.string().nullable(),
      speech_timeout_ms: z.number().nullable().optional(),
      emergency_fallback_provider: z.string().nullable().optional(),
      emergency_fallback_voice: z.string().nullable().optional(),
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
  /** Status stays an open string: a new server status must not break the console. */
  capabilities: z.array(z.object({ id: z.string(), status: z.string(), detail: z.string() })),
  sessions: z.array(sessionSchema),
  community: z.array(
    z.object({
      household_id: z.string(),
      display_name: z.string(),
      last_applied_version: z.number().int().nonnegative(),
    }),
  ),
  tactics: z.array(tacticSchema),
});
export type ConsoleState = z.infer<typeof stateSchema>;

export const matchSchema = z.object({
  id: z.string(),
  version: z.number().int().positive(),
  requested_action: z.string(),
  pretext_category: z.string(),
  expires_at: z.string(),
  household_applied_version: z.number().int().nonnegative(),
  matched_fields: z.array(z.enum(['requested_action', 'pretext_category'])),
});
export type TacticMatch = z.infer<typeof matchSchema>;

export const modelObservationSchema = z.object({
  backend: modelBackendSchema.optional(),
  requested_model: z.string().nullable().optional(),
  response_model: z.string().max(200).nullable().optional(),
  elapsed_ms: z.number().int().nonnegative(),
  completion_tokens: z.number().int().nonnegative().nullable().optional(),
  reasoning_tokens: z.number().int().nonnegative().nullable().optional(),
  finish_reason: z.string().nullable().optional(),
  error: z.string().nullable().optional(),
  outcome: z.string(),
});
export type ModelObservation = z.infer<typeof modelObservationSchema>;

/** Read-only automated caller status. Scenario IDs change often, so they stay open strings. */
export const fixtureStatusSchema = z.object({
  scenario_id: z.string(),
  destination: z.literal('owned_firstring_number_only'),
  test_kind: z.literal('automated_audio_caller'),
  voice: z.string(),
  clip_count: z.number().int().nonnegative(),
  time_limit_seconds: z.number().int().positive(),
  remaining_attempts: z.number().int().nonnegative(),
  max_attempts: z.number().int().nonnegative().optional(),
  attempt_count: z.number().int().nonnegative().optional(),
  state: z.string().optional(),
  provider_status: z.string().optional(),
  outcome: z.string().optional(),
  clips_cached: z.boolean().optional(),
  human_audition: z.literal(false),
  noise_tested: z.literal(false),
  verdict: z.literal('ungraded').optional(),
  observations: z
    .array(
      z.object({
        step: z.number().int().nonnegative(),
        speech_result: z.string(),
        origin: z.literal('twilio_gather_audio_asr'),
      }),
    )
    .optional(),
});
export type FixtureStatus = z.infer<typeof fixtureStatusSchema>;
