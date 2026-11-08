import { z } from 'zod';

/**
 * Reported-number history (proposed contract: frontend/docs/reputation-contract.md).
 * The browser never computes a score or a band: it displays what the server
 * returns, and only when the server advertises a "reputation" capability.
 * No history means "No reports yet", never "safe".
 */

export const reputationBandSchema = z.string();

export const reputationSchema = z.object({
  version: z.number().int().nonnegative(),
  /** "demonstration" reports are fictional and scoped to the demo; "production" are real. */
  scope: z.string(),
  /** Server-formatted, never the full number, e.g. "Number ending 34". */
  number_label: z.string(),
  band: reputationBandSchema,
  band_label: z.string().optional(),
  /** Server-computed; shown only when present. Never derived in the browser. */
  score: z.number().nullable().optional(),
  score_label: z.string().nullable().optional(),
  explanation: z.string(),
  independent_households: z.number().int().nonnegative(),
  verified_independent_households: z.number().int().nonnegative().optional(),
  /** Demo scope: simulated household profiles, never verified independence. */
  demo_household_count: z.number().int().nonnegative().optional(),
  identity_verified: z.literal(false).optional(),
  reports: z.number().int().nonnegative(),
  first_reported_at: z.string().nullable(),
  last_reported_at: z.string().nullable(),
  expires_at: z.string().nullable(),
  correction: z
    .object({
      state: z.string(),
      updated_at: z.string().nullable().optional(),
      note: z.string().nullable().optional(),
    })
    .optional(),
  related_tactic_ids: z.array(z.string()).optional(),
});
export type Reputation = z.infer<typeof reputationSchema>;

export type ReputationResult =
  | { kind: 'unavailable'; reason: string }
  | { kind: 'ok'; data: Reputation; fixture: boolean }
  | { kind: 'error'; message: string };

export const REPUTATION_UNAVAILABLE =
  'Not connected. This server does not provide reported-number history. FirstRing still screens each request and uses shared tactic warnings.';

export function hasNoReports(rep: Reputation): boolean {
  return rep.reports === 0 || rep.band === 'no_reports';
}

const BAND_TEXT: Record<string, string> = {
  no_reports: 'No reports yet',
  reported: 'Reported by one household',
  corroborated: 'Reported by more than one household',
  warning: 'Repeated warnings',
};

/** The server's own label when given; a neutral reading of its band otherwise. */
export function bandText(rep: Reputation): string {
  if (hasNoReports(rep)) return 'No reports yet';
  return rep.band_label ?? BAND_TEXT[rep.band] ?? rep.band.replaceAll('_', ' ');
}

export function correctionText(rep: Reputation): string | undefined {
  const state = rep.correction?.state;
  switch (state) {
    case undefined:
    case 'none':
      return undefined;
    case 'requested':
      return 'A correction was requested. The level is frozen while it is reviewed.';
    case 'upheld':
      return 'A correction was reviewed and the reports were kept.';
    case 'removed':
      return 'A correction was accepted. The warning was removed.';
    default:
      return `Correction: ${state.replaceAll('_', ' ')}.`;
  }
}

/** "demo" (current backend) and "demonstration" both mean simulated, fictional reports. */
export function isDemoScope(rep: Reputation): boolean {
  return rep.scope === 'demo' || rep.scope === 'demonstration';
}

/** Fictional, demonstration-scoped reputation for demo calls only. */
export function demoReputation(sessionId: string, now = Date.now()): Reputation | undefined {
  const iso = (hoursAgo: number) => new Date(now - hoursAgo * 3600_000).toISOString();
  const base = {
    version: 3,
    scope: 'demo',
    identity_verified: false as const,
    verified_independent_households: 0,
    score: null,
    score_label: null,
    correction: { state: 'none' },
  };
  switch (sessionId) {
    case 'demo-call-giftcard':
    case 'staged-maya':
      return {
        ...base,
        number_label: 'Fictional number ending 07',
        band: 'corroborated',
        band_label: 'Reported by 2 demo profiles',
        explanation:
          'Two fictional demo profiles reported this number after calls that asked for gift cards. Demo profiles simulate households; verified independence is zero. Reports expire unless renewed.',
        independent_households: 0,
        demo_household_count: 2,
        reports: 2,
        first_reported_at: iso(30),
        last_reported_at: iso(4),
        expires_at: new Date(now + 14 * 24 * 3600_000).toISOString(),
        related_tactic_ids: ['demo-tactic-giftcard'],
      };
    case 'demo-call-code':
    case 'staged-branch':
      return {
        ...base,
        number_label: 'Fictional number ending 41',
        band: 'reported',
        band_label: 'Reported by 1 demo profile',
        explanation:
          'One fictional demo profile reported this number. A single report only adds context to that household’s own screening.',
        independent_households: 0,
        demo_household_count: 1,
        reports: 1,
        first_reported_at: iso(1),
        last_reported_at: iso(1),
        expires_at: new Date(now + 14 * 24 * 3600_000).toISOString(),
        correction: { state: 'requested', updated_at: iso(0.5), note: null },
      };
    default:
      return {
        ...base,
        number_label: 'Fictional number',
        band: 'no_reports',
        explanation:
          'No household has reported this number. That does not mean the caller is safe.',
        independent_households: 0,
        reports: 0,
        first_reported_at: null,
        last_reported_at: null,
        expires_at: null,
      };
  }
}
