import { useEffect, useState } from "react";
import {
  ArrowDownLeft,
  Check,
  ChevronDown,
  Circle,
  Clock3,
  DoorOpen,
  Headphones,
  House,
  LockKeyhole,
  MessageCircle,
  Phone,
  PhoneCall,
  PhoneOff,
  Play,
  Radio,
  ShieldCheck,
  Square,
  Users,
  X,
} from "lucide-react";
import {
  confirmedMatches,
  displayName,
  legEvidence,
  legStatus,
  shortId,
  timeLabel,
} from "./model";
import type { CallEvent, ConsoleState, Leg } from "./types";

export interface ViewProps {
  state: ConsoleState;
  events: CallEvent[];
  stale: boolean;
  busy: string | null;
  onAction: (path: string, body: Record<string, unknown>) => Promise<void>;
}

const readable = (value: string) => value.replaceAll("_", " ");
const terminal = ["completed", "busy", "failed", "no-answer", "canceled"];
const eventNames: Record<string, string> = {
  "message.recorded": "Private message saved",
  "call.inbound.received": "Incoming call received",
  "voice.connected": "Conversation connected",
  "handoff.offered": "Private choice offered",
  "handoff.revoked": "Offer withdrawn",
  "handoff.choice": "Private keypad choice",
  "handoff.stale_rejected": "Old acceptance rejected",
  "outbound.requested": "Outbound request accepted",
  "outbound.ringing": "Phone ringing",
  "outbound.answered": "Phone answered",
  "contact.acknowledged": "Contact acknowledged",
  "bridge.connected": "Both participants connected",
  "tactic.created": "Tactic capsule saved",
  "tactic.matched": "Similar tactic retrieved",
  "community.applied": "Household applied update",
  "dependency.failed": "A dependency failed",
  "call.ended": "Call ended",
  "policy.allowed": "Action permitted",
  "policy.denied": "Action held",
  "audio.requested": "Assistant audio requested",
  "action.reserved": "Action reserved",
  "outbound.unknown": "Awaiting provider confirmation",
  "transport.keypad": "Keypad input received",
};

function PhoneLeg({
  leg,
  events,
  household,
}: {
  leg: Leg;
  events: CallEvent[];
  household: string;
}) {
  const observed = legEvidence(leg, events);
  const ended = terminal.includes(leg.status);
  return (
    <article className={`phone-leg ${leg.acknowledged ? "acknowledged" : ""}`}>
      <div className="leg-top">
        <span className="leg-icon">
          {leg.role === "family" ? <Users size={19} /> : <Phone size={19} />}
        </span>
        <div>
          <h4>
            {leg.role === "sarah"
              ? displayName(household)
              : displayName(leg.role)}
          </h4>
          <p>
            {leg.role === "sarah" ? "Private choice" : "Independent contact"}
          </p>
        </div>
        {leg.acknowledged && (
          <Check
            className="leg-check"
            size={21}
            aria-label="Keypad acknowledgement received"
          />
        )}
      </div>
      <p className={`leg-status ${ended && !leg.acknowledged ? "quiet" : ""}`}>
        {legStatus(leg)}
      </p>
      <ol
        className="leg-steps"
        aria-label={`${displayName(leg.role)} call receipts`}
      >
        {(leg.kind === "handoff"
          ? ["Requested", "Ringing", "Answered"]
          : ["Requested", "Ringing", "Answered", "Keypad 1"]
        ).map((label, i) => (
          <li
            key={label}
            className={observed[i] ? "observed" : ""}
            title={
              observed[i]
                ? `${label}: receipt observed`
                : `${label}: no receipt shown`
            }
          >
            {observed[i] ? <Check size={12} /> : <Circle size={10} />}
            <span>{label}</span>
          </li>
        ))}
      </ol>
      {leg.role !== "sarah" && (
        <p className="privacy-caption">
          <LockKeyhole size={12} /> The caller is not on this line.
        </p>
      )}
    </article>
  );
}

