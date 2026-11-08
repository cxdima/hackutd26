import { buildTrace, type CallRecord, type TraceRound, type TraceStep } from './calls';
import { modelObservationSchema } from './schemas';

/**
 * One row per caller turn for the call-detail decision table: what was said,
 * what the AI understood, the rule decision and what FirstRing actually did.
 * Everything comes from the call's receipts via the decision trace.
 */
export interface TurnRow {
  index: number;
  at?: string;
  said?: string;
  understood: string[];
  decision?: { verdict: 'protected' | 'allowed' | 'unclear'; text: string };
  proposed: string[];
  did: string[];
  replied: string[];
  modelRequests: number;
  modelMs: number;
  tokens: number;
  failures: string[];
}

function fact(step: TraceStep, label: string): string | undefined {
  const value = step.facts?.find((f) => f.label === label)?.value;
  return value && value !== '—' ? value : undefined;
}

function row(round: TraceRound): TurnRow {
  const out: TurnRow = {
    index: round.index,
    at: round.at,
    said: round.callerText,
    understood: [],
    proposed: [],
    did: [],
    replied: [],
    modelRequests: 0,
    modelMs: 0,
    tokens: 0,
    failures: [],
  };
  if (round.verdict) out.decision = { verdict: round.verdict, text: round.verdictText ?? '' };
  for (const step of round.steps) {
    switch (step.kind) {
      case 'understood': {
        const purpose = fact(step, 'Purpose (model’s words)');
        const request = fact(step, 'Request (model’s words)');
        const ready = fact(step, 'Message ready?');
        const missing = fact(step, 'Still missing');
        if (purpose) out.understood.push(`Purpose: ${purpose}`);
        if (request && request !== purpose) out.understood.push(`Wants: ${request}`);
        if (ready) out.understood.push(ready);
        if (missing) out.understood.push(`Still missing: ${missing}`);
        break;
      }
      case 'rule':
        if (step.event.payload['operation'] === 'assessment' && step.detail)
          out.understood.unshift(step.detail);
        break;
      case 'proposed':
        out.proposed.push(step.title.replace(/^The model proposed: /, '').replace(/\.$/, ''));
        break;
      case 'executed':
        out.did.push(step.title.replace(/\.$/, ''));
        break;
      case 'saved':
      case 'phone':
      case 'community':
      case 'closing':
        out.did.push(step.title.replace(/\.$/, ''));
        break;
      case 'spoke':
        if (step.quote) out.replied.push(step.quote);
        break;
      case 'model': {
        const obs = modelObservationSchema.safeParse(step.event.payload['observation']);
        if (obs.success) {
          out.modelRequests++;
          out.modelMs += obs.data.elapsed_ms;
          out.tokens += obs.data.completion_tokens ?? 0;
        }
        if (step.tone === 'fail') out.failures.push(step.title);
        break;
      }
      case 'failure':
        out.failures.push(step.title);
        break;
    }
  }
  out.understood = [...new Set(out.understood)];
  out.did = [...new Set(out.did)];
  return out;
}

export function turnRows(record: CallRecord): TurnRow[] {
  return buildTrace(record)
    .filter((round) => round.callerText !== undefined || round.index === 0)
    .map(row)
    .filter(
      (r) =>
        r.said !== undefined || r.replied.length > 0 || r.did.length > 0 || r.modelRequests > 0,
    );
}
