import type { CallEvent, ConsoleState, Session } from './schemas';

/**
 * Curated, fictional DEMO data for the explicitly selected demo view.
 * Shapes mirror real server receipts so the same code renders them, but every
 * record is marked data_origin "demo_fixture" and the UI labels the view DEMO.
 * Nothing here was a phone call or a model request.
 */
export const DEMO_ORIGIN = 'demo_fixture';

const SARAH = {
  household_id: 'sarah' as const,
  version: 3,
  display_name: 'Sarah',
  greeting: 'Hi, thanks for calling. What’s the call about?',
  tts_provider: 'ElevenLabs',
  voice_id: 'cgSgspJ2msm6clMCkdW9-v4',
  locale: 'en-US',
  timezone: 'America/Chicago',
};
const MAYA = {
  ...SARAH,
  household_id: 'maya' as const,
  version: 1,
  display_name: 'Maya',
  timezone: 'America/New_York',
};

let seq = 0;
function iso(minutesAgo: number, now: number, plusMs = 0) {
  return new Date(now - minutesAgo * 60000 + plusMs).toISOString();
}

interface Script {
  id: string;
  minutesAgo: number;
  profile: typeof SARAH | typeof MAYA;
  steps: [ms: number, type: string, payload: Record<string, unknown>, role?: string][];
}

function events(script: Script, now: number): CallEvent[] {
  return script.steps.map(([ms, type, payload, role]) => {
    seq += 1;
    return {
      id: `demo-${script.id}-${seq}`,
      sequence: seq,
      session_id: script.id,
      leg_role: role ?? (type.startsWith('transcript') ? 'caller' : 'system'),
      occurred_at: iso(script.minutesAgo, now, ms),
      received_at: iso(script.minutesAgo, now, ms),
      policy_epoch: 0,
      execution_mode: 'replay',
      data_origin: DEMO_ORIGIN,
      type,
      payload,
    };
  });
}

const model = (ms: number, tokens: number, finish = 'tool_calls') => ({
  boundary: 'model_http',
  heard: false,
  observation: {
    backend: 'brev_hosted',
    requested_model: 'nvidia/nemotron-3.5-lightning-30b-a3b',
    response_model: 'nvidia/nemotron-3.5-lightning-30b-a3b',
    elapsed_ms: ms,
    completion_tokens: tokens,
    finish_reason: finish,
    outcome: 'success',
    error: null,
  },
});
const obs = (speech_act: string, requested_object: string, direction: string) => ({
  speech_act,
  requested_object,
  direction,
});

