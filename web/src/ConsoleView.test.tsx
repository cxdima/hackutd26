import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { ConsoleView } from "./ConsoleView";
import { stateSchema, type ConsoleState } from "./types";
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