export function ConsoleView({
  state,
  events,
  stale,
  busy,
  onAction,
}: ViewProps) {
  const [selectedId, setSelectedId] = useState("");
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30000);
    return () => clearInterval(timer);
  }, []);
  const active =
    state.sessions.find((session) => session.id === selectedId) ??
    state.sessions[0];
  const sessionEvents = active
    ? events.filter((event) => event.session_id === active.id)
    : [];
  const replay = state.execution_mode === "replay";
  const sessionReplay = active?.execution_mode === "replay";
  const withdrawn = active?.offer?.status === "revoked";
  const name = displayName(active?.household_id ?? state.selected_household);
  const liveCaps = state.capabilities.filter(
    (cap) => cap.status === "verified",
  ).length;
  const allEnded =
    active?.inbound_ended &&
    active.legs.every((leg) => terminal.includes(leg.status));
  const disabled = !!busy || stale;
  const send = (path: string, body: Record<string, unknown>) => {
    void onAction(path, body);
  };
  const lastReceipt = sessionEvents.at(-1);
  const applied =
    state.community.find((h) => h.household_id === active?.household_id)
      ?.last_applied_version ?? 0;
  const matches =
    active && active.execution_mode === state.execution_mode
      ? confirmedMatches(
          events,
          active.household_id,
          active.id,
          active.execution_mode,
          applied,
          now,
        )
      : [];
  const partial = sessionEvents.findLast(
    (event) => event.type === "transcript.partial",
  );
  const partialText =
    typeof partial?.payload.text === "string"
      ? partial.payload.text
      : typeof partial?.payload.voice_prompt === "string"
        ? partial.payload.voice_prompt
        : null;

  return (
    <>
      <section className="call-banner" aria-label="Call service">
        <div className="number-block">
          <span className="number-icon">
            <PhoneCall size={21} />
          </span>
          <div>
            <span className="small-label">FirstRing phone line</span>
            <p className="phone-number">{state.phone_display}</p>
          </div>
        </div>
        <div className="line-state">
          <span className={`mode-pill ${replay ? "replay" : "live"}`}>
            {replay ? <Play size={13} /> : <Radio size={13} />}
            {replay ? "Replay" : "Live execution"}
          </span>
          <span>
            {replay
              ? "Authored scenarios · no phone calls"
              : liveCaps
                ? "Provider receipts available"
                : "Provider verification pending"}
          </span>
        </div>
        <div className="household-select">
          <label htmlFor="household">Next call for</label>
          <select
            id="household"
            value={state.selected_household}
            disabled={disabled}
            onChange={(event) =>
              send("/api/demo/household", { household_id: event.target.value })
            }
          >
            <option value="sarah">Sarah</option>
            <option value="maya">Maya</option>
          </select>
        </div>
      </section>
      {stale && (
        <div className="notice warning" role="status">
          <Clock3 size={17} /> Updates paused. Showing the last received state;
          reconnecting.
        </div>
      )}
      <div className="stage-heading">
        <div>
          <h1>{name}’s call space</h1>
          <p>A calm first conversation. A private choice.</p>
        </div>
        {state.sessions.length > 1 && (
          <div className="history-select">
            <label className="visually-hidden" htmlFor="session-history">
              Session history
            </label>
            <select
              id="session-history"
              value={active?.id ?? ""}
              onChange={(event) => setSelectedId(event.target.value)}
            >
              {state.sessions.map((session, i) => (
                <option key={session.id} value={session.id}>
                  {i === 0 ? "Most recent" : "Previous"} ·{" "}
                  {displayName(session.household_id)} · {shortId(session.id)}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>
      <main
        id="main"
        className={`call-stage ${withdrawn ? "has-intervention" : ""}`}
      >
        <section
          className="conversation-panel"
          aria-labelledby="conversation-heading"
        >
          <div className="panel-heading">
            <h2 id="conversation-heading">
              <MessageCircle size={18} /> The conversation
            </h2>
            <span className="panel-meta">
              {active?.inbound_ended
                ? "Caller ended"
                : active
                  ? "Caller line"
                  : "No active call"}
            </span>
          </div>
          <div
            className="transcript"
            aria-live="polite"
            aria-relevant="additions text"
            role="log"
            aria-label="Call transcript"
          >
            {active?.turns.length ? (
              active.turns.map((turn) => (
                <article
                  key={turn.id}
                  className={`turn ${turn.speaker === "caller" ? "caller" : "assistant"}`}
                >
                  <p className="speaker">
                    {displayName(turn.speaker)}
                    {turn.speaker === "caller" && (
                      <span>Identity unverified</span>
                    )}
                  </p>
                  <p className="turn-text">{turn.text}</p>
                </article>
              ))
            ) : (
              <div className="empty-transcript">
                <span className="empty-line-icon">
                  <Headphones size={27} strokeWidth={1.4} />
                </span>
                <h3>
                  Every call starts
                  <br />
                  with listening.
                </h3>
                <p>
                  {replay
                    ? "Choose a replay below to explore an authored conversation."
                    : "Call the FirstRing number. The caller’s words will appear here."}
                </p>
              </div>
            )}
            {active?.speech_pending && partialText && (
              <article className="turn partial">
                <p className="speaker">
                  Caller <span>Still speaking</span>
                </p>
                <p className="turn-text">{partialText}</p>
              </article>
            )}
          </div>
          <div className="panel-foot">
            <span className="tiny-dot" />
            {active?.speech_pending
              ? "New speech · choice on hold"
              : active
                ? `${active.turns.length} committed ${active.turns.length === 1 ? "turn" : "turns"}`
                : "Transcript appears from received events"}
          </div>
        </section>
        <section
          className={`decision-panel ${withdrawn ? "intervention" : ""}`}
          aria-labelledby="decision-heading"
        >
          <div className="decision-top">
            <span className="decision-mark">
              {withdrawn ? (
                <ShieldCheck size={24} strokeWidth={1.6} />
              ) : (
                <DoorOpen size={24} strokeWidth={1.6} />
              )}
            </span>
            <span>
              {active ? "FirstRing’s next step" : "Ready when you are"}
            </span>
            {sessionReplay && <span className="mini-replay">Replay</span>}
          </div>
          <div className="decision-body">
            <h2 id="decision-heading">
              {active?.current_action ??
                (replay
                  ? "See how a choice can change."
                  : "Waiting for the first ring.")}
            </h2>
            <p className="decision-context">
              {active
                ? active.speech_pending
                  ? "New caller speech is being assessed. A pending choice cannot connect yet."
                  : withdrawn
                    ? "The request changed while the choice was pending."
                    : active.message
                      ? "A minimized message is saved for this household’s review."
                      : active.state === "bridged" ||
                          active.state === "connected"
                        ? "The approved human conversation has begun. Screening has ended."
                        : "A private handoff requires a current choice from the intended recipient."
                : replay
                  ? "Explore the private handoff, then the moment a new request withdraws it."
                  : `FirstRing will ask what the caller needs before offering ${name} a private choice.`}
            </p>
            {active?.evidence_quote && (
              <div className="evidence-quote">
                <span>What changed the action</span>
                <blockquote>“{active.evidence_quote}”</blockquote>
                <p>
                  <ArrowDownLeft size={13} /> From the caller’s committed words
                </p>
              </div>
            )}
          </div>
          <div className="decision-foot">
            <LockKeyhole size={16} />
            <p>
              {withdrawn
                ? "The withdrawn offer cannot be accepted."
                : "The person receiving the call keeps the choice."}
            </p>
          </div>
        </section>
        <section className="private-panel" aria-labelledby="private-heading">
          <div className="panel-heading">
            <h2 id="private-heading">
              <LockKeyhole size={17} /> The private side
            </h2>
          </div>
          {active?.message && (
            <article className="offer" aria-label="Private saved message">
              <h3>Saved message for {name}</h3>
              <blockquote>{active.message.summary}</blockquote>
              <p className="offer-note">
                Caller identity unverified. Saved for review; not a delivery
                receipt.
              </p>
            </article>
          )}
          {active?.offer ? (
            <article className={`offer ${withdrawn ? "withdrawn" : ""}`}>
              <div className="offer-label">
                <span>
                  {withdrawn ? <X size={15} /> : <LockKeyhole size={14} />}
                  {withdrawn
                    ? "Offer withdrawn"
                    : active.offer.status === "pending"
                      ? `Only for ${name}`
                      : `Choice ${readable(active.offer.status)}`}
                </span>
              </div>
              <blockquote>{active.offer.summary}</blockquote>
              <div className="keypad-guide">
                <span>
                  <kbd>1</kbd> Connect
                </span>
                <span>
                  <kbd>2</kbd> Decline
                </span>
              </div>
              <p className="offer-note">
                {withdrawn
                  ? "An old press of 1 is rejected."
                  : active.offer.status === "pending"
                    ? "The summary is spoken only on the private line."
                    : "This offer is no longer awaiting a choice."}
              </p>
            </article>
          ) : !active?.message ? (
            <div className="empty-private">
              <span className="private-illustration">
                <Phone size={27} strokeWidth={1.3} />
                <span>
                  <LockKeyhole size={12} />
                </span>
              </span>
              <h3>A moment just for {name}.</h3>
              <p>
                A separate phone hears the summary and a keypad choice. The
                caller stays on their own line.
              </p>
            </div>
          ) : null}
          <div className="phone-legs">
            {active?.legs.map((leg) => (
              <PhoneLeg
                key={leg.action_id}
                leg={leg}
                events={sessionEvents}
                household={active.household_id}
              />
            ))}
          </div>
          {!active?.legs.length && (
            <div className="no-receipts">
              <Circle size={12} /> No outbound phone receipts yet
            </div>
          )}
        </section>
      </main>
      <section className="community-strip" aria-labelledby="community-heading">
        <div className="community-intro">
          <span className="community-icon">
            <House size={22} strokeWidth={1.6} />
          </span>
          <div>
            <h2 id="community-heading">Neighborhood Firebreak</h2>
            <p>A tactic can travel. A conversation stays private.</p>
          </div>
        </div>
        <div className="households">
          {state.community.map((household) => (
            <div className="household" key={household.household_id}>
              <span
                className={`house-avatar ${household.last_applied_version ? "applied" : ""}`}
              >
                {household.display_name.slice(0, 1)}
              </span>
              <div>
                <h3>{household.display_name}</h3>
                <p>
                  {household.last_applied_version > 0 ? (
                    <>
                      <Check size={12} /> Applied version{" "}
                      {household.last_applied_version}
                    </>
                  ) : (
                    "No applied update"
                  )}
                </p>
              </div>
            </div>
          ))}
        </div>
        <span className="fictional-note">Two fictional households</span>
      </section>
      {matches.length > 0 && (
        <section
          className="match-receipts"
          aria-label="Similar tactic evidence"
        >
          {matches.map((match) => (
            <article key={match.id}>
              <ShieldCheck size={20} />
              <div>
                <h3>Similar tactic retrieved on this call</h3>
                <p>
                  {readable(match.requested_action)} ·{" "}
                  {readable(match.pretext_category)}
                </p>
                <span>
                  {name} applied version {match.household_applied_version};
                  latest matched capsule version {match.version}. Caller
                  identity remains unverified.
                </span>
              </div>
              <div className="match-summary">
                <span className="match-fields">Requested action + context</span>
                <span className="match-count">
                  {match.supporting_count} matching{" "}
                  {match.supporting_count === 1 ? "capsule" : "capsules"}
                </span>
              </div>
            </article>
          ))}
        </section>
      )}
      {state.tactics.length > 0 && (
        <details className="tactic-details">
          <summary>
            <ShieldCheck size={16} />
            {state.tactics.length} minimized{" "}
            {state.tactics.length === 1 ? "tactic capsule" : "tactic capsules"}
            <ChevronDown size={15} />
          </summary>
          <div className="tactic-grid">
            {state.tactics.map((tactic) => (
              <article key={tactic.id}>
                <span className="tactic-version">Version {tactic.version}</span>
                <h3>{readable(tactic.tactic)}</h3>
                <dl>
                  <div>
                    <dt>Requested action</dt>
                    <dd>{readable(tactic.requested_action)}</dd>
                  </div>
                  <div>
                    <dt>Context</dt>
                    <dd>{readable(tactic.pretext_category)}</dd>
                  </div>
                  <div>
                    <dt>Pressure</dt>
                    <dd>{readable(tactic.pressure_category)}</dd>
                  </div>
                  <div>
                    <dt>Expires</dt>
                    <dd>{new Date(tactic.expires_at).toLocaleString()}</dd>
                  </div>
                </dl>
                <p>Similar tactic, not an identified person.</p>
              </article>
            ))}
          </div>
        </details>
      )}
      <section className="operator-bar" aria-label="Operator controls">
        <div className="operator-label">
          {replay ? (
            <>
              <Play size={16} />
              <div>
                <strong>Explore a replay</strong>
                <span>Authored fixtures. No calls or model requests.</span>
              </div>
            </>
          ) : (
            <>
              <PhoneCall size={16} />
              <div>
                <strong>Designated test phones</strong>
                <span>Actions use the server’s enrolled contacts.</span>
              </div>
            </>
          )}
        </div>
        <div className="operator-actions">
          {replay ? (
            <>
              <button
                disabled={disabled}
                onClick={() => send("/api/demo/replay", { scenario: "benign" })}
              >
                Replay a benign call
              </button>
              <button
                className="primary"
                disabled={disabled}
                onClick={() =>
                  send("/api/demo/replay", { scenario: "revocation" })
                }
              >
                Replay revocation
              </button>
              <button
                disabled={disabled}
                onClick={() =>
                  send("/api/demo/replay", { scenario: "benign_new_number" })
                }
              >
                New-number control
              </button>
            </>
          ) : (
            <button
              disabled={disabled || !active || sessionReplay}
              onClick={() =>
                active &&
                send("/api/demo/probe", {
                  session_id: active.id,
                  contact_id: "sarah",
                })
              }
            >
              <Phone size={14} />
              Call Phone B
            </button>
          )}
          {active && !allEnded && (
            <button
              className="quiet-button danger-text"
              disabled={disabled}
              onClick={() =>
                send("/api/demo/end-call", { session_id: active.id })
              }
            >
              <PhoneOff size={14} />
              End session
            </button>
          )}
        </div>
      </section>
      <details className="diagnostics">
        <summary>
          <span>Evidence &amp; setup</span>
          <span>
            {lastReceipt
              ? `Last receipt ${timeLabel(lastReceipt.received_at)}`
              : "No session receipts"}
            <ChevronDown size={15} />
          </span>
        </summary>
        <div className="diagnostic-body">
          <section>
            <h3>Provider readiness</h3>
            <ul className="capabilities">
              {state.capabilities.map((capability) => (
                <li key={capability.id}>
                  <span className={`cap-dot ${capability.status}`} />
                  <div>
                    <strong>{readable(capability.id)}</strong>
                    <p>{capability.detail}</p>
                  </div>
                  <span className="cap-status">{capability.status}</span>
                </li>
              ))}
            </ul>
            <p className="diagnostic-note">
              Configured is not phone-tested. Transport: {state.transport}.
              Data: {state.data_origin}.
            </p>
            <button
              className="quiet-button"
              disabled={disabled}
              onClick={() => send("/api/demo/reset", {})}
            >
              <Square size={13} />
              End test sessions &amp; reset
            </button>
          </section>
          <section>
            <h3>
              Received events{active && <span> · {shortId(active.id)}</span>}
            </h3>
            <ol className="receipt-list">
              {sessionEvents
                .filter(
                  (event) =>
                    !["session.updated", "transcript.partial"].includes(
                      event.type,
                    ),
                )
                .slice(-20)
                .reverse()
                .map((event) => (
                  <li key={event.id}>
                    <time>{timeLabel(event.received_at)}</time>
                    <div>
                      <strong>
                        {eventNames[event.type] ??
                          readable(event.type.replaceAll(".", " "))}
                      </strong>
                      <span>
                        {displayName(event.leg_role)} · {event.execution_mode} ·
                        #{event.sequence}
                      </span>
                    </div>
                  </li>
                ))}
            </ol>
            {!sessionEvents.length && (
              <p className="diagnostic-note">
                Received events will appear here. No success is inferred from an
                action request.
              </p>
            )}
          </section>
        </div>
      </details>
    </>
  );
}