export function buildDemo(now = Date.now()): { state: ConsoleState; events: CallEvent[] } {
  seq = 0;
  const scripts: Script[] = [];
  const sessions: Session[] = [];

  // 1. Clinic opener that turns into a sign-in code request: blocked.
  const t1 = { a: 'demo-turn-1a', b: 'demo-turn-1b' };
  scripts.push({
    id: 'demo-call-code',
    minutesAgo: 38,
    profile: SARAH,
    steps: [
      [
        0,
        'call.inbound.received',
        { caller_identity: 'unverified', transport: 'relay', requested_profile: SARAH },
        'caller',
      ],
      [
        400,
        'voice.connected',
        {
          transport: 'relay',
          speech: {
            provider: 'ElevenLabs',
            voice: 'Jessica',
            transcription_provider: 'Deepgram',
            transcription_model: 'flux',
          },
        },
        'caller',
      ],
      [
        6100,
        'transcript.final',
        {
          text: 'Hi, I’m calling from the clinic about Sarah’s appointment next week.',
          turn_id: t1.a,
        },
      ],
      [
        7300,
        'model.request.completed',
        { generation_id: 4, turn_version: 1, ...model(1190, 241, 'stop') },
      ],
      [
        7310,
        'agent.tool.proposed',
        {
          name: 'record_assessment',
          generation_id: 4,
          turn_version: 1,
          arguments: { observation: obs('statement', 'appointment_reference', 'caller_supplies') },
        },
      ],
      [
        7320,
        'policy.allowed',
        {
          operation: 'assessment',
          observation: obs('statement', 'appointment_reference', 'caller_supplies'),
          quote: 'calling from the clinic about Sarah’s appointment next week',
          handoff_allowed: true,
        },
      ],
      [
        7330,
        'agent.tool.result',
        {
          name: 'record_assessment',
          generation_id: 4,
          turn_version: 1,
          ok: true,
          result: { blocked: false, ambiguous: false, assessed_version: 1 },
        },
      ],
      [7900, 'model.request.completed', { generation_id: 4, turn_version: 1, ...model(588, 108) }],
      [
        7910,
        'agent.tool.proposed',
        {
          name: 'ask_caller',
          generation_id: 4,
          turn_version: 1,
          arguments: { purpose: 'clarify_request', question_characters: 37 },
        },
      ],
      [
        7920,
        'audio.requested',
        { text: 'Sure. What would you like me to tell her?', voice: 'preset', heard: false },
      ],
      [
        7930,
        'agent.tool.result',
        {
          name: 'ask_caller',
          generation_id: 4,
          turn_version: 1,
          ok: true,
          result: { audio_requested: true, stop: true },
        },
      ],
      [
        15200,
        'transcript.final',
        {
          text: 'To confirm it, read me the six-digit code we just texted to her phone.',
          turn_id: t1.b,
        },
      ],
      [
        16400,
        'model.request.completed',
        { generation_id: 9, turn_version: 2, ...model(1216, 207, 'stop') },
      ],
      [
        16410,
        'agent.tool.proposed',
        {
          name: 'record_assessment',
          generation_id: 9,
          turn_version: 2,
          arguments: {
            observation: obs('request', 'authentication_secret', 'caller_requests_from_sarah'),
          },
        },
      ],
      [
        16420,
        'policy.denied',
        {
          operation: 'assessment',
          observation: obs('request', 'authentication_secret', 'caller_requests_from_sarah'),
          quote: 'read me the six-digit code we just texted to her phone',
          handoff_allowed: false,
        },
      ],
      [
        16430,
        'agent.tool.result',
        {
          name: 'record_assessment',
          generation_id: 9,
          turn_version: 2,
          ok: true,
          result: { blocked: true, ambiguous: false, assessed_version: 2 },
        },
      ],
      [
        16440,
        'call.understanding.accepted',
        {
          generation_id: 9,
          turn_version: 2,
          purpose: 'confirm appointment',
          requested_action: 'read a texted verification code',
          readiness: 'needs_information',
          missing_information: [],
        },
      ],
      [17050, 'model.request.completed', { generation_id: 9, turn_version: 2, ...model(607, 113) }],
      [
        17060,
        'agent.tool.proposed',
        {
          name: 'take_message',
          generation_id: 9,
          turn_version: 2,
          arguments: { summary_characters: 66 },
        },
      ],
      [
        17070,
        'message.recorded',
        { recorded_at: iso(38, now, 17070), turn_version: 2, evidence_turn_ids: [t1.b] },
      ],
      [
        17080,
        'audio.requested',
        {
          text: 'I can’t help with codes. I’ve saved a note for Sarah. Goodbye.',
          transport: 'twiml_closing',
          voice: 'preset',
          heard: false,
        },
      ],
      [
        17090,
        'agent.tool.result',
        {
          name: 'take_message',
          generation_id: 9,
          turn_version: 2,
          ok: true,
          result: { message_recorded: true, closing: true, stop: true },
        },
      ],
      [
        19000,
        'tactic.matched',
        { matches: [], matched_fields: ['requested_action', 'pretext_category'] },
      ],
      [22400, 'relay.ended', { duration_seconds: 22, status: 'ended' }],
      [24600, 'closing.completed', { speech_received: false }],
      [24800, 'call.ended', {}, 'caller'],
    ],
  });
  sessions.push({
    id: 'demo-call-code',
    household_id: 'sarah',
    profile: SARAH,
    state: 'ended',
    policy_epoch: 1,
    generation_id: 10,
    speech_pending: false,
    inbound_ended: true,
    closing_reason: 'message_saved',
    current_action: 'Call ended',
    evidence_quote: 'read me the six-digit code we just texted to her phone',
    turn_version: 2,
    assessed_version: 2,
    blocked: true,
    ambiguous: false,
    assessments: [
      {
        turn_version: 1,
        generation_id: 4,
        observation: obs('statement', 'appointment_reference', 'caller_supplies') as never,
        evidence_refs: [
          { turn_id: t1.a, quote: 'calling from the clinic about Sarah’s appointment next week' },
        ],
      },
      {
        turn_version: 2,
        generation_id: 9,
        observation: obs('request', 'authentication_secret', 'caller_requests_from_sarah') as never,
        evidence_refs: [
          { turn_id: t1.b, quote: 'read me the six-digit code we just texted to her phone' },
        ],
      },
    ],
    understanding: {
      generation_id: 9,
      turn_version: 2,
      policy_epoch: 1,
      understanding: {
        version: 1,
        claimed_identity: 'from the clinic',
        purpose: 'confirm appointment',
        requested_action: 'read a texted verification code',
        supplied_facts: [],
        missing_information: [],
        readiness: 'needs_information',
      },
    },
    message: {
      summary: 'Suspicious request blocked; independent verification recommended.',
      recorded_at: iso(38, now, 17070),
      turn_version: 2,
      evidence_turn_ids: [t1.b],
    },
    turns: [
      {
        id: t1.a,
        speaker: 'caller',
        text: 'Hi, I’m calling from the clinic about Sarah’s appointment next week.',
      },
      { id: 'demo-a-1', speaker: 'assistant', text: 'Sure. What would you like me to tell her?' },
      {
        id: t1.b,
        speaker: 'caller',
        text: 'To confirm it, read me the six-digit code we just texted to her phone.',
      },
      {
        id: 'demo-a-2',
        speaker: 'assistant',
        text: 'I can’t help with codes. I’ve saved a note for Sarah. Goodbye.',
      },
    ],
    offer: null,
    legs: [],
    execution_mode: 'replay',
    data_origin: DEMO_ORIGIN,
  });

  // 2. Ordinary appointment message: saved.
  const t2 = 'demo-turn-2a';
  scripts.push({
    id: 'demo-call-dentist',
    minutesAgo: 95,
    profile: SARAH,
    steps: [
      [
        0,
        'call.inbound.received',
        { caller_identity: 'unverified', transport: 'relay', requested_profile: SARAH },
        'caller',
      ],
      [
        380,
        'voice.connected',
        {
          transport: 'relay',
          speech: {
            provider: 'ElevenLabs',
            voice: 'Jessica',
            transcription_provider: 'Deepgram',
            transcription_model: 'flux',
          },
        },
        'caller',
      ],
      [
        7400,
        'transcript.final',
        {
          text: 'This is Lakeside Dental. Sarah’s cleaning is moved to Thursday at 2:30. No need to call back.',
          turn_id: t2,
        },
      ],
      [
        8520,
        'model.request.completed',
        { generation_id: 3, turn_version: 1, ...model(1120, 236, 'stop') },
      ],
      [
        8530,
        'agent.tool.proposed',
        {
          name: 'record_assessment',
          generation_id: 3,
          turn_version: 1,
          arguments: { observation: obs('statement', 'appointment_reference', 'caller_supplies') },
        },
      ],
      [
        8540,
        'policy.allowed',
        {
          operation: 'assessment',
          observation: obs('statement', 'appointment_reference', 'caller_supplies'),
          quote: 'Sarah’s cleaning is moved to Thursday at 2:30',
          handoff_allowed: true,
        },
      ],
      [
        8550,
        'agent.tool.result',
        {
          name: 'record_assessment',
          generation_id: 3,
          turn_version: 1,
          ok: true,
          result: { blocked: false, ambiguous: false, assessed_version: 1 },
        },
      ],
      [
        8560,
        'call.understanding.accepted',
        {
          generation_id: 3,
          turn_version: 1,
          purpose: 'reschedule dental cleaning',
          requested_action: 'tell Sarah the new time',
          readiness: 'ready',
          missing_information: [],
        },
      ],
      [9150, 'model.request.completed', { generation_id: 3, turn_version: 1, ...model(599, 102) }],
      [
        9160,
        'agent.tool.proposed',
        {
          name: 'take_message',
          generation_id: 3,
          turn_version: 1,
          arguments: { summary_characters: 61 },
        },
      ],
      [
        9170,
        'message.recorded',
        { recorded_at: iso(95, now, 9170), turn_version: 1, evidence_turn_ids: [t2] },
      ],
      [
        9180,
        'audio.requested',
        {
          text: 'Thanks, I’ve saved your message. Goodbye.',
          transport: 'twiml_closing',
          voice: 'preset',
          heard: false,
        },
      ],
      [
        9190,
        'agent.tool.result',
        {
          name: 'take_message',
          generation_id: 3,
          turn_version: 1,
          ok: true,
          result: { message_recorded: true, closing: true, stop: true },
        },
      ],
      [13000, 'relay.ended', { duration_seconds: 13, status: 'ended' }],
      [15200, 'call.ended', {}, 'caller'],
    ],
  });
  sessions.push({
    id: 'demo-call-dentist',
    household_id: 'sarah',
    profile: SARAH,
    state: 'ended',
    policy_epoch: 0,
    generation_id: 4,
    speech_pending: false,
    inbound_ended: true,
    closing_reason: 'message_saved',
    current_action: 'Call ended',
    evidence_quote: 'Sarah’s cleaning is moved to Thursday at 2:30',
    turn_version: 1,
    assessed_version: 1,
    blocked: false,
    ambiguous: false,
    assessments: [
      {
        turn_version: 1,
        generation_id: 3,
        observation: obs('statement', 'appointment_reference', 'caller_supplies') as never,
        evidence_refs: [{ turn_id: t2, quote: 'Sarah’s cleaning is moved to Thursday at 2:30' }],
      },
    ],
    understanding: {
      generation_id: 3,
      turn_version: 1,
      policy_epoch: 0,
      understanding: {
        version: 1,
        claimed_identity: 'Lakeside Dental',
        purpose: 'reschedule dental cleaning',
        requested_action: 'tell Sarah the new time',
        supplied_facts: [
          {
            text: 'cleaning moved to Thursday at 2:30',
            evidence_refs: [{ turn_id: t2, quote: 'moved to Thursday at 2:30' }],
          },
        ],
        missing_information: [],
        readiness: 'ready',
      },
    },
    message: {
      summary: 'Lakeside Dental: your cleaning moved to Thursday at 2:30. No callback needed.',
      recorded_at: iso(95, now, 9170),
      turn_version: 1,
      evidence_turn_ids: [t2],
    },
    turns: [
      {
        id: t2,
        speaker: 'caller',
        text: 'This is Lakeside Dental. Sarah’s cleaning is moved to Thursday at 2:30. No need to call back.',
      },
      { id: 'demo-a-3', speaker: 'assistant', text: 'Thanks, I’ve saved your message. Goodbye.' },
    ],
    offer: null,
    legs: [],
    execution_mode: 'replay',
    data_origin: DEMO_ORIGIN,
  });

  // 3. Unclear call that hung up mid-way: needs review.
  const t3 = 'demo-turn-3a';
  scripts.push({
    id: 'demo-call-unclear',
    minutesAgo: 260,
    profile: SARAH,
    steps: [
      [
        0,
        'call.inbound.received',
        { caller_identity: 'unverified', transport: 'relay', requested_profile: SARAH },
        'caller',
      ],
      [
        5200,
        'transcript.final',
        {
          text: 'Yeah, it’s about the account, I need to sort something out with her.',
          turn_id: t3,
        },
      ],
      [
        6450,
        'model.request.completed',
        { generation_id: 2, turn_version: 1, ...model(1250, 228, 'stop') },
      ],
      [
        6460,
        'agent.tool.proposed',
        {
          name: 'record_assessment',
          generation_id: 2,
          turn_version: 1,
          arguments: { observation: obs('unclear', 'unknown', 'unclear') },
        },
      ],
      [
        6470,
        'policy.allowed',
        {
          operation: 'assessment',
          observation: obs('unclear', 'unknown', 'unclear'),
          quote: 'it’s about the account',
          handoff_allowed: false,
        },
      ],
      [
        6480,
        'agent.tool.result',
        {
          name: 'record_assessment',
          generation_id: 2,
          turn_version: 1,
          ok: true,
          result: { blocked: false, ambiguous: true, assessed_version: 1 },
        },
      ],
      [
        6490,
        'call.understanding.accepted',
        {
          generation_id: 2,
          turn_version: 1,
          purpose: 'account matter',
          requested_action: 'speak with Sarah',
          readiness: 'needs_information',
          missing_information: ['which account', 'who is calling'],
        },
      ],
      [7080, 'model.request.completed', { generation_id: 2, turn_version: 1, ...model(590, 96) }],
      [
        7090,
        'agent.tool.proposed',
        {
          name: 'ask_caller',
          generation_id: 2,
          turn_version: 1,
          arguments: { purpose: 'clarify_request', question_characters: 45 },
        },
      ],
      [
        7100,
        'audio.requested',
        { text: 'Which account is this about, and who’s calling?', voice: 'preset', heard: false },
      ],
      [
        7110,
        'agent.tool.result',
        {
          name: 'ask_caller',
          generation_id: 2,
          turn_version: 1,
          ok: true,
          result: { audio_requested: true, stop: true },
        },
      ],
      [11000, 'relay.ended', { duration_seconds: 11, status: 'ended' }],
      [11200, 'call.ended', {}, 'caller'],
    ],
  });
  sessions.push({
    id: 'demo-call-unclear',
    household_id: 'sarah',
    profile: SARAH,
    state: 'ended',
    policy_epoch: 0,
    generation_id: 2,
    speech_pending: false,
    inbound_ended: true,
    closing_reason: null,
    current_action: 'Call ended',
    evidence_quote: 'it’s about the account',
    turn_version: 1,
    assessed_version: 1,
    blocked: false,
    ambiguous: true,
    assessments: [
      {
        turn_version: 1,
        generation_id: 2,
        observation: obs('unclear', 'unknown', 'unclear') as never,
        evidence_refs: [{ turn_id: t3, quote: 'it’s about the account' }],
      },
    ],
    understanding: {
      generation_id: 2,
      turn_version: 1,
      policy_epoch: 0,
      understanding: {
        version: 1,
        claimed_identity: null,
        purpose: 'account matter',
        requested_action: 'speak with Sarah',
        supplied_facts: [],
        missing_information: ['which account', 'who is calling'],
        readiness: 'needs_information',
      },
    },
    message: null,
    turns: [
      {
        id: t3,
        speaker: 'caller',
        text: 'Yeah, it’s about the account, I need to sort something out with her.',
      },
      {
        id: 'demo-a-4',
        speaker: 'assistant',
        text: 'Which account is this about, and who’s calling?',
      },
    ],
    offer: null,
    legs: [],
    execution_mode: 'replay',
    data_origin: DEMO_ORIGIN,
  });

  // 4. Maya: gift-card request matched a shared tactic. Blocked.
  const t4 = 'demo-turn-4a';
  scripts.push({
    id: 'demo-call-giftcard',
    minutesAgo: 1440 + 120,
    profile: MAYA,
    steps: [
      [
        0,
        'call.inbound.received',
        { caller_identity: 'unverified', transport: 'relay', requested_profile: MAYA },
        'caller',
      ],
      [
        6800,
        'transcript.final',
        {
          text: 'This is the tax office. To avoid a penalty today, buy two gift cards and read me the numbers.',
          turn_id: t4,
        },
      ],
      [
        8000,
        'model.request.completed',
        { generation_id: 5, turn_version: 1, ...model(1201, 219, 'stop') },
      ],
      [
        8010,
        'agent.tool.proposed',
        {
          name: 'record_assessment',
          generation_id: 5,
          turn_version: 1,
          arguments: {
            observation: obs('request', 'payment_diversion', 'caller_requests_from_sarah'),
          },
        },
      ],
      [
        8020,
        'policy.denied',
        {
          operation: 'assessment',
          observation: obs('request', 'payment_diversion', 'caller_requests_from_sarah'),
          quote: 'buy two gift cards and read me the numbers',
          handoff_allowed: false,
        },
      ],
      [
        8030,
        'agent.tool.result',
        {
          name: 'record_assessment',
          generation_id: 5,
          turn_version: 1,
          ok: true,
          result: { blocked: true, ambiguous: false, assessed_version: 1 },
        },
      ],
      [8600, 'model.request.completed', { generation_id: 5, turn_version: 1, ...model(571, 104) }],
      [
        8610,
        'agent.tool.proposed',
        { name: 'lookup_tactics', generation_id: 5, turn_version: 1, arguments: {} },
      ],
      [
        8620,
        'agent.tool.result',
        { name: 'lookup_tactics', generation_id: 5, turn_version: 1, ok: true, result: {} },
      ],
      [9200, 'model.request.completed', { generation_id: 5, turn_version: 1, ...model(594, 110) }],
      [
        9210,
        'agent.tool.proposed',
        {
          name: 'take_message',
          generation_id: 5,
          turn_version: 1,
          arguments: { summary_characters: 64 },
        },
      ],
      [
        9220,
        'message.recorded',
        { recorded_at: iso(1560, now, 9220), turn_version: 1, evidence_turn_ids: [t4] },
      ],
      [
        9230,
        'audio.requested',
        {
          text: 'I can’t help with payments. I’ve saved a note for Maya. Goodbye.',
          transport: 'twiml_closing',
          voice: 'preset',
          heard: false,
        },
      ],
      [
        9240,
        'agent.tool.result',
        {
          name: 'take_message',
          generation_id: 5,
          turn_version: 1,
          ok: true,
          result: { message_recorded: true, closing: true, stop: true },
        },
      ],
      [14000, 'relay.ended', { duration_seconds: 14, status: 'ended' }],
      [14300, 'call.ended', {}, 'caller'],
    ],
  });
  sessions.push({
    id: 'demo-call-giftcard',
    household_id: 'maya',
    profile: MAYA,
    state: 'ended',
    policy_epoch: 1,
    generation_id: 6,
    speech_pending: false,
    inbound_ended: true,
    closing_reason: 'message_saved',
    current_action: 'Call ended',
    evidence_quote: 'buy two gift cards and read me the numbers',
    turn_version: 1,
    assessed_version: 1,
    blocked: true,
    ambiguous: false,
    assessments: [
      {
        turn_version: 1,
        generation_id: 5,
        observation: obs('request', 'payment_diversion', 'caller_requests_from_sarah') as never,
        evidence_refs: [{ turn_id: t4, quote: 'buy two gift cards and read me the numbers' }],
      },
    ],
    understanding: null,
    message: {
      summary: 'Suspicious payment request blocked; call the agency at a number you already know.',
      recorded_at: iso(1560, now, 9220),
      turn_version: 1,
      evidence_turn_ids: [t4],
    },
    turns: [
      {
        id: t4,
        speaker: 'caller',
        text: 'This is the tax office. To avoid a penalty today, buy two gift cards and read me the numbers.',
      },
      {
        id: 'demo-a-5',
        speaker: 'assistant',
        text: 'I can’t help with payments. I’ve saved a note for Maya. Goodbye.',
      },
    ],
    offer: null,
    legs: [],
    execution_mode: 'replay',
    data_origin: DEMO_ORIGIN,
  });

  // 5. Model dependency failure: incomplete, recovered safely.
  scripts.push({
    id: 'demo-call-failure',
    minutesAgo: 1440 * 2 + 30,
    profile: SARAH,
    steps: [
      [
        0,
        'call.inbound.received',
        { caller_identity: 'unverified', transport: 'relay', requested_profile: SARAH },
        'caller',
      ],
      [
        5600,
        'transcript.final',
        { text: 'Hello? Is anyone there? I’m returning a call.', turn_id: 'demo-turn-5a' },
      ],
      [
        9100,
        'model.request.completed',
        {
          generation_id: 2,
          turn_version: 1,
          boundary: 'model_http',
          heard: false,
          observation: {
            backend: 'nvidia_hosted',
            requested_model: 'nvidia/nemotron-3.5-lightning-30b-a3b',
            response_model: null,
            elapsed_ms: 3500,
            completion_tokens: null,
            finish_reason: null,
            outcome: 'timeout',
            error: 'request exceeded the 3.5 s turn budget',
          },
        },
      ],
      [
        9110,
        'dependency.failed',
        { provider: 'nemotron', detail: 'Model did not answer in time; caller asked to call back' },
      ],
      [
        9120,
        'audio.requested',
        {
          text: 'Sorry, I can’t take a message right now. Please call back later. Goodbye.',
          transport: 'twiml_closing',
          voice: 'preset',
          heard: false,
        },
      ],
      [13500, 'call.ended', {}, 'caller'],
    ],
  });
  sessions.push({
    id: 'demo-call-failure',
    household_id: 'sarah',
    profile: SARAH,
    state: 'ended',
    policy_epoch: 0,
    generation_id: 3,
    speech_pending: false,
    inbound_ended: true,
    closing_reason: 'dependency_failed',
    current_action: 'Call ended',
    evidence_quote: null,
    turn_version: 1,
    assessed_version: 0,
    blocked: false,
    ambiguous: false,
    assessments: [],
    understanding: null,
    message: null,
    turns: [
      {
        id: 'demo-turn-5a',
        speaker: 'caller',
        text: 'Hello? Is anyone there? I’m returning a call.',
      },
      {
        id: 'demo-a-6',
        speaker: 'assistant',
        text: 'Sorry, I can’t take a message right now. Please call back later. Goodbye.',
      },
    ],
    offer: null,
    legs: [],
    execution_mode: 'replay',
    data_origin: DEMO_ORIGIN,
  });

  const allEvents = scripts.flatMap((script) => events(script, now));
  const capsuleExpiry = new Date(now + 20 * 3600000).toISOString();
  allEvents.push(
    {
      id: 'demo-capsule-1',
      sequence: ++seq,
      session_id: null,
      leg_role: 'system',
      occurred_at: iso(1555, now),
      received_at: iso(1555, now),
      policy_epoch: 0,
      execution_mode: 'replay',
      data_origin: DEMO_ORIGIN,
      type: 'tactic.created',
      payload: { id: 'demo-tactic-giftcard', version: 1 },
    },
    {
      id: 'demo-applied-1',
      sequence: ++seq,
      session_id: null,
      leg_role: 'system',
      occurred_at: iso(1554, now),
      received_at: iso(1554, now),
      policy_epoch: 0,
      execution_mode: 'replay',
      data_origin: DEMO_ORIGIN,
      type: 'community.applied',
      payload: { household_id: 'sarah', version: 2, capsule_id: 'demo-tactic-code' },
    },
  );
  allEvents.sort((a, b) => a.sequence - b.sequence);

  const state: ConsoleState = {
    execution_mode: 'replay',
    data_origin: DEMO_ORIGIN,
    transport: 'relay',
    phone_display: 'Demo (no phone line)',
    selected_household: 'sarah',
    profile: SARAH,
    model_backend: {
      backend: 'brev_hosted',
      label: 'Demo data: no model is called',
      requested_model: 'nvidia/nemotron-3.5-lightning-30b-a3b',
      configured: false,
      protocol_verified: false,
      detail: 'This demo view replays authored receipts. It never contacts a model.',
    },
    speech: {
      source: 'configured',
      transport: 'relay',
      provider: 'ElevenLabs',
      voice: 'cgSgspJ2msm6clMCkdW9-v4',
      voice_label: 'Jessica · warm, conversational',
      closing_provider: 'ElevenLabs',
      closing_voice: 'cgSgspJ2msm6clMCkdW9-v4',
      transcription_provider: 'Deepgram',
      transcription_model: 'flux',
    },
    evidence: {
      model_protocol: { status: 'pending', detail: 'Demo data has no protocol evidence.' },
      inbound_phone: { status: 'pending', detail: 'Demo data has no phone evidence.' },
      relay_phone: { status: 'pending', detail: 'Demo data has no phone evidence.' },
    },
    capabilities: [
      { id: 'inbound', status: 'disabled', detail: 'Demo view: no phone line is connected' },
      { id: 'nemotron', status: 'disabled', detail: 'Demo view: no model is called' },
      {
        id: 'voice_clone',
        status: 'disabled',
        detail: 'Preset assistant voice; no clone endpoint verified',
      },
    ],
    sessions,
    community: [
      { household_id: 'sarah', display_name: 'Sarah', last_applied_version: 2 },
      { household_id: 'maya', display_name: 'Maya', last_applied_version: 2 },
    ],
    tactics: [
      {
        id: 'demo-tactic-code',
        version: 1,
        tactic: 'authentication_code_extraction',
        requested_action: 'share_authentication_secret',
        pretext_category: 'clinic',
        pressure_category: 'urgency',
        recommended_action: 'independent_contact',
        provenance: 'demo_fixture',
        ttl_hours: 24,
        expires_at: capsuleExpiry,
      },
      {
        id: 'demo-tactic-giftcard',
        version: 2,
        tactic: 'payment_diversion',
        requested_action: 'send_payment',
        pretext_category: 'government',
        pressure_category: 'threat',
        recommended_action: 'independent_contact',
        provenance: 'demo_fixture',
        ttl_hours: 24,
        expires_at: capsuleExpiry,
      },
    ],
  };
  return { state, events: allEvents };
}
