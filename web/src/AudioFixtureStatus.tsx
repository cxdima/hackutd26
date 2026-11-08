import { useState } from "react";
import { z } from "zod";

const fixtureStatus = z.object({
  scenario_id: z.enum([
    "clinic_to_login_george",
    "routine_message_george",
    "ambiguous_to_login_george",
    "ambiguous_to_reference_george",
    "explicit_login_george",
    "negated_warning_george",
    "supplied_reference_george",
    "quoted_report_george",
    "handoff_changed_george",
    "family_pickup_benign_george",
    "family_pickup_risky_george",
    "exact_appointment_george",
    "maya_appointment_george",
    "appointment_correction_george",
    "model_failure_recovery_george",
    "identity_human_decline_george",
  ]),
  destination: z.literal("owned_firstring_number_only"),
  test_kind: z.literal("automated_audio_caller"),
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
  verdict: z.literal("ungraded").optional(),
  observations: z
    .array(
      z.object({
        step: z.number().int().nonnegative(),
        speech_result: z.string(),
        origin: z.literal("twilio_gather_audio_asr"),
      }),
    )
    .optional(),
});
type FixtureStatus = z.infer<typeof fixtureStatus>;
const scenarioNames: Record<FixtureStatus["scenario_id"], string> = {
  clinic_to_login_george: "Clinic → sign-in request",
  routine_message_george: "Routine message",
  ambiguous_to_login_george: "Ambiguous request → sign-in request",
  ambiguous_to_reference_george: "Ambiguous request → supplied reference",
  explicit_login_george: "Explicit sign-in request",
  negated_warning_george: "Negated warning",
  supplied_reference_george: "Caller-supplied reference",
  quoted_report_george: "Quoted report",
  handoff_changed_george: "Changed handoff request",
  family_pickup_benign_george: "Claimed family pickup → routine details",
  family_pickup_risky_george: "Claimed family pickup → sign-in request",
  exact_appointment_george: "Appointment with exact time",
  maya_appointment_george: "Maya appointment",
  appointment_correction_george: "Appointment time correction",
  model_failure_recovery_george:
    "Deliberately injected model failure → recovery",
  identity_human_decline_george:
    "Assistant identity → human request → message declined",
};

/** Read-only: refreshing this panel cannot start or retry a caller leg. */
export function AudioFixtureStatus({ token }: { token: string }) {
  const [status, setStatus] = useState<FixtureStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  async function refresh() {
    if (loading) return;
    setLoading(true);
    setError("");
    try {
      const response = await fetch("/api/demo/audio-fixture", {
        method: "GET",
        headers: { Authorization: `Bearer ${token}` },
        cache: "no-store",
        signal: AbortSignal.timeout(10000),
      });
      if (!response.ok)
        throw new Error(
          "Status unavailable; check the server and operator token.",
        );
      const parsed = fixtureStatus.safeParse(await response.json());
      if (!parsed.success)
        throw new Error("Runner status format changed; no result inferred.");
      setStatus(parsed.data);
    } catch (failure) {
      setError(
        failure instanceof Error
          ? failure.message
          : "Status could not be refreshed.",
      );
    } finally {
      setLoading(false);
    }
  }
  const count = status?.observations?.length ?? 0;
  return (
    <section
      className="audio-fixture-status"
      aria-label="Automated caller status"
    >
      <div className="operator-bar">
        <div className="operator-label">
          <div>
            <strong>Automated caller · read-only status</strong>
            <span>Owned FirstRing number only. Refreshing places no call.</span>
          </div>
        </div>
        <button
          disabled={loading}
          onClick={() => {
            void refresh();
          }}
        >
          {loading ? "Refreshing…" : "Refresh caller test status"}
        </button>
      </div>
      {error && (
        <p role="alert">
          {error}
          {status && " Showing the last received status."}
        </p>
      )}
      {status ? (
        <div className="fixture-status-detail">
          <strong>
            {scenarioNames[status.scenario_id]} · {status.voice} ·{" "}
            {status.clip_count} audio clips
          </strong>
          <span>
            Provider: {status.provider_status ?? "No call status received"} ·{" "}
            {String(
              status.outcome ?? status.state ?? "Status not reported",
            ).replaceAll("_", " ")}
          </span>
          <span>
            Attempts remaining: {status.remaining_attempts} ·{" "}
            {status.attempt_count !== undefined &&
              status.max_attempts !== undefined &&
              `${status.attempt_count} of ${status.max_attempts} reserved · `}
            {status.time_limit_seconds}s call limit · {count} final ASR{" "}
            {count === 1 ? "observation" : "observations"}
          </span>
          {status.clips_cached === false && (
            <span>Cached audio is unavailable.</span>
          )}
          <span>Ungraded. Human listening and noise testing unverified.</span>
        </div>
      ) : (
        <p className="diagnostic-note">
          No runner status loaded. Start and retry actions are not exposed here.
        </p>
      )}
    </section>
  );
}
