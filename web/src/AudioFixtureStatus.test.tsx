import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AudioFixtureStatus } from "./AudioFixtureStatus";

afterEach(() => vi.unstubAllGlobals());
it.each([
  ["family_pickup_benign_george", "Claimed family pickup → routine details"],
  ["family_pickup_risky_george", "Claimed family pickup → sign-in request"],
])(
  "shows %s through read-only status without grading it",
  async (scenario, label) => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        scenario_id: scenario,
        destination: "owned_firstring_number_only",
        test_kind: "automated_audio_caller",
        voice: "George",
        clip_count: 2,
        time_limit_seconds: 90,
        remaining_attempts: 1,
        provider_status: "completed",
        outcome: "audio_exchange_captured_ungraded",
        human_audition: false,
        noise_tested: false,
        verdict: "ungraded",
      }),
    });
    vi.stubGlobal("fetch", fetcher);
    render(<AudioFixtureStatus token="test-only-token" />);
    expect(fetcher).not.toHaveBeenCalled();
    fireEvent.click(
      screen.getByRole("button", { name: "Refresh caller test status" }),
    );
    expect(
      await screen.findByText(label, { exact: false }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("region", { name: "Automated caller status" }),
    ).toHaveTextContent("Ungraded");
    expect(screen.getAllByRole("button")).toHaveLength(1);
    expect(fetcher).toHaveBeenCalledExactlyOnceWith(
      "/api/demo/audio-fixture",
      expect.objectContaining({ method: "GET" }),
    );
  },
);
