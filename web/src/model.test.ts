import { describe, expect, it } from "vitest";
import { confirmedMatches, legEvidence, legStatus, mergeEvents } from "./model";
import type { CallEvent, Leg } from "./types";
const leg: Leg = {
  action_id: "action-b",
  role: "family",
  status: "in-progress",
  acknowledged: false,
};
const event = (
  id: string,
  sequence: number,
  type = "outbound.ringing",
  action = "action-b",
): CallEvent => ({
  id,
  sequence,
  type,
  session_id: "s1",
  leg_role: "family",
  occurred_at: "2026-11-07T23:45:00Z",
  received_at: "2026-11-07T23:45:00Z",
  policy_epoch: 2,
  execution_mode: "replay",
  data_origin: "synthetic",
  payload: { action_id: action },
});
describe("actual phone progression", () => {
  it("does not turn an answer or an ended call into an acknowledgement", () => {
    expect(legStatus(leg)).toBe("Answered");
    expect(legStatus({ ...leg, status: "completed" })).toBe(
      "Ended · no acknowledgement",
    );
    expect(legStatus({ ...leg, acknowledged: true })).toBe(
      "Acknowledged by keypad",
    );
  });
  it("does not infer missing ringing or answer receipts from a terminal status", () => {
    expect(
      legEvidence({ ...leg, status: "completed" }, [
        event("1", 1, "outbound.requested"),
        event("2", 2, "outbound.answered", "other-leg"),
      ]),
    ).toEqual([true, false, false, false]);
    expect(legEvidence(leg, [event("1", 1)])).toEqual([
      true,
      true,
      true,
      false,
    ]);
  });
});
describe("reconnect event history", () => {
  it("deduplicates delivery and orders reordered receipts without changing their meaning", () => {
    const first = event("1", 1);
    const second = event("2", 2, "outbound.answered");
    expect(mergeEvents([second], first).map((e) => e.id)).toEqual(["1", "2"]);
    expect(mergeEvents([first, second], first)).toEqual([first, second]);
    expect(mergeEvents([first], second)[1].execution_mode).toBe("replay");
  });
});

it("does not confuse a private handoff choice with a contact acknowledgement", () => {
  expect(
    legStatus({ ...leg, role: "sarah", kind: "handoff", status: "completed" }),
  ).toBe("Call ended");
});

it("shows a tactic match only for this session and an applied, unexpired version in the same execution mode", () => {
  const match = {
    id: "capsule",
    version: 2,
    requested_action: "relay_sign_in_text_digits",
    pretext_category: "clinic_appointment",
    expires_at: "2026-11-09T00:00:00Z",
    household_applied_version: 2,
    matched_fields: ["requested_action", "pretext_category"],
  };
  const receipt = {
    ...event("match-event", 3, "tactic.matched"),
    payload: { matches: [match] },
  };
  const now = Date.parse("2026-11-08T00:00:00Z");
  expect(
    confirmedMatches([receipt], "maya", "s1", "replay", 2, now).map(
      (m) => m.id,
    ),
  ).toEqual(["capsule"]);
  expect(confirmedMatches([receipt], "maya", "s1", "replay", 1, now)).toEqual(
    [],
  );
  expect(
    confirmedMatches([receipt], "maya", "other", "replay", 2, now),
  ).toEqual([]);
  expect(confirmedMatches([receipt], "maya", "s1", "live", 2, now)).toEqual([]);
  expect(
    confirmedMatches(
      [receipt],
      "maya",
      "s1",
      "replay",
      2,
      Date.parse("2026-11-10T00:00:00Z"),
    ),
  ).toEqual([]);
});

it("groups distinct supporting capsules by action and context and keeps the latest version despite duplicate or reordered receipts", () => {
  const capsule = (
    id: string,
    version: number,
    pretext = "clinic_appointment",
  ) => ({
    id,
    version,
    requested_action: "relay_sign_in_text_digits",
    pretext_category: pretext,
    expires_at: "2026-11-09T00:00:00Z",
    household_applied_version: 8,
    matched_fields: ["requested_action", "pretext_category"],
  });
  const recent = {
    ...event("recent-match", 8, "tactic.matched"),
    payload: {
      matches: [
        capsule("c4", 4),
        capsule("c2", 2),
        capsule("c1", 1),
        capsule("c3", 3),
        capsule("bank", 5, "bank_support"),
        capsule("unapplied", 9),
      ],
    },
  };
  const earlier = {
    ...event("earlier-match", 7, "tactic.matched"),
    payload: { matches: [capsule("c1", 1), capsule("c2", 2)] },
  };
  const grouped = confirmedMatches(
    [recent, earlier, recent],
    "maya",
    "s1",
    "replay",
    8,
    Date.parse("2026-11-08T00:00:00Z"),
  );
  expect(grouped).toHaveLength(2);
  expect(
    grouped.find((match) => match.pretext_category === "clinic_appointment"),
  ).toMatchObject({ id: "c4", version: 4, supporting_count: 4 });
  expect(
    grouped.find((match) => match.pretext_category === "bank_support"),
  ).toMatchObject({ id: "bank", version: 5, supporting_count: 1 });
});
