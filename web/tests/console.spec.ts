import { expect, test } from "@playwright/test";
import type { CallEvent, ConsoleState } from "../src/types";
const speech: NonNullable<ConsoleState["speech"]> = {
  source: "configured",
  transport: "relay",
  provider: "ElevenLabs",
  voice: "21m00Tcm4TlvDq8ikWAM-v4",
  voice_label: "Rachel v4",
  closing_provider: "Amazon Polly",
  closing_voice: "Joanna-Neural",
  transcription_provider: "Deepgram",
  transcription_model: "nova-3-general",
};
const empty: ConsoleState = {
  execution_mode: "replay",
  data_origin: "synthetic",
  transport: "probe",
  phone_display: "Not configured",
  selected_household: "sarah",
  capabilities: [
    {
      id: "telephony",
      status: "pending",
      detail: "Provider credentials and test endpoints are not configured.",
    },
    {
      id: "nemotron",
      status: "pending",
      detail: "Native tool-call probe pending.",
    },
  ],
  sessions: [],
  community: [
    { household_id: "sarah", display_name: "Sarah", last_applied_version: 0 },
    { household_id: "maya", display_name: "Maya", last_applied_version: 0 },
  ],
  tactics: [],
};
const revoked: ConsoleState = {
  ...empty,
  sessions: [
    {
      id: "fixture-session",
      household_id: "sarah",
      state: "contact_pending",
      policy_epoch: 2,
      generation_id: 3,
      speech_pending: false,
      inbound_ended: true,
      current_action: "Offer withdrawn. Calling a trusted contact.",
      evidence_quote: "Read me the six digits from the sign-in text.",
      execution_mode: "replay",
      data_origin: "synthetic",
      turns: [
        {
          id: "t1",
          speaker: "caller",
          text: "Hi, I’m calling from the clinic. Sarah’s appointment has moved to Tuesday at two.",
        },
        {
          id: "t2",
          speaker: "assistant",
          text: "Is there anything Sarah needs to prepare before the visit?",
        },
        {
          id: "t3",
          speaker: "caller",
          text: "Read me the six digits from the sign-in text.",
        },
      ],
      offer: {
        id: "fixture-offer",
        summary:
          "A caller says your clinic appointment is Tuesday at two. Identity unverified.",
        status: "revoked",
        policy_epoch: 1,
        expires_at: "2026-11-08T00:00:00Z",
      },
      legs: [
        {
          action_id: "fixture-sarah",
          role: "sarah",
          status: "completed",
          acknowledged: false,
        },
        {
          action_id: "fixture-family",
          role: "family",
          status: "ringing",
          acknowledged: false,
        },
      ],
    },
  ],
  community: [
    { household_id: "sarah", display_name: "Sarah", last_applied_version: 1 },
    { household_id: "maya", display_name: "Maya", last_applied_version: 0 },
  ],
  tactics: [],
};

test("model HTTP receipt remains distinct from a current assessment and audible result", async ({
  page,
}, info) => {
  const snapshot: ConsoleState = {
    ...empty,
    transport: "relay",
    model_backend: {
      backend: "brev_hosted",
      label: "Brev-hosted Nemotron",
      requested_model: "nvidia/Nemotron-3-Nano-30B-A3B-FP8",
      configured: true,
      protocol_verified: false,
      detail: "This backend's native tool cycle has not been verified.",
    },
    sessions: [
      {
        ...revoked.sessions[0],
        state: "screening",
        current_action: "Waiting for a complete caller turn",
        generation_id: 4,
        turn_version: 2,
        speech_pending: true,
        inbound_ended: false,
        offer: null,
        legs: [],
      },
    ],
  };
  const completed: CallEvent = {
    id: "fixture-model",
    sequence: 1,
    session_id: "fixture-session",
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
        backend: "nvidia_hosted",
        requested_model: "nvidia/nemotron-3.5-lightning-30b-a3b",
        response_model: "nvidia/nemotron-3.5-lightning-30b-a3b",
        elapsed_ms: 998,
        outcome: "success",
        error: null,
      },
    },
  };
  const cancelled: CallEvent = {
    ...completed,
    id: "fixture-cancel",
    sequence: 2,
    type: "model.cancelled",
    payload: { generation_id: 3, reason: "caller_speech" },
  };
  await page.route("**/api/state", (route) =>
    route.fulfill({ json: snapshot }),
  );
  await page.route("**/api/events", (route) =>
    route.fulfill({
      contentType: "text/event-stream",
      body: [completed, cancelled]
        .map(
          (event) =>
            `id: ${event.sequence}\ndata: ${JSON.stringify(event)}\n\n`,
        )
        .join(""),
    }),
  );
  await page.goto("/");
  await page.getByLabel("Operator token").fill("fixture-operator-token");
  await page.getByRole("button", { name: "Connect console" }).click();
  const card = page.getByRole("group", { name: "Model HTTP observation" });
  const selectedBackend = page.getByRole("group", {
    name: "Current model backend",
  });
  await expect(selectedBackend).toContainText("Brev-hosted Nemotron");
  await expect(selectedBackend).toContainText(
    "Native protocol: Pending for this backend",
  );
  await expect(card).toContainText("Backend: NVIDIA hosted");
  await expect(card).toContainText(
    "Recorded backend differs from the selected backend",
  );
  await expect(card).toContainText("998 ms");
  await expect(card).toContainText("Earlier turn or generation");
  await expect(card).toContainText("Model job later cancelled");
  await expect(
    page.getByText("No accepted assessment for the current turn.", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("group", { name: "Full Relay rehearsal" }),
  ).toContainText("Pending");
  await page.screenshot({
    path: info.outputPath("fixture-model-http-desktop.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await card.scrollIntoViewIfNeeded();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("fixture-model-http-mobile.png"),
    fullPage: true,
  });
});

