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
      screen.getByRole("group", { name: "Live Relay conversation" }),
    ).getByText("Pending"),
  ).toBeInTheDocument();
  expect(
    screen.queryByText("Provider receipts available"),
  ).not.toBeInTheDocument();
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
