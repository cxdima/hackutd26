import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ProfileEditor } from "./ProfileEditor";
const profile = {
  household_id: "sarah",
  version: 0,
  display_name: "Sarah",
  greeting: "Hi, thanks for calling. What’s the call about?",
  tts_provider: "ElevenLabs",
  voice_id: "cgSgspJ2msm6clMCkdW9-v4",
  locale: "en-US",
  timezone: "America/Chicago",
  source: "configured",
  applies_to: "new_calls",
};
afterEach(() => vi.unstubAllGlobals());

it("saves only editable fields with authentication and shows the returned version for new calls", async () => {
  const requests: { url: string; init?: RequestInit }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      requests.push({ url, init });
      return new Response(
        JSON.stringify(
          init?.method === "PUT"
            ? { ...profile, version: 1, voice_id: "JBFqnCBsd6RMkjVDRZzb-v4" }
            : profile,
        ),
        { status: 200 },
      );
    }),
  );
  render(<ProfileEditor token="private-test" household="sarah" />);
  expect(requests).toHaveLength(0);
  fireEvent.click(screen.getByRole("button", { name: "Edit call profile" }));
  await screen.findByLabelText("Voice preset");
  fireEvent.change(screen.getByLabelText("Voice preset"), {
    target: { value: "JBFqnCBsd6RMkjVDRZzb-v4" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save for new calls" }));
  await screen.findByText(/Saved version 1/);
  expect(requests[1].url).toBe("/api/profile/sarah");
  expect(requests[1].init?.headers).toEqual({
    Authorization: "Bearer private-test",
    "Content-Type": "application/json",
  });
  const body = JSON.parse(requests[1].init?.body as string);
  expect(body).toEqual({
    display_name: "Sarah",
    greeting: profile.greeting,
    tts_provider: "ElevenLabs",
    voice_id: "JBFqnCBsd6RMkjVDRZzb-v4",
    locale: "en-US",
    timezone: "America/Chicago",
  });
  expect(
    screen.getByText(/Active calls keep their captured profile/),
  ).toBeInTheDocument();
  expect(screen.queryByText(/audio heard/i)).not.toBeInTheDocument();
});

it("does not claim a saved version after a rejected update", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (_url: string, init?: RequestInit) =>
      init?.method === "PUT"
        ? new Response("{}", { status: 409 })
        : new Response(JSON.stringify(profile), { status: 200 }),
    ),
  );
  render(<ProfileEditor token="private-test" household="sarah" />);
  fireEvent.click(screen.getByRole("button", { name: "Edit call profile" }));
  await screen.findByLabelText("Voice preset");
  fireEvent.click(screen.getByRole("button", { name: "Save for new calls" }));
  await waitFor(() =>
    expect(screen.getByRole("alert")).toHaveTextContent("not saved"),
  );
  expect(screen.queryByText(/Saved version/)).not.toBeInTheDocument();
});
