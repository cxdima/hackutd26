// Record real local actor/SSE behavior with authored inputs. Never phone proof.
import assert from "node:assert/strict";
import { mkdir, readFile, rename } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createParser } from "eventsource-parser";

process.env.PLAYWRIGHT_BROWSERS_PATH = "0";
const { chromium } = await import("playwright");
const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);
const base = new URL(process.env.FIRSTRING_TEST_URL ?? "http://127.0.0.1:8080");
assert(
  base.protocol === "http:" &&
    ["127.0.0.1", "localhost"].includes(base.hostname) &&
    !base.username &&
    !base.password &&
    base.pathname === "/" &&
    !base.search &&
    !base.hash,
  "Recording requires a plain localhost HTTP origin",
);
const token = (
  await readFile(path.join(root, ".tools/replay-admin-token"), "utf8")
).trim();
assert(token.length > 0, "A local operator token is required");
const headers = { Authorization: `Bearer ${token}` };
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function getState() {
  const response = await fetch(new URL("/api/state", base), {
    headers,
    redirect: "error",
  });
  assert.equal(response.status, 200, "Local state must authenticate");
  const state = await response.json();
  assert.equal(state.execution_mode, "replay", "Refusing live execution");
  assert.equal(state.data_origin, "synthetic", "Authored inputs are required");
  assert(
    !/\d(?:[\s()+.-]*\d){6}/.test(state.phone_display),
    "Recording requires an unconfigured or redacted phone display",
  );
  return state;
}

// Check before launching the browser or submitting any action.
await getState();
const out = path.join(root, ".tools/artifacts");
await mkdir(out, { recursive: true });
const videoPath = path.join(out, "firstring-replay.webm");
const imagePath = path.join(out, "firstring-replay.png");
for (const file of [videoPath, imagePath]) {
  try {
    await rename(file, `${file}.${Date.now()}.previous`);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
}

const controller = new AbortController();
const receipts = [];
const stream = await fetch(new URL("/api/events", base), {
  headers,
  signal: controller.signal,
  redirect: "error",
});
assert.equal(stream.status, 200, "Actual event stream must authenticate");
const parser = createParser({
  onEvent(event) {
    const receipt = JSON.parse(event.data);
    assert.equal(receipt.execution_mode, "replay", "Live receipt rejected");
    receipts.push(receipt);
    if (receipts.length > 10000) receipts.shift();
  },
});
let streamFailure;
const streamReader = stream.body.getReader();
const streamTask = (async () => {
  try {
    const decoder = new TextDecoder();
    while (!controller.signal.aborted) {
      const { done, value } = await streamReader.read();
      if (done) break;
      parser.feed(decoder.decode(value, { stream: true }));
    }
    if (!controller.signal.aborted) throw new Error("Event stream closed");
  } catch (error) {
    if (!controller.signal.aborted) streamFailure = error;
  }
})();

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1440, height: 1040 },
  serviceWorkers: "block",
});
page.setDefaultTimeout(15000);
const violations = [];
const browserErrors = [];
let authenticatedStream = false;
let recording = false;
page.on("pageerror", () => browserErrors.push("Browser runtime error"));
// This is a network boundary, not a response fixture: local requests continue
// unchanged; everything else is blocked and makes the recording fail.
await page.route("**/*", async (route) => {
  const request = route.request();
  const url = new URL(request.url());
  if (
    url.origin !== base.origin ||
    (!["GET", "HEAD"].includes(request.method()) &&
      !["/api/demo/replay", "/api/demo/household"].includes(url.pathname))
  ) {
    violations.push("Unexpected network destination or mutation");
    await route.abort();
    return;
  }
  if (url.pathname === "/api/events") {
    assert.equal(request.headers().authorization, `Bearer ${token}`);
    authenticatedStream = true;
  }
  assert(!request.url().includes(token), "Token must not enter a URL");
  await route.continue();
});

