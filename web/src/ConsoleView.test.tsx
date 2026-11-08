import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { ConsoleView } from "./ConsoleView";
import { stateSchema, type CallEvent, type ConsoleState } from "./types";
const state: ConsoleState = {
  execution_mode: "replay",
  data_origin: "synthetic",
  transport: "relay",
  phone_display: "Not configured",
  selected_household: "sarah",
  capabilities: [
    { id: "telephony", status: "pending", detail: "No credentials configured" },
  ],
  sessions: [
    {
      id: "s1",
      household_id: "sarah",
      state: "screening",
      policy_epoch: 2,
      generation_id: 3,
      speech_pending: false,
      inbound_ended: false,
      current_action: "Offer withdrawn. Calling a trusted contact.",
      evidence_quote: "read the digits",
      turns: [
        {
          id: "t1",
          speaker: "caller",
          text: "<script>secret()</script>read the digits",
        },
      ],
      offer: {
        id: "o1",
        summary: "Private appointment summary",
        status: "revoked",
        policy_epoch: 1,
        expires_at: "2026-11-08T00:00:00Z",
      },
      legs: [
        {
          action_id: "a1",
          role: "family",
          status: "in-progress",
          acknowledged: false,
        },
      ],
      execution_mode: "replay",
      data_origin: "synthetic",
    },
  ],
  community: [
    { household_id: "sarah", display_name: "Sarah", last_applied_version: 1 },
    { household_id: "maya", display_name: "Maya", last_applied_version: 0 },
  ],
  tactics: [
    {
      id: "c1",
      version: 1,
      tactic: "authentication_code_extraction",
      requested_action: "relay_sign_in_text_digits",
      pretext_category: "clinic_appointment",
      pressure_category: "cancellation",
      expires_at: "2099-11-08T00:00:00Z",
    },
  ],
};
const measured: CallEvent = {
  id: "model-observation",
  sequence: 8,
  session_id: "s1",
  leg_role: "system",
  occurred_at: "2026-11-08T03:54:00Z",
  received_at: "2026-11-08T03:54:00Z",
  policy_epoch: 2,
  execution_mode: "replay",
  data_origin: "synthetic",
  type: "model.request.completed",
  payload: {
    generation_id: 3,
    turn_version: 1,
    boundary: "model_http",
    heard: false,
    observation: {
      requested_model: "nvidia/nemotron-3.5-lightning-30b-a3b",
      response_model: "nvidia/nemotron-3.5-lightning-30b-a3b",
      elapsed_ms: 998,
      outcome: "success",
      error: null,
    },
  },
};
it("shows the captured call profile separately from settings for the next household", () => {
  const captured = {
    household_id: "sarah" as const,
    version: 2,
    display_name: "Jamie",
    greeting: "Hi, what is the call about?",
    tts_provider: "ElevenLabs" as const,
    voice_id: "cgSgspJ2msm6clMCkdW9-v4",
    locale: "en-US" as const,
    timezone: "America/Chicago",
  };
  render(
    <ConsoleView
      state={{
        ...state,
        selected_household: "maya",
        sessions: [{ ...state.sessions[0], profile: captured }],
      }}
      events={[]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  expect(
    screen.getByRole("heading", { name: "Jamie’s call space" }),
  ).toBeInTheDocument();
  expect(screen.getByText(/Captured profile v2/)).toHaveTextContent(
    "listening proof is separate",
  );
});
it.each([false, true])(
  "keeps selected Brev protocol proof separate from historical NVIDIA proof: %s",
  (verified) => {
    const snapshot = stateSchema.parse({
      ...state,
      model_backend: {
        backend: "brev_hosted",
        label: "Brev-hosted Nemotron",
        requested_model: "nvidia/Nemotron-3-Nano-30B-A3B-FP8",
        configured: true,
        protocol_verified: verified,
        detail: "This deployment requires its own native tool-cycle receipt.",
      },
      evidence: {
        model_protocol: {
          status: "passed",
          source: "hosted_model_probe",
          detail: "Recorded hosted NVIDIA native tool/result probe passed.",
        },
        inbound_phone: { status: "pending", detail: "Pending" },
        relay_phone: { status: "pending", detail: "Pending" },
      },
    });
    render(
      <ConsoleView
        state={snapshot}
        events={[]}
        stale={false}
        busy={null}
        onAction={vi.fn()}
      />,
    );
    const selected = screen.getByRole("group", {
      name: "Current model backend",
    });
    expect(selected).toHaveTextContent("Brev-hosted Nemotron");
    expect(selected).toHaveTextContent("nvidia/Nemotron-3-Nano-30B-A3B-FP8");
    expect(selected).toHaveTextContent("Configuration: Configured");
    expect(selected).toHaveTextContent(
      verified
        ? "Native protocol: Verified for this backend"
        : "Native protocol: Pending for this backend",
    );
    expect(
      screen.getByRole("group", { name: "Model protocol" }),
    ).toHaveTextContent("Recorded pass");
  },
);
it("explains message-taking when the live household endpoint is unavailable", () => {
  const snapshot: ConsoleState = {
    ...state,
    execution_mode: "live",
    phone_b: {
      enrolled: false,
      enabled: false,
      authorized: false,
      remaining_attempts: 1,
      can_request: false,
      reason: "Household endpoint is not enrolled.",
    },
    sessions: [
      {
        ...state.sessions[0],
        execution_mode: "live",
        current_action: "Screening caller",
        offer: null,
        legs: [],
      },
    ],
  };
  render(
    <ConsoleView
      state={snapshot}
      events={[]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  expect(
    screen.getByRole("region", { name: "Screening caller" }),
  ).toHaveTextContent(
    "Screening and private message-taking; household connection unavailable.",
  );
  expect(screen.getByRole("button", { name: /Phone B/i })).toBeDisabled();
});
it.each([undefined, "nvidia_hosted", "brev_hosted"])(
  "attributes HTTP evidence only to its own recorded backend: %s",
  (backend) => {
    const snapshot = stateSchema.parse({
      ...state,
      model_backend: {
        backend: "brev_hosted",
        label: "Brev-hosted Nemotron",
        requested_model: "nvidia/Nemotron-3-Nano-30B-A3B-FP8",
        configured: false,
        protocol_verified: false,
        detail: "Configuration incomplete.",
      },
    });
    const event = {
      ...measured,
      payload: {
        ...measured.payload,
        observation: {
          ...(measured.payload.observation as Record<string, unknown>),
          ...(backend ? { backend } : {}),
        },
      },
    };
    render(
      <ConsoleView
        state={snapshot}
        events={[event]}
        stale={false}
        busy={null}
        onAction={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("group", { name: "Current model backend" }),
    ).toHaveTextContent("Configuration: Incomplete");
    const observation = screen.getByRole("group", {
      name: "Model HTTP observation",
    });
    expect(observation).toHaveTextContent(
      backend === "brev_hosted"
        ? "Backend: Brev hosted"
        : backend === "nvidia_hosted"
          ? "Backend: NVIDIA hosted"
          : "Backend: Not recorded",
    );
    if (backend !== "brev_hosted")
      expect(observation).not.toHaveTextContent("Backend: Brev hosted");
    if (backend === "nvidia_hosted")
      expect(observation).toHaveTextContent(
        "Recorded backend differs from the selected backend",
      );
    expect(observation).toHaveTextContent("audible response unverified");
  },
);
function understoodState() {
  return stateSchema.parse({
    ...state,
    sessions: [
      {
        ...state.sessions[0],
        offer: null,
        legs: [],
        turn_version: 2,
        assessed_version: 2,
        turns: [
          {
            id: "opening",
            speaker: "caller",
            text: "I’m Alex, Sarah’s son. I’m calling about picking me up.",
          },
          {
            id: "details",
            speaker: "caller",
            text: "My train arrives at eight thirty. Please pick me up at the north entrance.",
          },
        ],
        understanding: {
          generation_id: 3,
          turn_version: 2,
          policy_epoch: 2,
          understanding: {
            version: 1,
            claimed_identity: "Alex, Sarah’s son",
            purpose: "Arrange a pickup",
            requested_action: "Tell Sarah the pickup details",
            supplied_facts: [
              {
                text: "eight thirty",
                evidence_refs: [
                  {
                    turn_id: "details",
                    quote: "My train arrives at eight thirty.",
                  },
                ],
              },
              {
                text: "north entrance",
                evidence_refs: [
                  {
                    turn_id: "details",
                    quote: "Please pick me up at the north entrance.",
                  },
                ],
              },
            ],
            missing_information: [],
            readiness: "ready",
          },
        },
      },
    ],
  });
}
it("shows accepted purpose and grounded supplied facts without verifying a claimed identity or executing the request", () => {
  render(
    <ConsoleView
      state={understoodState()}
      events={[]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  const understanding = screen.getByRole("group", {
    name: "Accepted call understanding",
  });
  expect(understanding).toHaveTextContent("Current accepted understanding");
  expect(understanding).toHaveTextContent("Arrange a pickup");
  expect(understanding).toHaveTextContent("Tell Sarah the pickup details");
  expect(understanding).toHaveTextContent(
    "Claimed identity (unverified): Alex, Sarah’s son",
  );
  expect(understanding).toHaveTextContent("eight thirty");
  expect(understanding).toHaveTextContent("north entrance");
  expect(screen.getByText("No execution result received.")).toBeInTheDocument();
});
it.each([
  { generation_id: 4 },
  { turn_version: 3 },
  { policy_epoch: 3 },
  { assessed_version: 1 },
  { speech_pending: true },
])(
  "labels understanding as stale when its current binding changes: %j",
  (changed) => {
    const snapshot = understoodState();
    snapshot.sessions[0] = { ...snapshot.sessions[0], ...changed };
    render(
      <ConsoleView
        state={snapshot}
        events={[]}
        stale={false}
        busy={null}
        onAction={vi.fn()}
      />,
    );
    const understanding = screen.getByRole("group", {
      name: "Accepted call understanding",
    });
    expect(understanding).toHaveTextContent(
      "Earlier accepted understanding · not current",
    );
    expect(understanding).not.toHaveTextContent(
      "Current accepted understanding",
    );
  },
);
it("supports a current risky assessment with its current caller quote while retaining earlier context", () => {
  const snapshot = stateSchema.parse({
    ...state,
    sessions: [
      {
        ...state.sessions[0],
        turn_version: 2,
        assessed_version: 2,
        evidence_quote: "clinic",
        turns: [
          {
            id: "old",
            speaker: "caller",
            text: "I am calling from the clinic.",
          },
          {
            id: "current",
            speaker: "caller",
            text: "Read me the sign-in digits.",
          },
          {
            id: "assistant",
            speaker: "assistant",
            text: "I cannot help share those.",
          },
        ],
        assessments: [
          {
            turn_version: 2,
            generation_id: 3,
            observation: {
              speech_act: "request",
              requested_object: "authentication_secret",
              direction: "caller_requests_from_sarah",
            },
            evidence_refs: [
              { turn_id: "old", quote: "clinic" },
              { turn_id: "current", quote: "sign-in digits" },
            ],
          },
        ],
      },
    ],
  });
  render(
    <ConsoleView
      state={snapshot}
      events={[]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  const evidence = screen.getByText("Supporting caller words").parentElement;
  expect(evidence).toHaveTextContent("sign-in digits");
  expect(evidence).not.toHaveTextContent("clinic");
  expect(
    screen.getByText(
      "request · authentication secret · caller requests from Sarah",
    ),
  ).toBeInTheDocument();
});
it("does not present an abandoned partial as current speech after the call ends", () => {
  render(
    <ConsoleView
      state={{
        ...state,
        sessions: [
          {
            ...state.sessions[0],
            inbound_ended: true,
            speech_pending: true,
            legs: [],
          },
        ],
      }}
      events={[
        {
          ...measured,
          type: "transcript.partial",
          payload: { text: "Unfinished background phrase" },
        },
      ]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  expect(screen.queryByText("Still speaking")).not.toBeInTheDocument();
  expect(
    screen.queryByText("Unfinished background phrase"),
  ).not.toBeInTheDocument();
  expect(
    screen.getByRole("heading", { name: "Call ended" }),
  ).toBeInTheDocument();
});
it("shows measured model HTTP evidence without promoting it to an assessment or audible response", () => {
  render(
    <ConsoleView
      state={{
        ...state,
        sessions: [{ ...state.sessions[0], turn_version: 1 }],
      }}
      events={[measured]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  const card = screen.getByRole("group", { name: "Model HTTP observation" });
  expect(card).toHaveTextContent("nvidia/nemotron-3.5-lightning-30b-a3b");
  expect(card).toHaveTextContent("998 ms");
  expect(card).toHaveTextContent("success");
  expect(card).toHaveTextContent("Turn 1 · generation 3");
  expect(card).toHaveTextContent("HTTP timing; audible response unverified");
  expect(
    screen.getByText("No accepted assessment for the current turn."),
  ).toBeInTheDocument();
  expect(screen.getByText("No execution result received.")).toBeInTheDocument();
});
it.each([true, false])(
  "marks only a later cancellation of the measured generation: matched=%s",
  (matched) => {
    const cancellation = {
      ...measured,
      id: "cancel",
      sequence: 9,
      type: "model.cancelled",
      payload: { generation_id: matched ? 3 : 4, reason: "caller_speech" },
    };
    render(
      <ConsoleView
        state={{
          ...state,
          sessions: [
            { ...state.sessions[0], generation_id: 5, turn_version: 2 },
          ],
        }}
        events={[cancellation, measured]}
        stale={false}
        busy={null}
        onAction={vi.fn()}
      />,
    );
    const card = screen.getByRole("group", { name: "Model HTTP observation" });
    expect(card).toHaveTextContent("Earlier turn or generation");
    expect(card.textContent?.includes("Model job later cancelled")).toBe(
      matched,
    );
    expect(
      screen.getByText("No accepted assessment for the current turn."),
    ).toBeInTheDocument();
  },
);
it("does not fabricate HTTP timing from a cancelled job or another session's observation", () => {
  const cancellation = {
    ...measured,
    id: "cancel",
    sequence: 9,
    type: "model.cancelled",
    payload: { generation_id: 3, reason: "caller_speech" },
  };
  render(
    <ConsoleView
      state={state}
      events={[{ ...measured, session_id: "other" }, cancellation]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  const card = screen.getByRole("group", { name: "Model HTTP observation" });
  expect(card).toHaveTextContent("Model job cancelled · generation 3");
  expect(card).toHaveTextContent("No completed HTTP observation received");
  expect(card).not.toHaveTextContent("998");
  expect(card).not.toHaveTextContent("nvidia/");
});
it("keeps replay, answer, withdrawn consent and unapplied memory explicit", () => {
  render(
    <ConsoleView
      state={state}
      events={[]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  expect(screen.getAllByText(/replay/i).length).toBeGreaterThan(0);
  expect(
    screen.getByRole("heading", {
      name: "Offer withdrawn. Calling a trusted contact.",
    }),
  ).toBeInTheDocument();
  expect(screen.getByText("Answered", { selector: "p" })).toBeInTheDocument();
  expect(screen.queryByText("Acknowledged by keypad")).not.toBeInTheDocument();
  expect(screen.getByText("Offer withdrawn")).toBeInTheDocument();
  expect(screen.getByText("No applied update")).toBeInTheDocument();
  expect(
    screen.getByText("<script>secret()</script>read the digits"),
  ).toBeInTheDocument();
  expect(document.querySelector("script")).toBeNull();
});
it("sends only an enrolled role for Phone B and never offers replay controls in live mode", () => {
  const onAction = vi.fn().mockResolvedValue(undefined);
  render(
    <ConsoleView
      state={{
        ...state,
        execution_mode: "live",
        phone_b: {
          enrolled: true,
          enabled: true,
          authorized: true,
          remaining_attempts: 1,
          can_request: true,
          reason: "Ready for the designated test.",
        },
        sessions: state.sessions.map((session) => ({
          ...session,
          execution_mode: "live",
        })),
      }}
      events={[]}
      stale={false}
      busy={null}
      onAction={onAction}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Call Phone B" }));
  expect(onAction).toHaveBeenCalledWith("/api/demo/probe", {
    session_id: "s1",
    contact_id: "sarah",
  });
  expect(
    screen.queryByRole("button", { name: "Replay revocation" }),
  ).not.toBeInTheDocument();
});

it("cannot request a live probe for a historical replay session", () => {
  render(
    <ConsoleView
      state={{ ...state, execution_mode: "live" }}
      events={[]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  expect(screen.getByRole("button", { name: "Call Phone B" })).toBeDisabled();
});

it("keeps Phone B disabled when enrollment and remaining-attempt evidence are absent", () => {
  const onAction = vi.fn();
  render(
    <ConsoleView
      state={{
        ...state,
        execution_mode: "live",
        sessions: state.sessions.map((session) => ({
          ...session,
          execution_mode: "live",
        })),
      }}
      events={[]}
      stale={false}
      busy={null}
      onAction={onAction}
    />,
  );
  const button = screen.getByRole("button", { name: "Call Phone B" });
  expect(button).toBeDisabled();
  fireEvent.click(button);
  expect(onAction).not.toHaveBeenCalled();
});

it("separates recorded protocol and inbound proof from pending live Relay", () => {
  const proofState = stateSchema.parse({
    ...state,
    execution_mode: "live",
    capabilities: [
      {
        id: "relay",
        status: "configured",
        detail: "Local WebSocket inputs set",
      },
    ],
    evidence: {
      model_protocol: {
        status: "passed",
        detail: "Four native requests completed",
        recorded_at: "2026-11-08T02:04:57Z",
      },
      inbound_phone: {
        status: "passed",
        detail: "Signed inbound call and user-heard audio",
      },
      relay_phone: {
        status: "pending",
        detail: "No live Relay conversation verified",
      },
    },
  });
  render(
    <ConsoleView
      state={proofState}
      events={[]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  expect(
    within(screen.getByRole("group", { name: "Model protocol" })).getByText(
      "Recorded pass",
    ),
  ).toBeInTheDocument();
  expect(
    within(screen.getByRole("group", { name: "Real inbound phone" })).getByText(
      "Recorded pass",
    ),
  ).toBeInTheDocument();
  expect(
    within(
      screen.getByRole("group", { name: "Full Relay rehearsal" }),
    ).getByText("Pending"),
  ).toBeInTheDocument();
  expect(
    screen.queryByText("Provider receipts available"),
  ).not.toBeInTheDocument();
});

it("labels speech choices as current configuration without promoting them to observed phone proof", () => {
  const configured = stateSchema.parse({
    ...state,
    speech: {
      provider: "ElevenLabs",
      voice: "21m00Tcm4TlvDq8ikWAM-v4",
      voice_label: "Rachel v4",
      source: "configured",
      transport: "relay",
      closing_provider: "Amazon Polly",
      closing_voice: "Joanna-Neural",
      transcription_provider: "Deepgram",
      transcription_model: "nova-3-general",
    },
  });
  render(
    <ConsoleView
      state={configured}
      events={[]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  const speech = screen.getByRole("group", {
    name: "Current speech configuration",
  });
  expect(speech).toHaveTextContent("ElevenLabs · Rachel v4");
  expect(speech).toHaveTextContent("Deepgram · nova-3-general");
  expect(speech).toHaveTextContent("Amazon Polly · Joanna-Neural");
  expect(speech).toHaveTextContent("Configuration only");
  expect(
    within(
      screen.getByRole("group", { name: "Full Relay rehearsal" }),
    ).getByText("Pending"),
  ).toBeInTheDocument();
  expect(
    screen.getByText("Awaiting a successful Rust result."),
  ).toBeInTheDocument();
});

it.each([
  { enrolled: false },
  { enabled: false },
  { authorized: false },
  { remaining_attempts: 0 },
  { can_request: false },
])("holds Phone B when a required gate is missing: %j", (missing) => {
  const onAction = vi.fn();
  const snapshot = stateSchema.parse({
    ...state,
    execution_mode: "live",
    sessions: state.sessions.map((session) => ({
      ...session,
      execution_mode: "live",
    })),
    phone_b: {
      enrolled: true,
      enabled: true,
      authorized: true,
      remaining_attempts: 1,
      can_request: true,
      reason: "Calling unavailable",
      ...missing,
    },
  });
  render(
    <ConsoleView
      state={snapshot}
      events={[]}
      stale={false}
      busy={null}
      onAction={onAction}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Call Phone B" }));
  expect(screen.getByRole("button", { name: "Call Phone B" })).toBeDisabled();
  expect(onAction).not.toHaveBeenCalled();
});

it("shows a persisted message only in the private view without claiming it was delivered", () => {
  const messageState = stateSchema.parse({
    ...state,
    sessions: [
      {
        ...state.sessions[0],
        offer: null,
        legs: [],
        message: {
          summary: "Caller asks to reschedule the clinic visit.",
          recorded_at: "2026-11-08T00:00:00Z",
          turn_version: 1,
          evidence_turn_ids: ["t1"],
        },
      },
    ],
  });
  render(
    <ConsoleView
      state={messageState}
      events={[]}
      stale={false}
      busy={null}
      onAction={vi.fn()}
    />,
  );
  const heading = screen.getByRole("heading", {
    name: "Saved message for Sarah",
  });
  expect(heading.closest(".private-panel")).toHaveTextContent(
    "Caller asks to reschedule the clinic visit.",
  );
  expect(
    screen.getByText(
      "Caller identity unverified. Saved for review; not a delivery receipt.",
    ),
  ).toBeInTheDocument();
  expect(screen.queryByText("Acknowledged by keypad")).not.toBeInTheDocument();
  expect(document.querySelector(".community-strip")).not.toHaveTextContent(
    "reschedule",
  );
});

it.each([false, true])(
  "distinguishes saved-message closing from a received call end: ended=%s",
  (ended) => {
    const snapshot = stateSchema.parse({
      ...state,
      sessions: [
        {
          ...state.sessions[0],
          state: "closing",
          offer: null,
          legs: [],
          speech_pending: true,
          inbound_ended: ended,
          closing_reason: "message_saved",
          message: {
            summary: "Caller left a message",
            recorded_at: "2026-11-08T03:00:00Z",
            turn_version: 1,
            evidence_turn_ids: ["t1"],
          },
        },
      ],
    });
    render(
      <ConsoleView
        state={snapshot}
        events={[]}
        stale={false}
        busy={null}
        onAction={vi.fn()}
      />,
    );
    expect(
      screen.getByText(
        ended
          ? "A private message is saved and the call has ended. Goodbye playback remains unverified."
          : "A private message is saved. Goodbye delivery and call completion still need their own receipts.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(
        "New caller speech is being assessed. A pending choice cannot connect yet.",
      ),
    ).not.toBeInTheDocument();
  },
);