test("reads automated caller receipts without initiating or retrying a call", async ({
  page,
}, info) => {
  await page.route("**/api/state", (route) =>
    route.fulfill({
      json: { ...empty, execution_mode: "live", transport: "relay", speech },
    }),
  );
  await page.route("**/api/events", () => new Promise(() => {}));
  const methods: string[] = [];
  await page.route("**/api/demo/audio-fixture", (route) => {
    expect(route.request().headers().authorization).toBe(
      "Bearer fixture-operator-token",
    );
    methods.push(route.request().method());
    return route.fulfill({
      json: {
        scenario_id: "family_pickup_risky_george",
        execution_mode: "live",
        data_origin: "synthetic",
        test_kind: "automated_audio_caller",
        destination: "owned_firstring_number_only",
        voice: "George",
        clip_count: 2,
        time_limit_seconds: 90,
        provider_status: "completed",
        outcome: "audio_exchange_captured_ungraded",
        remaining_attempts: 0,
        human_audition: false,
        noise_tested: false,
        verdict: "ungraded",
        observations: [
          {
            step: 1,
            speech_result: "I cannot share sign-in details.",
            confidence: 0.9,
            received_at: "2026-11-08T04:30:00Z",
            origin: "twilio_gather_audio_asr",
          },
        ],
        partial_observations: {},
      },
    });
  });
  await page.goto("/");
  await page.getByLabel("Operator token").fill("fixture-operator-token");
  await page.getByRole("button", { name: "Connect console" }).click();
  expect(methods).toEqual([]);
  await expect(
    page.getByRole("group", { name: "Current speech configuration" }),
  ).toContainText("ElevenLabs · Rachel v4");
  const refresh = page.getByRole("button", {
    name: "Refresh caller test status",
  });
  await expect(refresh).toBeVisible({ timeout: 1000 });
  await refresh.click();
  const status = page.getByRole("region", { name: "Automated caller status" });
  await expect(status).toContainText("completed");
  await expect(status).toContainText("Ungraded");
  await expect(status).toContainText("Claimed family pickup → sign-in request");
  await expect(status).toContainText("1 final ASR observation");
  await expect(status).toContainText("Attempts remaining: 0");
  await expect(status).toContainText(
    "Human listening and noise testing unverified",
  );
  await expect(status.getByRole("button")).toHaveCount(1);
  expect(methods).toEqual(["GET"]);
  await page.screenshot({
    path: info.outputPath("fixture-audio-status-desktop.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await status.scrollIntoViewIfNeeded();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("fixture-audio-status-mobile.png"),
    fullPage: true,
  });
});

test("recorded inbound and model proof do not enable Relay or an unenrolled Phone B", async ({
  page,
}, info) => {
  const snapshot: ConsoleState = {
    ...empty,
    execution_mode: "live",
    speech,
    transport: "probe",
    capabilities: [
      {
        id: "inbound",
        status: "configured",
        detail: "Signing inputs configured",
      },
      { id: "relay", status: "configured", detail: "Local Relay inputs only" },
    ],
    evidence: {
      model_protocol: {
        status: "passed",
        detail: "Synthetic native tool/result probe completed.",
      },
      inbound_phone: {
        status: "passed",
        detail: "Recorded signed inbound phone test.",
      },
      relay_phone: {
        status: "pending",
        detail: "Live Relay listening test still required.",
      },
    },
    phone_b: {
      enrolled: false,
      enabled: false,
      authorized: false,
      remaining_attempts: 1,
      can_request: false,
      reason: "Sarah / Phone B is not enrolled; outbound calling is disabled.",
    },
    sessions: [
      {
        ...revoked.sessions[0],
        execution_mode: "live",
        state: "screening",
        current_action: "Assessing the caller’s request",
        inbound_ended: false,
        offer: null,
        legs: [],
        turn_version: 2,
        assessed_version: 2,
        generation_id: 2,
        evidence_quote: "clinic",
        understanding: {
          generation_id: 2,
          turn_version: 2,
          policy_epoch: 2,
          understanding: {
            version: 1,
            claimed_identity: null,
            purpose: "Reschedule a clinic visit",
            requested_action: "Share the new appointment time",
            supplied_facts: [
              {
                text: "reschedule the clinic visit",
                evidence_refs: [
                  {
                    turn_id: "t1",
                    quote: "Please reschedule the clinic visit.",
                  },
                ],
              },
            ],
            missing_information: ["New appointment time"],
            readiness: "needs_information",
          },
        },
        turns: [
          {
            id: "earlier",
            speaker: "caller",
            text: "I am calling from the clinic.",
          },
          {
            id: "t1",
            speaker: "caller",
            text: "Please reschedule the clinic visit.",
          },
        ],
        assessments: [
          {
            turn_version: 2,
            generation_id: 2,
            observation: {
              speech_act: "request",
              requested_object: "routine_information",
              direction: "caller_requests_from_sarah",
            },
            evidence_refs: [
              { turn_id: "earlier", quote: "clinic" },
              { turn_id: "t1", quote: "reschedule the clinic visit" },
            ],
          },
        ],
      },
    ],
  };
  await page.route("**/api/events", () => new Promise(() => {}));
  await page.route("**/api/state", (route) =>
    route.fulfill({ json: snapshot }),
  );
  let mutations = 0;
  await page.route("**/api/demo/**", (route) => {
    mutations++;
    return route.abort();
  });
  await page.goto("/");
  await page.getByLabel("Operator token").fill("fixture-operator-token");
  await page.getByRole("button", { name: "Connect console" }).click();
  await expect(
    page.getByRole("group", { name: "Model protocol" }),
  ).toContainText("Recorded pass");
  await expect(
    page.getByRole("group", { name: "Real inbound phone" }),
  ).toContainText("Recorded pass");
  await expect(
    page.getByRole("group", { name: "Full Relay rehearsal" }),
  ).toContainText("Pending");
  await expect(
    page.getByRole("button", { name: "Call Phone B" }),
  ).toBeDisabled();
  await expect(
    page.getByText("Accepted assessment", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "request · routine information · caller requests from Sarah",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(
    page.getByText("No execution result received.", { exact: true }),
  ).toBeVisible();
  await expect(page.locator(".evidence-quote blockquote")).toHaveText(
    "“reschedule the clinic visit”",
  );
  const understanding = page.getByRole("group", {
    name: "Accepted call understanding",
  });
  await expect(understanding).toContainText("Current accepted understanding");
  await expect(understanding).toContainText("Reschedule a clinic visit");
  await expect(understanding).toContainText(
    "Still needed: New appointment time",
  );
  await page.screenshot({
    path: info.outputPath("fixture-evidence-desktop.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("fixture-evidence-mobile.png"),
    fullPage: true,
  });
  expect(mutations).toBe(0);
});

test("stored message appears privately without a delivery or community claim", async ({
  page,
}, info) => {
  const message = {
    summary: "Caller asks to reschedule the clinic visit on Tuesday.",
    recorded_at: "2026-11-08T00:00:00Z",
    turn_version: 1,
    evidence_turn_ids: ["message-turn"],
  };
  const snapshot: ConsoleState = {
    ...empty,
    sessions: [
      {
        ...revoked.sessions[0],
        state: "screening",
        policy_epoch: 0,
        current_action: "Message received; caller identity unverified",
        turns: [
          {
            id: "message-turn",
            speaker: "caller",
            text: "Please reschedule the clinic visit on Tuesday.",
          },
        ],
        evidence_quote: "Please reschedule the clinic visit on Tuesday.",
        offer: null,
        legs: [],
        message,
      },
    ],
  };
  await page.route("**/api/events", () => new Promise(() => {}));
  await page.route("**/api/state", (route) =>
    route.fulfill({ json: snapshot }),
  );
  await page.goto("/");
  await page.getByLabel("Operator token").fill("fixture-operator-token");
  await page.getByRole("button", { name: "Connect console" }).click();
  await expect(
    page.getByRole("heading", { name: "Saved message for Sarah" }),
  ).toBeVisible();
  await expect(
    page.getByRole("article", { name: "Private saved message" }),
  ).toContainText(message.summary);
  await expect(
    page.getByText(
      "Caller identity unverified. Saved for review; not a delivery receipt.",
    ),
  ).toBeVisible();
  await expect(page.locator(".community-strip")).not.toContainText(
    "reschedule",
  );
  await page.screenshot({
    path: info.outputPath("private-message-desktop.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page
    .getByRole("heading", { name: "Saved message for Sarah" })
    .scrollIntoViewIfNeeded();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("private-message-mobile.png"),
    fullPage: true,
  });
});

test("locked console keeps credentials out of storage and unlocks an honest empty replay", async ({
  page,
}, info) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.route("**/api/events", () => new Promise(() => {}));
  await page.route("**/api/state", async (route) => {
    expect(route.request().headers().authorization).toBe(
      "Bearer fixture-operator-token",
    );
    await route.fulfill({ json: empty });
  });
  await page.goto("/");
  await page.screenshot({
    path: info.outputPath("locked-desktop.png"),
    fullPage: true,
  });
  await page.getByLabel("Operator token").fill("fixture-operator-token");
  await page.getByRole("button", { name: "Connect console" }).click();
  await expect(
    page.getByRole("heading", { name: "See how a choice can change." }),
  ).toBeVisible();
  await expect(page.getByText("Not configured", { exact: true })).toBeVisible();
  expect(
    await page.evaluate(() => ({
      local: localStorage.length,
      session: sessionStorage.length,
    })),
  ).toEqual({ local: 0, session: 0 });
  await page.screenshot({
    path: info.outputPath("empty-desktop.png"),
    fullPage: true,
  });
  await page.getByRole("button", { name: "Lock console" }).click();
  await expect(page.getByLabel("Operator token")).toHaveValue("");
  expect(errors).toEqual([]);
});

test("replay action shows withdrawn consent and unacknowledged phone progression at desktop and mobile sizes", async ({
  page,
}, info) => {
  let snapshot = empty;
  await page.route("**/api/events", () => new Promise(() => {}));
  await page.route("**/api/state", (route) =>
    route.fulfill({ json: snapshot }),
  );
  await page.route("**/api/demo/replay", async (route) => {
    expect(route.request().postDataJSON()).toEqual({ scenario: "revocation" });
    snapshot = revoked;
    await route.fulfill({ json: { accepted: true } });
  });
  await page.goto("/");
  await page.getByLabel("Operator token").fill("fixture-operator-token");
  await page.getByRole("button", { name: "Connect console" }).click();
  await page.getByRole("button", { name: "Replay revocation" }).click();
  await expect(
    page.getByRole("heading", { name: revoked.sessions[0].current_action }),
  ).toBeVisible();
  await expect(
    page.getByText("Offer withdrawn", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Acknowledged by keypad", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByText("No applied update", { exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: info.outputPath("revocation-desktop.png"),
    fullPage: true,
  });
  for (const width of [390, 768]) {
    await page.setViewportSize({ width, height: 844 });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    await page.screenshot({
      path: info.outputPath(`revocation-${width}.png`),
      fullPage: true,
    });
  }
});

test("rejected token leaves the call console locked and presents a useful error", async ({
  page,
}) => {
  await page.route("**/api/state", (route) =>
    route.fulfill({ status: 401, json: { error: "unauthorized" } }),
  );
  await page.goto("/");
  await page.getByLabel("Operator token").fill("invalid-fixture-token");
  await page.getByRole("button", { name: "Connect console" }).click();
  await expect(page.getByRole("alert")).toContainText("token was not accepted");
  await expect(page.getByRole("button", { name: "Call Phone B" })).toHaveCount(
    0,
  );
});

test("marks a stalled event connection as stale while retaining the last snapshot", async ({
  page,
}) => {
  await page.route("**/api/events", () => new Promise(() => {}));
  await page.route("**/api/state", (route) => route.fulfill({ json: empty }));
  await page.goto("/");
  await page.getByLabel("Operator token").fill("fixture-operator-token");
  await page.getByRole("button", { name: "Connect console" }).click();
  await expect(
    page.getByRole("heading", { name: "See how a choice can change." }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Updates paused. Showing the last received state; reconnecting.",
    ),
  ).toBeVisible({ timeout: 15000 });
  await expect(
    page.getByRole("button", { name: "Replay revocation" }),
  ).toBeDisabled();
});
