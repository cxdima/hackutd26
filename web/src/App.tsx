import { useEffect, useRef, useState } from "react";
import {
  ArrowRight,
  KeyRound,
  LockKeyhole,
  LogOut,
  Radio,
  ShieldCheck,
} from "lucide-react";
import { ConsoleView } from "./ConsoleView";
import { consumeEvents } from "./events";
import { mergeEvents } from "./model";
import { stateSchema } from "./types";
import type { CallEvent, ConsoleState } from "./types";

function responseError(status: number) {
  if (status === 401 || status === 403)
    return "The console token was not accepted. Check the server’s operator token.";
  if (status === 409)
    return "The action is not available in the current call state. Refresh the state and try again.";
  if (status === 503)
    return "The service is waiting for provider setup. Check Evidence & setup below.";
  return "The request could not be completed. Check that the FirstRing server is running.";
}
async function loadState(token: string, signal?: AbortSignal) {
  const response = await fetch("/api/state", {
    headers: { Authorization: `Bearer ${token}` },
    signal: signal
      ? AbortSignal.any([signal, AbortSignal.timeout(10000)])
      : AbortSignal.timeout(10000),
    cache: "no-store",
  });
  if (!response.ok) throw new Error(responseError(response.status));
  const parsed = stateSchema.safeParse(await response.json());
  if (!parsed.success)
    throw new Error(
      "The server returned an incompatible console state. Check that the server and console versions match.",
    );
  return parsed.data;
}

