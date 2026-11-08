import { z } from 'zod';
import type { CallRecord } from './calls';

/**
 * Callers: the calls each presented number made and that number's reported
 * history. The server groups by an HMAC key of the presented number and only
 * returns a two-digit suffix; the browser never sees or stores the number and
 * never computes a score.
 */

export const callerReputationSchema = z.object({
  level: z.string(),
  reports: z.number().int().nonnegative(),
  scam_categories: z.array(z.string()).default([]),
  tactic_ids: z.array(z.string()).default([]),
  first_reported_at: z.string().nullable().optional(),
  last_reported_at: z.string().nullable().optional(),
  expires_at: z.string().nullable().optional(),
  correction: z.string().default('none'),
  demo_household_count: z.number().int().nonnegative().default(0),
  verified_independent_households: z.number().int().nonnegative().default(0),
  version: z.number().int().nonnegative().default(0),
  explanation: z.string().optional(),
});
export type CallerReputation = z.infer<typeof callerReputationSchema>;

export const callerItemSchema = z.object({
  number_key: z.string().min(8),
  display_suffix: z.string(),
  session_ids: z.array(z.string()),
  reputation: callerReputationSchema.nullable(),
});
export type CallerItem = z.infer<typeof callerItemSchema>;

export const callersResponseSchema = z.object({
  items: z.array(callerItemSchema),
  scope: z.string().optional(),
});

export const reputationReportSchema = z.object({
  report_id: z.string(),
  created_at: z.string(),
  expires_at: z.string(),
  household_id: z.string(),
  retracted: z.boolean(),
  scam_category: z.string().nullable().optional(),
  source_turn_version: z.number().optional(),
});
export type ReputationReport = z.infer<typeof reputationReportSchema>;

export const callerDetailSchema = z.object({
  reports: z.array(reputationReportSchema).default([]),
  audit: z
    .array(
      z.object({
        action: z.string(),
        at: z.string(),
        version: z.number().optional(),
        details: z.record(z.string(), z.unknown()).optional(),
      }),
    )
    .default([]),
});
export type CallerDetail = z.infer<typeof callerDetailSchema>;

export type Standing = 'clear' | 'reported' | 'warned' | 'corrected';

export interface Caller {
  /** URL-safe short key (first 12 hex of the HMAC); "unknown" groups calls with no captured number. */
  id: string;
  numberKey?: string;
  label: string;
  captured: boolean;
  calls: CallRecord[];
  firstAt?: string;
  lastAt?: string;
  blocked: number;
  households: string[];
  reputation: CallerReputation | null;
  standing: Standing;
}

export const STANDING_LABEL: Record<Standing, string> = {
  clear: 'No reports',
  reported: 'Reported',
  warned: 'Warned by several lines',
  corrected: 'Report withdrawn',
};

export const CATEGORY_LABEL: Record<string, string> = {
  authentication_code_extraction: 'Asked for a code, password or SSN',
  remote_access_request: 'Asked to control a computer',
  payment_diversion: 'Asked for a payment or gift cards',
};

export function categoryLabel(value: string): string {
  return CATEGORY_LABEL[value] ?? value.replaceAll('_', ' ');
}

function standing(rep: CallerReputation | null): Standing {
  if (!rep || rep.reports === 0) return rep?.correction === 'removed' ? 'corrected' : 'clear';
  if (rep.correction === 'removed') return 'corrected';
  return rep.level === 'reported' ? 'reported' : 'warned';
}

function time(record: CallRecord): number {
  const t = Date.parse(record.startedAt ?? record.lastActivityAt ?? '');
  return Number.isNaN(t) ? 0 : t;
}

/** Joins the server's caller groups with the call records; calls with no captured number form one group. */
export function buildCallers(items: readonly CallerItem[], calls: readonly CallRecord[]): Caller[] {
  const byId = new Map(calls.map((c) => [c.id, c]));
  const claimed = new Set<string>();
  const callers: Caller[] = items.map((item) => {
    const own = item.session_ids
      .map((id) => byId.get(id))
      .filter((c): c is CallRecord => !!c)
      .sort((a, b) => time(b) - time(a));
    for (const id of item.session_ids) claimed.add(id);
    return summarize({
      id: item.number_key.slice(0, 12),
      numberKey: item.number_key,
      label: `Number ending ${item.display_suffix}`,
      captured: true,
      calls: own,
      reputation: item.reputation,
    });
  });
  const rest = calls.filter((c) => !claimed.has(c.id)).sort((a, b) => time(b) - time(a));
  callers.sort((a, b) => Date.parse(b.lastAt ?? '') - Date.parse(a.lastAt ?? '') || 0);
  if (rest.length)
    callers.push(
      summarize({
        id: 'unknown',
        label: 'Number not captured',
        captured: false,
        calls: rest,
        reputation: null,
      }),
    );
  return callers;
}

