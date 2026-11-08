import { useEffect, useRef, useState } from "react";
import { profileSchema } from "./types";
import type { z } from "zod";

type Profile = z.infer<typeof profileSchema>;

export function ProfileEditor({
  token,
  household,
  disabled,
}: {
  token: string;
  household: string;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<Profile | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const request = useRef<AbortController | null>(null);
  useEffect(() => () => request.current?.abort(), []);

  async function exchange(save: boolean) {
    if (busy) return;
    setBusy(true);
    setError("");
    setNotice("");
    const controller = new AbortController();
    request.current = controller;
    try {
      const fields = draft && {
        display_name: draft.display_name,
        greeting: draft.greeting,
        tts_provider: draft.tts_provider,
        voice_id: draft.voice_id,
        locale: draft.locale,
        timezone: draft.timezone,
      };
      const response = await fetch(
        `/api/profile/${encodeURIComponent(household)}`,
        {
          method: save ? "PUT" : "GET",
          headers: {
            Authorization: `Bearer ${token}`,
            ...(save ? { "Content-Type": "application/json" } : {}),
          },
          ...(save ? { body: JSON.stringify(fields) } : {}),
          cache: "no-store",
          signal: AbortSignal.any([
            controller.signal,
            AbortSignal.timeout(10000),
          ]),
        },
      );
      if (!response.ok)
        throw new Error(
          save
            ? "Profile not saved. Use a supported preset and short greeting without links or phone numbers."
            : "Profile unavailable. Check the server and operator token.",
        );
      const profile = profileSchema.parse(await response.json());
      if (profile.household_id !== household)
        throw new Error(
          "Profile household did not match. Refresh before editing.",
        );
      if (controller.signal.aborted) return;
      setDraft(profile);
      if (save) setNotice(`Saved version ${profile.version} for new calls.`);
    } catch (failure) {
      if (!controller.signal.aborted)
        setError(
          failure instanceof Error
            ? failure.message
            : "Profile could not be loaded.",
        );
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }

  return (
    <div className="profile-editor">
      <button
        className="quiet-button"
        disabled={disabled}
        aria-expanded={open}
        aria-controls="profile-editor-form"
        onClick={() => {
          setOpen(!open);
          if (!open && !draft) void exchange(false);
        }}
      >
        Edit call profile
      </button>
      {open && (
        <section
          id="profile-editor-form"
          className="profile-form"
          aria-label="Call profile editor"
        >
          <div className="profile-form-heading">
            <strong>Call profile</strong>
            <button
              className="quiet-button"
              onClick={() => setOpen(false)}
              aria-label="Close profile editor"
            >
              Close
            </button>
          </div>
          <p>
            Saved settings apply to new calls. Active calls keep their captured
            profile.
          </p>
          {busy && !draft && <p role="status">Loading profile…</p>}
          {error && (
            <p className="profile-error" role="alert">
              {error}
            </p>
          )}
          {notice && <p role="status">{notice}</p>}
          {draft && (
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void exchange(true);
              }}
            >
              <label htmlFor="profile-name">Display name</label>
              <input
                id="profile-name"
                value={draft.display_name}
                required
                maxLength={60}
                onChange={(event) =>
                  setDraft({ ...draft, display_name: event.target.value })
                }
              />
              <label htmlFor="profile-greeting">Opening greeting</label>
              <textarea
                id="profile-greeting"
                value={draft.greeting}
                required
                maxLength={240}
                rows={3}
                onChange={(event) =>
                  setDraft({ ...draft, greeting: event.target.value })
                }
              />
              <label htmlFor="profile-voice">Voice preset</label>
              <select
                id="profile-voice"
                value={draft.voice_id}
                onChange={(event) =>
                  setDraft({ ...draft, voice_id: event.target.value })
                }
              >
                <option value="cgSgspJ2msm6clMCkdW9-v4">
                  Jessica · warm, conversational
                </option>
                <option value="JBFqnCBsd6RMkjVDRZzb-v4">
                  George · warm storyteller
                </option>
                <option value="21m00Tcm4TlvDq8ikWAM-v4">
                  Janet · legacy voice ID
                </option>
              </select>
              <label htmlFor="profile-timezone">Timezone</label>
              <select
                id="profile-timezone"
                value={draft.timezone}
                onChange={(event) =>
                  setDraft({ ...draft, timezone: event.target.value })
                }
              >
                <option value="America/Chicago">
                  Central · America/Chicago
                </option>
                <option value="America/New_York">
                  Eastern · America/New_York
                </option>
                <option value="America/Denver">
                  Mountain · America/Denver
                </option>
                <option value="America/Los_Angeles">
                  Pacific · America/Los_Angeles
                </option>
                <option value="UTC">UTC</option>
              </select>
              <small>
                ElevenLabs v4 · English (US) · Configured version{" "}
                {draft.version}. Voice selection is configuration, not a
                listening receipt.
              </small>
              <button
                className="primary-button"
                type="submit"
                disabled={busy || disabled}
              >
                {busy ? "Saving…" : "Save for new calls"}
              </button>
            </form>
          )}
          {!draft && !busy && error && (
            <button
              className="quiet-button"
              onClick={() => void exchange(false)}
            >
              Retry profile load
            </button>
          )}
        </section>
      )}
    </div>
  );
}
