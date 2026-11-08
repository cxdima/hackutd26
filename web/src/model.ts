import { matchSchema } from "./types";
import type { CallEvent, Leg, TacticMatch } from "./types";

export function legStatus(leg: Leg): string {
  if (leg.acknowledged) return "Acknowledged by keypad";
  if (leg.kind === "handoff" && leg.status === "completed") return "Call ended";
  const labels: Record<string, string> = {
    reserved: "Preparing the request",
    unknown: "Awaiting provider confirmation",
    queued: "Request accepted",
    initiated: "Dialing",
    ringing: "Ringing",
    "in-progress": "Answered",
    completed: "Ended · no acknowledgement",
    busy: "Line busy",
    failed: "Call failed",
    "no-answer": "No answer",
    canceled: "Canceled",
  };
  return labels[leg.status] ?? "Awaiting a status receipt";
}

export function mergeEvents(
  events: CallEvent[],
  incoming: CallEvent,
): CallEvent[] {
  if (events.some((event) => event.id === incoming.id)) return events;
  return [...events, incoming]
    .sort((a, b) => a.sequence - b.sequence)
    .slice(-200);
}

export function legEvidence(leg: Leg, events: CallEvent[]): boolean[] {
  const types = new Set(
    events
      .filter((event) => event.payload.action_id === leg.action_id)
      .map((event) => event.type),
  );
  return [
    types.has("outbound.requested") ||
      ["queued", "initiated", "ringing", "in-progress"].includes(leg.status),
    types.has("outbound.ringing") || leg.status === "ringing",
    types.has("outbound.answered") || leg.status === "in-progress",
    leg.acknowledged,
  ];
}

export function displayName(role: string) {
  return (
    (
      {
        sarah: "Sarah",
        maya: "Maya",
        family: "Trusted family contact",
        clinic: "Enrolled clinic",
        caller: "Caller",
        assistant: "FirstRing",
      } as Record<string, string>
    )[role] ?? role.replaceAll("_", " ")
  );
}
export function shortId(id: string) {
  return id.length > 12 ? `${id.slice(0, 6)}…${id.slice(-4)}` : id;
}
export function timeLabel(time: string) {
  const date = new Date(time);
  return Number.isNaN(date.getTime())
    ? "Time unavailable"
    : date.toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
}

export function confirmedMatches(
  events: CallEvent[],
  household: string,
  sessionId: string,
  mode: string,
  applied: number,
  now: number,
): (TacticMatch & { supporting_count: number })[] {
  if (!["sarah", "maya"].includes(household)) return [];
  const found = new Map<string, TacticMatch>();
  for (const event of events) {
    if (
      event.type !== "tactic.matched" ||
      event.session_id !== sessionId ||
      event.execution_mode !== mode ||
      !Array.isArray(event.payload.matches)
    )
      continue;
    for (const value of event.payload.matches) {
      const result = matchSchema.safeParse(value);
      if (!result.success) continue;
      const match = result.data;
      if (
        match.version <= applied &&
        match.version <= match.household_applied_version &&
        Date.parse(match.expires_at) > now &&
        new Set(match.matched_fields).size === 2
      )
        found.set(match.id, match);
    }
  }
  const groups = new Map<string, TacticMatch & { supporting_count: number }>();
  for (const match of found.values()) {
    const key = JSON.stringify([
      match.requested_action,
      match.pretext_category,
    ]);
    const group = groups.get(key);
    const latest = group && group.version > match.version ? group : match;
    groups.set(key, {
      ...latest,
      supporting_count: (group?.supporting_count ?? 0) + 1,
    });
  }
  return [...groups.values()].sort((a, b) => b.version - a.version);
}