function summarize(
  base: Pick<Caller, 'id' | 'numberKey' | 'label' | 'captured' | 'calls' | 'reputation'>,
): Caller {
  const sorted = base.calls;
  return {
    ...base,
    lastAt: sorted[0]?.startedAt ?? sorted[0]?.lastActivityAt,
    firstAt: sorted.at(-1)?.startedAt ?? sorted.at(-1)?.lastActivityAt,
    blocked: sorted.filter((c) => c.outcome === 'blocked').length,
    households: [...new Set(sorted.map((c) => c.name))],
    standing: standing(base.reputation),
  };
}

export async function fetchCallers(signal?: AbortSignal): Promise<CallerItem[]> {
  const response = await fetch('/api/callers', { cache: 'no-store', signal });
  if (!response.ok) throw new Error(`The server answered ${response.status}.`);
  return callersResponseSchema.parse(await response.json()).items;
}

export async function fetchCallerDetail(numberKey: string): Promise<CallerDetail | null> {
  const response = await fetch(`/api/reputation/${encodeURIComponent(numberKey)}`, {
    cache: 'no-store',
  });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`The server answered ${response.status}.`);
  return callerDetailSchema.parse(await response.json());
}

/** Fictional callers for the demo: suffixes and reports match the demo reputation fixtures. */
export function demoCallerItems(now = Date.now()): CallerItem[] {
  const iso = (hoursAgo: number) => new Date(now - hoursAgo * 3600_000).toISOString();
  const expires = new Date(now + 14 * 24 * 3600_000).toISOString();
  const base = {
    correction: 'none',
    verified_independent_households: 0,
    tactic_ids: [] as string[],
  };
  return [
    {
      number_key: 'demo0000000741aa',
      display_suffix: '41',
      session_ids: ['demo-call-code', 'staged-branch'],
      reputation: {
        ...base,
        level: 'reported',
        reports: 1,
        scam_categories: ['authentication_code_extraction'],
        first_reported_at: iso(1),
        last_reported_at: iso(1),
        expires_at: expires,
        demo_household_count: 1,
        version: 1,
        correction: 'requested',
      },
    },
    {
      number_key: 'demo0000000707bb',
      display_suffix: '07',
      session_ids: ['demo-call-giftcard', 'staged-maya'],
      reputation: {
        ...base,
        level: 'corroborated',
        reports: 2,
        scam_categories: ['payment_diversion'],
        first_reported_at: iso(30),
        last_reported_at: iso(4),
        expires_at: expires,
        demo_household_count: 2,
        version: 2,
      },
    },
    {
      number_key: 'demo0000000052cc',
      display_suffix: '52',
      session_ids: ['demo-call-dentist'],
      reputation: null,
    },
    {
      number_key: 'demo0000000088dd',
      display_suffix: '88',
      session_ids: ['demo-call-unclear', 'demo-call-failure'],
      reputation: null,
    },
  ];
}

export function demoCallerDetail(item: CallerItem): CallerDetail {
  const rep = item.reputation;
  if (!rep) return { reports: [], audit: [] };
  const households = ['sarah', 'maya'];
  const reports = Array.from({ length: rep.reports }, (_, i) => ({
    report_id: `${item.number_key}-r${i + 1}`,
    created_at: i === 0 ? (rep.first_reported_at ?? '') : (rep.last_reported_at ?? ''),
    expires_at: rep.expires_at ?? '',
    household_id: households[i % households.length],
    retracted: false,
    scam_category: rep.scam_categories[0] ?? null,
  }));
  return {
    reports,
    audit: reports.map((r, i) => ({ action: 'report', at: r.created_at, version: i + 1 })),
  };
}
