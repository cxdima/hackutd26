import { matchSchema } from "./types";
import type { CallEvent, Leg, Session, TacticMatch } from "./types";
import { z } from "zod";

const httpObservation = z.object({
  generation_id: z.number().int().nonnegative(),
  turn_version: z.number().int().nonnegative(),
  boundary: z.literal("model_http"),
  heard: z.literal(false),
  observation: z.object({
    response_model: z.string().max(200).nullable(),
    elapsed_ms: z.number().int().nonnegative(),
    outcome: z.enum([
      "success",
      "timeout",
      "authentication",
      "authorization",
      "rate_limit_or_quota",
      "request_schema",
      "model_or_endpoint",
      "provider",
      "transport",
      "configuration",
      "schema",
    ]),
  }),
});

export function modelHttpEvidence(session: Session, events: CallEvent[]) {
  const relevant = events
    .filter(
      (event) =>
        event.session_id === session.id &&
        event.execution_mode === session.execution_mode,
    )
    .sort((a, b) => a.sequence - b.sequence);
  const completed = relevant
    .flatMap((event) => {
      if (event.type !== "model.request.completed") return [];
      const parsed = httpObservation.safeParse(event.payload);
      return parsed.success
        ? [{ ...parsed.data, sequence: event.sequence }]
        : [];
    })
    .at(-1);
  const cancelled = relevant.findLast(
    (event) =>
      event.type === "model.cancelled" &&
      Number.isInteger(event.payload.generation_id) &&
      Number(event.payload.generation_id) >= 0 &&
      (!completed ||
        (event.sequence > completed.sequence &&
          event.payload.generation_id === completed.generation_id)),
  );
  return {
    completed,
    cancelledGeneration: cancelled
      ? Number(cancelled.payload.generation_id)
      : undefined,
    current:
      !!completed &&
      completed.generation_id === session.generation_id &&
      completed.turn_version === session.turn_version,
  };
}

export function decisionEvidence(session: Session, events: CallEvent[]) {
  const messageClosing =
    session.closing_reason === "message_saved" &&
    session.message != null &&
    session.turn_version !== undefined &&
    session.message?.turn_version === session.turn_version;
  const matchesGeneration = (generation: unknown) =>
    generation === session.generation_id ||
    (messageClosing &&
      typeof generation === "number" &&
      generation + 1 === session.generation_id);
  const assessment =
    (!session.speech_pending || messageClosing) &&
    session.turn_version !== undefined &&
    session.assessed_version === session.turn_version
      ? session.assessments?.findLast(
          (item) =>
            item.turn_version === session.turn_version &&
            matchesGeneration(item.generation_id) &&
            item.evidence_refs.some((reference) =>
              session.turns.some(
                (turn) =>
                  turn.speaker === "caller" &&
                  turn.id === reference.turn_id &&
                  reference.quote.length > 0 &&
                  turn.text.includes(reference.quote),
              ),
            ),
        )
      : undefined;
  const sameSession = events
    .filter(
      (event) =>
        event.session_id === session.id &&
        event.execution_mode === session.execution_mode,
    )
    .sort((a, b) => a.sequence - b.sequence);
  const current = sameSession.filter(
    (event) =>
      matchesGeneration(event.payload.generation_id) &&
      event.payload.turn_version === session.turn_version &&
      session.turn_version !== undefined,
  );
  const proposed = current.findLast(
    (event) =>
      event.type === "agent.tool.proposed" &&
      typeof event.payload.name === "string",
  );
  const legacy = proposed
    ? undefined
    : sameSession.findLast(
        (event) =>
          event.type === "agent.tool.proposed" &&
          typeof event.payload.name === "string" &&
          event.payload.generation_id === undefined &&
          event.payload.turn_version === undefined,
      );
  const proposal = proposed
    ? { name: String(proposed.payload.name), legacy: false }
    : legacy
      ? { name: String(legacy.payload.name), legacy: true }
      : undefined;
  const receipt =
    proposed &&
    current.findLast(
      (event) =>
        event.type === "agent.tool.result" &&
        event.sequence > proposed.sequence &&
        event.payload.generation_id === proposed.payload.generation_id &&
        event.payload.name === proposed.payload.name &&
        typeof event.payload.ok === "boolean",
    );
  let outcome = "No execution result received.";
  if (receipt?.payload.ok === false)
    outcome = "Rust reported a failure; check the action receipts.";
  if (receipt?.payload.ok === true) {
    const result = receipt.payload.result;
    const value =
      result && typeof result === "object" && !Array.isArray(result)
        ? (result as Record<string, unknown>)
        : {};
    if (value.message_recorded === true)
      outcome = "Private message saved for review.";
    else if (value.audio_requested === true)
      outcome = "Reply queued; caller playback is unverified.";
    else if (value.acknowledged === true)
      outcome = "Contact acknowledgement recorded.";
    else if (proposal?.name === "offer_handoff")
      outcome = "Private offer created; phone receipts are shown separately.";
    else if (proposal?.name === "start_trusted_callback")
      outcome =
        "Independent contact request accepted; no acknowledgement implied.";
    else if (proposal?.name === "record_assessment")
      outcome = "Assessment accepted by Rust.";
    else if (proposal?.name === "lookup_trusted_contact")
      outcome = "Enrolled directory lookup completed; no call requested.";
    else if (proposal?.name === "lookup_tactics")
      outcome = "Tactic lookup completed; matches require their own receipt.";
    else outcome = "Tool completed; no phone outcome implied.";
  }
  const acceptance =
    receipt?.payload.ok === true
      ? "accepted"
      : receipt?.payload.ok === false
        ? "failed"
        : "pending";
  return { assessment, proposal, outcome, acceptance };
}

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
