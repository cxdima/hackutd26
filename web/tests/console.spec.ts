import { expect, test } from "@playwright/test";
import type { ConsoleState } from "../src/types";
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
