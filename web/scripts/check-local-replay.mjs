// Real local Rust API + browser check. Never sends a live call or model request.
import assert from "node:assert/strict";
import { readFile, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
process.env.PLAYWRIGHT_BROWSERS_PATH = "0";
const { chromium } = await import("playwright");
const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);
const base = new URL(process.env.FIRSTRING_TEST_URL ?? "http://127.0.0.1:8080");
assert(
  ["127.0.0.1", "localhost"].includes(base.hostname),
  "This check is for a local server only",
);
const token = (
  await readFile(path.join(root, ".tools/replay-admin-token"), "utf8")
).trim();
const get = async () => {
  const response = await fetch(new URL("/api/state", base), {
    headers: { Authorization: `Bearer ${token}` },
  });
  assert.equal(response.status, 200, "Local state must authenticate");
  return response.json();
};
const before = await get();
assert.equal(
  before.execution_mode,
  "replay",
  "Refusing to run this check in live execution mode",
);
const out = path.join(root, "web/test-results/actual-local-replay");
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
const browserErrors = [];
page.on("pageerror", (error) => browserErrors.push(error.message));
let streamAuthenticated = false;
page.on("request", (request) => {
  if (new URL(request.url()).pathname === "/api/events") {
    assert.equal(request.headers().authorization, `Bearer ${token}`);
    assert(!request.url().includes(token), "Credentials must not enter a URL");
    streamAuthenticated = true;
  }
});
async function waitForState(predicate, label) {
  const until = Date.now() + 30000;
  while (Date.now() < until) {
    const current = await get();
    if (predicate(current)) return current;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Timed out waiting for ${label}`);
}
async function showReplay(buttonName, expected) {
  const count = (await get()).sessions.length;
  const settled = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === "/api/demo/replay" &&
      response.request().method() === "POST",
  );
  await page.getByRole("button", { name: buttonName, exact: true }).click();
  assert.equal(
    (await settled).status(),
    200,
    "Replay action must complete locally",
  );
  const current = await waitForState(
    (state) => state.sessions.length > count && expected(state.sessions[0]),
    buttonName,
  );
  assert.equal(current.sessions[0].execution_mode, "replay");
  await page
    .getByRole("heading", {
      name: current.sessions[0].current_action,
      exact: true,
    })
    .waitFor();
  return current;
}
try {
  await page.goto(base.href);
  await page.getByLabel("Operator token").fill(token);
  await page
    .getByRole("button", { name: "Connect console", exact: true })
    .click();
  await page.getByRole("heading", { name: /call space/ }).waitFor();
  await page.locator(".mode-pill.replay").waitFor();
  assert.equal(
    await page.evaluate(() => localStorage.length + sessionStorage.length),
    0,
  );
  await page.screenshot({
    path: path.join(out, "connected.png"),
    fullPage: true,
  });
  const benign = await showReplay(
    "Replay a benign call",
    (session) => session.state === "connected",
  );
  assert.equal(benign.sessions[0].offer.status, "accepted");
  await page.screenshot({ path: path.join(out, "benign.png"), fullPage: true });
  const risky = await showReplay(
    "Replay revocation",
    (session) =>
      session.offer?.status === "revoked" &&
      session.legs.some((leg) => leg.role === "family" && leg.acknowledged),
  );
  assert.equal(risky.sessions[0].offer.status, "revoked");
  await page.getByText("Acknowledged by keypad", { exact: true }).waitFor();
  await page
    .locator(".receipt-list")
    .getByText("Old acceptance rejected", { exact: true })
    .waitFor({ state: "attached" });
  await page.screenshot({
    path: path.join(out, "revocation.png"),
    fullPage: true,
  });
  await waitForState(
    (state) =>
      state.community.find((h) => h.household_id === "maya")
        ?.last_applied_version > 0,
    "Maya subscriber receipt",
  );
  await page.getByLabel("Next call for").selectOption("maya");
  await waitForState(
    (state) => state.selected_household === "maya",
    "next household selector",
  );
  await showReplay(
    "Replay revocation",
    (session) =>
      session.household_id === "maya" &&
      session.legs.some((leg) => leg.role === "family" && leg.acknowledged),
  );
  await page
    .getByRole("heading", {
      name: "Similar tactic retrieved on this call",
      exact: true,
    })
    .first()
    .waitFor();
  assert.equal(
    await page.locator(".match-receipts article").count(),
    1,
    "Repeated matching capsules must share a compact action/context card",
  );
  assert.match(
    await page.locator(".match-count").innerText(),
    /^\d+ matching capsules?$/,
    "The group must expose its actual supporting capsule count",
  );
  await page.screenshot({
    path: path.join(out, "maya-retrieval.png"),
    fullPage: true,
  });
  const control = await showReplay(
    "New-number control",
    (session) => session.household_id === "maya" && session.turns.length > 0,
  );
  assert.equal(
    control.sessions[0].legs.length,
    0,
    "Benign new-number control must not cause outbound calls",
  );
  assert.equal(
    control.sessions[0].offer,
    null,
    "Control is a clarification, not a private offer",
  );
  assert.equal(
    control.sessions[0].blocked,
    false,
    "Negation/reference control must remain benign",
  );
  assert.equal(
    await page
      .getByRole("heading", {
        name: "Similar tactic retrieved on this call",
        exact: true,
      })
      .count(),
    0,
    "An unrelated match must not appear on this control",
  );
  await page.setViewportSize({ width: 390, height: 844 });
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
    "Mobile layout must not overflow",
  );
  await page.screenshot({
    path: path.join(out, "maya-control-mobile.png"),
    fullPage: true,
  });
  assert(streamAuthenticated, "Authenticated SSE request was not observed");
  assert.equal(browserErrors.length, 0, "Browser runtime errors occurred");
  await page.getByRole("button", { name: "Lock console", exact: true }).click();
  await page.getByLabel("Operator token").waitFor();
  assert.equal(await page.getByLabel("Operator token").inputValue(), "");
  console.log(
    "PASS: actual local Rust replay API, authenticated SSE, benign bridge, revocation/family receipt, subscriber receipt, household selector, mobile layout, and token clearing. No live phone gate tested.",
  );
} finally {
  await browser.close();
}