export default function App() {
  const [token, setToken] = useState("");
  const [draft, setDraft] = useState("");
  const [state, setState] = useState<ConsoleState | null>(null);
  const [events, setEvents] = useState<CallEvent[]>([]);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [stale, setStale] = useState(false);
  const authGeneration = useRef(0);

  useEffect(() => {
    if (!token) return;
    const controller = new AbortController();
    let lastSequence = 0;
    let refreshRunning = false;
    let refreshAgain = false;
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
    async function refresh() {
      if (refreshRunning) {
        refreshAgain = true;
        return;
      }
      refreshRunning = true;
      do {
        refreshAgain = false;
        try {
          const fresh = await loadState(token, controller.signal);
          if (!controller.signal.aborted) setState(fresh);
        } catch {
          if (!controller.signal.aborted) setStale(true);
        }
      } while (refreshAgain && !controller.signal.aborted);
      refreshRunning = false;
    }
    async function connectEvents() {
      const opening = new AbortController();
      const headerTimeout = setTimeout(() => opening.abort(), 10000);
      try {
        const response = await fetch("/api/events", {
          headers: {
            Authorization: `Bearer ${token}`,
            "Last-Event-ID": String(lastSequence),
            Accept: "text/event-stream",
          },
          signal: AbortSignal.any([controller.signal, opening.signal]),
          cache: "no-store",
        });
        clearTimeout(headerTimeout);
        if (!response.ok) throw new Error(responseError(response.status));
        if (!controller.signal.aborted) setStale(false);
        void refresh();
        await consumeEvents(response, (event) => {
          if (controller.signal.aborted) return;
          lastSequence = Math.max(lastSequence, event.sequence);
          setEvents((previous) => mergeEvents(previous, event));
          void refresh();
        });
      } catch {
        /* A receipt stream is recoverable; the last snapshot remains explicit. */
      } finally {
        clearTimeout(headerTimeout);
      }
      if (!controller.signal.aborted) {
        setStale(true);
        reconnectTimer = setTimeout(() => {
          void connectEvents();
        }, 2000);
      }
    }
    void connectEvents();
    return () => {
      controller.abort();
      clearTimeout(reconnectTimer);
    };
  }, [token]);

  async function connect(event: React.FormEvent) {
    event.preventDefault();
    if (!draft.trim() || busy) return;
    setBusy("connect");
    setError("");
    setNotice("");
    const generation = ++authGeneration.current;
    try {
      const cleanToken = draft.trim();
      const initial = await loadState(cleanToken);
      if (generation !== authGeneration.current) return;
      setState(initial);
      setEvents([]);
      setToken(cleanToken);
      setDraft("");
      setStale(false);
    } catch (failure) {
      if (generation === authGeneration.current)
        setError(
          failure instanceof Error ? failure.message : "Connection failed.",
        );
    } finally {
      if (generation === authGeneration.current) setBusy(null);
    }
  }
  function disconnect() {
    authGeneration.current++;
    setToken("");
    setDraft("");
    setState(null);
    setEvents([]);
    setError("");
    setNotice("");
    setBusy(null);
  }
  async function action(path: string, body: Record<string, unknown>) {
    if (busy || !token) return;
    const generation = authGeneration.current;
    setBusy(path);
    setError("");
    setNotice("");
    try {
      const response = await fetch(path, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(15000),
      });
      if (!response.ok) throw new Error(responseError(response.status));
      const fresh = await loadState(token);
      if (generation !== authGeneration.current) return;
      setState(fresh);
      setNotice("Request sent. The console will show its received outcome.");
    } catch (failure) {
      if (generation === authGeneration.current)
        setError(
          failure instanceof Error
            ? failure.message
            : "The action did not return a result. Check the received events before trying again.",
        );
    } finally {
      if (generation === authGeneration.current) setBusy(null);
    }
  }

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main">
        Skip to call space
      </a>
      <header className="app-header">
        <a className="brand" href="/" aria-label="FirstRing home">
          <span className="brand-mark" aria-hidden="true">
            <span />
            <span />
            <span />
          </span>
          <span>
            FirstRing<span className="brand-period">.</span>
          </span>
        </a>
        <span className="brand-promise">A familiar first hello.</span>
        <div className="header-right">
          {token ? (
            <>
              <span className={`connection-label ${stale ? "paused" : ""}`}>
                <span className="tiny-dot" />
                {stale ? "Updates paused" : "Console connected"}
              </span>
              <button className="quiet-button disconnect" onClick={disconnect}>
                <LogOut size={15} />
                Lock console
              </button>
            </>
          ) : (
            <span className="header-fiction">
              <ShieldCheck size={16} /> Fictional household demo
            </span>
          )}
        </div>
      </header>
      {error && (
        <div className="notice error" role="alert">
          {error}
        </div>
      )}
      {notice && (
        <div className="notice success" role="status">
          {notice}
        </div>
      )}
      {state && token ? (
        <ConsoleView
          state={state}
          events={events}
          stale={stale}
          busy={busy}
          onAction={action}
        />
      ) : (
        <main id="main" className="welcome">
          <section className="welcome-story">
            <span className="welcome-icon">
              <LockKeyhole size={29} strokeWidth={1.4} />
            </span>
            <h1>
              A little space
              <br />
              before you answer.
            </h1>
            <p>
              FirstRing listens first, gives you a private choice, and keeps a
              trusted person one call away.
            </p>
            <div className="welcome-flow">
              <span>
                <Radio size={18} /> Hear the request
              </span>
              <i />
              <span>
                <LockKeyhole size={18} /> Choose privately
              </span>
              <i />
              <span>
                <ShieldCheck size={18} /> Take the next step
              </span>
            </div>
          </section>
          <section className="connect-card" aria-labelledby="connect-heading">
            <span className="connect-icon">
              <KeyRound size={21} />
            </span>
            <h2 id="connect-heading">Open the call space</h2>
            <p>
              Use the operator token configured on your FirstRing server to see
              call events and test controls.
            </p>
            <form
              onSubmit={(event) => {
                void connect(event);
              }}
            >
              <label htmlFor="operator-token">Operator token</label>
              <input
                id="operator-token"
                type="password"
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                autoComplete="off"
                spellCheck={false}
                required
                aria-describedby="token-help"
              />
              <p id="token-help" className="input-help">
                Kept only in this tab. Cleared when you lock or reload.
              </p>
              <button
                className="primary connect-button"
                disabled={!draft.trim() || !!busy}
              >
                {busy ? "Connecting…" : "Connect console"}
                <ArrowRight size={17} />
              </button>
            </form>
            <div className="connect-foot">
              <LockKeyhole size={13} /> No call is placed by connecting.
            </div>
          </section>
          <div className="welcome-note">
            <span>Built around a person’s choice.</span>
            <p>
              Screening ends when an approved human handoff connects. Caller
              identity remains unverified.
            </p>
          </div>
        </main>
      )}
      <footer className="app-footer">
        <span>
          FirstRing <span className="footer-rule" /> HackUTD 2026
        </span>
        <p>Fictional people. Visible evidence. A choice that can change.</p>
      </footer>
    </div>
  );
}
