import { useEffect, useState } from "react";
import type { ReactNode } from "react";
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
  decisionEvidence,
  displayName,
  legEvidence,
  legStatus,
  modelHttpEvidence,
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
  profileEditor?: ReactNode;
}

const readable = (value: string) => value.replaceAll("_", " ");
const terminal = ["completed", "busy", "failed", "no-answer", "canceled"];
const eventNames: Record<string, string> = {
  "agent.tool.proposed": "Model proposed a tool",
  "agent.tool.result": "Rust execution result",
  "model.request.completed": "Model HTTP request completed",
  "model.cancelled": "Model job cancelled",
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
  profileEditor,
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
  const name =
    active?.profile?.display_name ??
    displayName(active?.household_id ?? state.selected_household);
  const configured = state.capabilities.filter(
    (cap) =>
      ["inbound", "outbound", "nemotron", "relay"].includes(cap.id) &&
      cap.status === "configured",
  ).length;
  const phoneBReady =
    !!state.phone_b &&
    state.phone_b.enrolled &&
    state.phone_b.enabled &&
    state.phone_b.authorized &&
    state.phone_b.remaining_attempts > 0 &&
    state.phone_b.can_request &&
    !!active &&
    active.execution_mode === "live" &&
    active.household_id === "sarah" &&
    !active.inbound_ended &&
    !active.legs.some((leg) => leg.kind === "probe");
  const phoneBReason = !state.phone_b
    ? "Enrollment and attempt budget have not been received."
    : !state.phone_b.can_request
      ? state.phone_b.reason
      : !active ||
          sessionReplay ||
          active.household_id !== "sarah" ||
          active.inbound_ended
        ? "An active live Sarah session is required."
        : active.legs.some((leg) => leg.kind === "probe")
          ? "This session already has a reserved probe."
          : state.phone_b.reason;
  const householdConnectionUnavailable =
    !sessionReplay &&
    state.phone_b !== undefined &&
    (!state.phone_b.enrolled || !state.phone_b.enabled);
  const allEnded =
    active?.inbound_ended &&
    active.legs.every((leg) => terminal.includes(leg.status));
  const disabled = !!busy || stale;
  const send = (path: string, body: Record<string, unknown>) => {
    void onAction(path, body);
  };
  const lastReceipt = sessionEvents.at(-1);
  const decision = active ? decisionEvidence(active, sessionEvents) : undefined;
  const modelHttp = active
    ? modelHttpEvidence(active, sessionEvents)
    : undefined;
  const acceptedUnderstanding = active?.understanding;
  const understanding = acceptedUnderstanding?.understanding;
  const understandingCurrent =
    !!active &&
    !!acceptedUnderstanding &&
    acceptedUnderstanding.generation_id === active.generation_id &&
    acceptedUnderstanding.turn_version === active.turn_version &&
    acceptedUnderstanding.policy_epoch === active.policy_epoch &&
    active.assessed_version === active.turn_version &&
    !active.speech_pending;
  const toolNames: Record<string, string> = {
    record_assessment: "Assess the caller’s request",
    ask_caller: "Ask a follow-up",
    offer_handoff: "Offer a private handoff",
    take_message: "Save a private message",
    lookup_tactics: "Look up similar tactics",
    lookup_trusted_contact: "Look up an enrolled contact",
    start_trusted_callback: "Request an independent contact",
  };
  const currentCallerTurn = active?.turns.findLast(
    (turn) => turn.speaker === "caller",
  );
  const supportingQuote = decision?.assessment
    ? decision.assessment.evidence_refs.find(
        (reference) =>
          reference.turn_id === currentCallerTurn?.id &&
          reference.quote.length > 0 &&
          currentCallerTurn.text.includes(reference.quote),
      )?.quote
    : active?.evidence_quote;
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
            {replay ? "Replay" : "Execution mode: live"}
          </span>
          <span>
            {replay
              ? "Authored scenarios · no phone calls"
              : "Verification is shown separately below"}
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
          {profileEditor}
        </div>
      </section>
      <section
        className="verification-strip"
        aria-label="Configuration and verification"
      >
        <div role="group" aria-label="Local configuration">
          <strong>Local configuration</strong>
          <span className="proof-status">{configured}/4 inputs configured</span>
          <p>Credentials and routes only. See Evidence &amp; setup.</p>
        </div>
        {(
          [
            ["model_protocol", "Model protocol"],
            ["inbound_phone", "Real inbound phone"],
            ["relay_phone", "Full Relay rehearsal"],
          ] as const
        ).map(([key, label]) => {
          const proof = state.evidence?.[key];
          return (
            <div role="group" aria-label={label} key={key}>
              <strong>{label}</strong>
              <span className={`proof-status ${proof?.status ?? "pending"}`}>
                {proof?.status === "passed"
                  ? "Recorded pass"
                  : proof?.status === "failed"
                    ? "Recorded failure"
                    : "Pending"}
              </span>
              <p>{proof?.detail ?? "No verified receipt received."}</p>
              {key === "model_protocol" &&
                state.model_backend?.backend === "brev_hosted" && (
                  <small>
                    NVIDIA-hosted history; selected Brev verification is
                    separate.
                  </small>
                )}
              {proof?.recorded_at && (
                <time dateTime={proof.recorded_at}>
                  Recorded {new Date(proof.recorded_at).toLocaleString()}
                </time>
              )}
            </div>
          );
        })}
      </section>
      {state.model_backend && (
        <div
          className="speech-configuration"
          role="group"
          aria-label="Current model backend"
        >
          <strong>Selected model backend · {state.model_backend.label}</strong>
          <span style={{ overflowWrap: "anywhere", minWidth: 0 }}>
            Requested model: {state.model_backend.requested_model}
          </span>
          <span>
            Configuration:{" "}
            {state.model_backend.configured ? "Configured" : "Incomplete"}
          </span>
          <span>
            Native protocol:{" "}
            {state.model_backend.protocol_verified
              ? "Verified for this backend"
              : "Pending for this backend"}
          </span>
          <small>{state.model_backend.detail}</small>
        </div>
      )}
      {state.speech && (
        <div
          className="speech-configuration"
          role="group"
          aria-label="Current speech configuration"
        >
          <strong>Current speech setup · {state.speech.transport}</strong>
          <span>
            Voice: {state.speech.provider} · {state.speech.voice_label}
          </span>
          {state.speech.transcription_provider && (
            <span>
              Transcription: {state.speech.transcription_provider} ·{" "}
              {state.speech.transcription_model ?? "Model not reported"}
            </span>
          )}
          <span>
            Goodbye: {state.speech.closing_provider} ·{" "}
            {state.speech.closing_voice}
          </span>
          <small>
            Configuration only; measured model and phone results are separate.
          </small>
        </div>
      )}
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
          {active?.profile && (
            <p className="profile-snapshot">
              Captured profile v{active.profile.version} ·{" "}
              {active.profile.tts_provider} · {active.profile.voice_id} ·{" "}
              {active.profile.locale}. Settings for this call; listening proof
              is separate.
            </p>
          )}
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
            {active?.speech_pending &&
              !active.inbound_ended &&
              !active.closing_reason &&
              partialText && (
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
            {active?.inbound_ended
              ? "Caller ended"
              : active?.closing_reason === "message_saved"
                ? "Message saved; goodbye pending"
                : active?.speech_pending
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
              {allEnded
                ? "Call ended"
                : (active?.current_action ??
                  (replay
                    ? "See how a choice can change."
                    : "Waiting for the first ring."))}
            </h2>
            <p className="decision-context">
              {active
                ? active.closing_reason === "message_saved"
                  ? active.inbound_ended
                    ? "A private message is saved and the call has ended. Goodbye playback remains unverified."
                    : "A private message is saved. Goodbye delivery and call completion still need their own receipts."
                  : active.inbound_ended
                    ? "The caller has ended. Any independent contact outcome is shown separately."
                    : active.speech_pending
                      ? "Waiting for the caller to finish. A pending choice cannot connect yet."
                      : withdrawn
                        ? "The request changed while the choice was pending."
                        : active.message
                          ? "A minimized message is saved for this household’s review."
                          : active.state === "bridged" ||
                              active.state === "connected"
                            ? "The approved human conversation has begun. Screening has ended."
                            : householdConnectionUnavailable
                              ? "Screening and private message-taking; household connection unavailable."
                              : "A private handoff requires a current choice from the intended recipient."
                : replay
                  ? "Explore the private handoff, then the moment a new request withdraws it."
                  : householdConnectionUnavailable
                    ? "FirstRing will ask what the caller needs and can take a private message."
                    : `FirstRing will ask what the caller needs before offering ${name} a private choice.`}
            </p>
            {understanding && acceptedUnderstanding && (
              <div
                className="understanding-summary"
                role="group"
                aria-label="Accepted call understanding"
              >
                <strong>
                  {understandingCurrent
                    ? "Current accepted understanding"
                    : "Earlier accepted understanding · not current"}
                </strong>
                <p>
                  <span>Purpose:</span> {understanding.purpose}
                </p>
                <p>
                  <span>Requested action:</span>{" "}
                  {understanding.requested_action}
                </p>
                <p>
                  Claimed identity (unverified):{" "}
                  {understanding.claimed_identity ?? "None stated"}
                </p>
                {understanding.supplied_facts.length > 0 && (
                  <>
                    <span>Supplied facts · caller’s words</span>
                    <ul>
                      {understanding.supplied_facts.map((fact, index) => (
                        <li key={index}>{fact.text}</li>
                      ))}
                    </ul>
                  </>
                )}
                {understanding.missing_information.length > 0 && (
                  <p>
                    <span>Still needed:</span>{" "}
                    {understanding.missing_information.join("; ")}
                  </p>
                )}
                <p>
                  <span>Readiness:</span> {readable(understanding.readiness)}
                </p>
                <small>
                  Turn {acceptedUnderstanding.turn_version} · generation{" "}
                  {acceptedUnderstanding.generation_id} · policy{" "}
                  {acceptedUnderstanding.policy_epoch}. Action permission and
                  completion are shown separately.
                </small>
              </div>
            )}
            {active && (
              <dl className="decision-trace">
                <div>
                  <dt>Accepted assessment</dt>
                  <dd>
                    {decision?.assessment
                      ? `${readable(decision.assessment.observation.speech_act)} · ${readable(decision.assessment.observation.requested_object)} · ${readable(decision.assessment.observation.direction).replace("sarah", name)}`
                      : active.turns.some((turn) => turn.speaker === "caller")
                        ? "No accepted assessment for the current turn."
                        : "No committed caller words yet."}
                  </dd>
                </div>
                <div>
                  <dt>
                    {decision?.proposal?.legacy
                      ? "Earlier proposal (turn not recorded)"
                      : "Proposed action"}
                  </dt>
                  <dd>
                    {decision?.proposal
                      ? (toolNames[decision.proposal.name] ??
                        readable(decision.proposal.name))
                      : "No model proposal received for this turn."}
                  </dd>
                </div>
                <div>
                  <dt>Action acceptance</dt>
                  <dd>
                    {decision?.acceptance === "accepted"
                      ? "Accepted by Rust; successful tool result received."
                      : decision?.acceptance === "failed"
                        ? "Rust reported a failed result; acceptance is not confirmed."
                        : "Awaiting a successful Rust result."}
                  </dd>
                </div>
                <div>
                  <dt>Actual execution outcome</dt>
                  <dd>{decision?.outcome}</dd>
                </div>
              </dl>
            )}
            {supportingQuote && (
              <div className="evidence-quote">
                <span>
                  {decision?.assessment
                    ? "Supporting caller words"
                    : "Last recorded caller evidence"}
                </span>
                <blockquote>“{supportingQuote}”</blockquote>
                <p>
                  <ArrowDownLeft size={13} /> From the caller’s committed words
                </p>
              </div>
            )}
            {modelHttp && (
              <div
                role="group"
                aria-label="Model HTTP observation"
                className="model-observation"
              >
                <dl className="decision-trace">
                  <div>
                    <dt>Model HTTP observation</dt>
                    <dd>
                      {modelHttp.completed ? (
                        <>
                          Backend:{" "}
                          {modelHttp.completed.observation.backend ===
                          "brev_hosted"
                            ? "Brev hosted"
                            : modelHttp.completed.observation.backend ===
                                "nvidia_hosted"
                              ? "NVIDIA hosted"
                              : "Not recorded"}
                          <br />
                          {modelHttp.completed.observation.backend &&
                            state.model_backend &&
                            modelHttp.completed.observation.backend !==
                              state.model_backend.backend && (
                              <>
                                Recorded backend differs from the selected
                                backend
                                <br />
                              </>
                            )}
                          <span style={{ overflowWrap: "anywhere" }}>
                            {modelHttp.completed.observation.response_model ??
                              "Response model not reported"}
                          </span>
                          <br />
                          {readable(
                            modelHttp.completed.observation.outcome,
                          )} · {modelHttp.completed.observation.elapsed_ms} ms
                          <br />
                          Turn {modelHttp.completed.turn_version} · generation{" "}
                          {modelHttp.completed.generation_id}
                          <br />
                          {modelHttp.current
                            ? "Current turn and generation"
                            : "Earlier turn or generation"}
                          {modelHttp.cancelledGeneration !== undefined && (
                            <>
                              <br />
                              Model job later cancelled
                            </>
                          )}
                        </>
                      ) : (
                        <>
                          No completed HTTP observation received.
                          {modelHttp.cancelledGeneration !== undefined && (
                            <>
                              <br />
                              Model job cancelled · generation{" "}
                              {modelHttp.cancelledGeneration}
                            </>
                          )}
                        </>
                      )}
                    </dd>
                  </div>
                </dl>
                <p className="offer-note">
                  HTTP timing; audible response unverified. This is not an
                  accepted assessment or execution receipt.
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
                <span id="phone-b-reason">
                  {phoneBReason}
                  {state.phone_b &&
                    ` Attempt budget remaining: ${state.phone_b.remaining_attempts}.`}
                </span>
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
              disabled={disabled || !phoneBReady}
              aria-describedby="phone-b-reason"
              onClick={() =>
                phoneBReady &&
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