async function waitFor(predicate, label) {
  const until = Date.now() + 15000;
  while (Date.now() < until) {
    assert(!streamFailure, "The actual event stream failed");
    const result = await predicate();
    if (result) return result;
    await sleep(150);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function household(id) {
  await getState();
  await page.getByLabel("Next call for").selectOption(id);
  await waitFor(async () => (await getState()).selected_household === id, id);
}

async function replay(name, expected) {
  await getState();
  const responsePromise = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === "/api/demo/replay" &&
      response.request().method() === "POST",
  );
  await page.getByRole("button", { name, exact: true }).click();
  const response = await responsePromise;
  assert.equal(response.status(), 200, "Replay must complete locally");
  const result = await response.json();
  assert.equal(result.execution_mode, "replay");
  const session = await waitFor(async () => {
    const current = (await getState()).sessions.find(
      (candidate) => candidate.id === result.session_id,
    );
    return current && expected(current) ? current : false;
  }, name);
  await page
    .getByRole("heading", { name: session.current_action, exact: true })
    .waitFor();
  await page.evaluate(() => scrollTo({ top: 0, behavior: "instant" }));
  return session;
}

async function segment(title, description, duration, action) {
  const started = Date.now();
  await page.screencast.showChapter(title, { description, duration: 1600 });
  await sleep(1650);
  if (action) await action();
  await sleep(Math.max(0, duration - (Date.now() - started)));
  assert.equal(violations.length, 0, "Unexpected network activity blocked");
  assert.equal(browserErrors.length, 0, "Browser errors occurred");
  assert(!streamFailure, "Actual SSE failed");
  console.log(`Recorded: ${title}`);
}

try {
  await page.goto(base.href);
  const tokenInput = page.getByLabel("Operator token");
  assert.equal(await tokenInput.getAttribute("type"), "password");
  // Authentication occurs before capture; no input value or token is logged.
  await tokenInput.fill(token);
  await page
    .getByRole("button", { name: "Connect console", exact: true })
    .click();
  await page.locator(".mode-pill.replay").waitFor();
  assert.equal(await tokenInput.count(), 0);
  assert.equal(
    await page.evaluate(() => localStorage.length + sessionStorage.length),
    0,
  );
  await household("sarah");
  await page.screencast.start({
    path: videoPath,
    size: { width: 1440, height: 1040 },
    fps: 25,
  });
  recording = true;
  await page.screencast.showOverlay(
    '<div style="position:fixed;bottom:0;left:0;right:0;padding:12px 24px;background:#163e36;color:white;font:600 17px sans-serif;text-align:center;pointer-events:none;z-index:2147483647">AUTHORED REPLAY · No phone calls or model inference · Actual local actor, SQLite &amp; SSE</div>',
  );
  const started = Date.now();
  await segment(
    "FirstRing · authored replay",
    "Local behavior demonstration. Live phone and provider gates remain pending.",
    4000,
  );
  await segment(
    "Sarah: an ordinary call",
    "Authored consent input; both intended conference participants are required.",
    10000,
    async () => {
      const session = await replay(
        "Replay a benign call",
        (s) => s.state === "connected",
      );
      assert.equal(session.household_id, "sarah");
      assert.equal(session.offer?.status, "accepted");
    },
  );
  let risky;
  let capsule;
  const previousVersion = Math.max(
    0,
    ...(await getState()).tactics.map((t) => t.version),
  );
  await segment(
    "The request changes. Consent is withdrawn.",
    "An obsolete acceptance is rejected; the separate family leg acknowledges.",
    13000,
    async () => {
      risky = await replay(
        "Replay revocation",
        (s) =>
          s.offer?.status === "revoked" &&
          s.legs.some((leg) => leg.role === "family" && leg.acknowledged),
      );
      assert.equal(risky.household_id, "sarah");
      await page.getByText("Acknowledged by keypad", { exact: true }).waitFor();
      const stale = page
        .locator(".receipt-list")
        .getByText("Old acceptance rejected", { exact: true });
      await stale.waitFor({ state: "attached" });
      await sleep(3500);
      await page.locator(".diagnostics > summary").click();
      await stale.scrollIntoViewIfNeeded();
      await sleep(2300);
      await page.locator(".diagnostics > summary").click();
      await page.evaluate(() => scrollTo({ top: 0, behavior: "instant" }));
    },
  );
  await segment(
    "Wait for Maya’s applied receipt",
    "A stored tactic is shared only as categories, with an actual subscriber version.",
    6000,
    async () => {
      capsule = await waitFor(
        () =>
          receipts.find(
            (event) =>
              event.type === "tactic.created" &&
              event.payload.version > previousVersion,
          )?.payload,
        "new tactic publication",
      );
      await waitFor(
        () =>
          receipts.some(
            (event) =>
              event.type === "community.applied" &&
              event.payload.household_id === "maya" &&
              event.payload.capsule_id === capsule.id &&
              event.payload.version === capsule.version,
          ),
        "Maya’s actual community.applied event",
      );
      await waitFor(
        async () =>
          (await getState()).community.some(
            (h) =>
              h.household_id === "maya" &&
              h.last_applied_version >= capsule.version,
          ),
        "stored subscriber receipt",
      );
      await page.locator(".community-strip").scrollIntoViewIfNeeded();
    },
  );
  await segment(
    "Maya: similar tactic, a different caller",
    "Match the requested action and context. This is not identity matching or retraining.",
    10000,
    async () => {
      await household("maya");
      const session = await replay(
        "Replay revocation",
        (s) =>
          s.household_id === "maya" &&
          s.legs.some((leg) => leg.role === "family" && leg.acknowledged),
      );
      await waitFor(
        () =>
          receipts.some(
            (event) =>
              event.type === "tactic.matched" &&
              event.session_id === session.id &&
              event.payload.matches.some(
                (match) =>
                  match.id === capsule.id &&
                  match.household_applied_version >= capsule.version,
              ),
          ),
        "Maya’s actual tactic match",
      );
      await page
        .getByRole("heading", {
          name: "Similar tactic retrieved on this call",
          exact: true,
        })
        .waitFor();
      await page.locator(".match-receipts").scrollIntoViewIfNeeded();
      await page.screenshot({ path: imagePath, fullPage: true });
    },
  );
  await segment(
    "Control: a benign call from a new number",
    "An appointment reference and a warning about codes do not request a secret.",
    10000,
    async () => {
      const session = await replay(
        "New-number control",
        (s) => s.household_id === "maya" && s.turns.length > 0,
      );
      assert.equal(session.blocked, false);
      assert.equal(session.offer, null);
      assert.equal(session.legs.length, 0);
      assert.equal(await page.locator(".match-receipts article").count(), 0);
    },
  );
  await segment(
    "Replay verified. Phone proof still pending.",
    "Authored inputs exercised real local state, consent checks, receipts and subscriber application.",
    3000,
  );
  assert(authenticatedStream, "Browser did not connect to authenticated SSE");
  await getState();
  await page.screencast.stop();
  recording = false;
  console.log(
    `PASS: ${(Date.now() - started) / 1000}s authored replay; no phone calls, model requests or mocked responses. Files: .tools/artifacts/firstring-replay.webm and .tools/artifacts/firstring-replay.png`,
  );
} finally {
  if (recording) await page.screencast.stop();
  controller.abort();
  await streamReader.cancel().catch(() => {});
  await streamTask;
  await browser.close();
}
