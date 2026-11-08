import { z } from 'zod';

/**
 * Owner voice and call recordings. These calls go to the FirstRing server on
 * this computer (the server trusts its own machine; no token in the browser).
 */

export const cloneStatusSchema = z.object({
  household_id: z.string(),
  state: z.string(),
  speaker_name: z.string().nullable().optional(),
  expires_at: z.string().nullable().optional(),
  voice_ready: z.boolean(),
  active: z.boolean(),
  sample_seconds: z.number().nullable().optional(),
  error: z.string().nullable().optional(),
  passage: z.string(),
  provider: z.string().optional(),
  provider_configured: z.boolean().optional(),
});
export type CloneStatus = z.infer<typeof cloneStatusSchema>;

async function json<T>(response: Response, schema: z.ZodType<T>): Promise<T> {
  const body: unknown = await response.json().catch(() => ({}));
  if (!response.ok) {
    const message = body && typeof body === 'object' && 'error' in body ? String(body.error) : '';
    throw new Error(message || 'The FirstRing server did not accept that.');
  }
  return schema.parse(body);
}

export async function cloneStatus(household?: string): Promise<CloneStatus> {
  const query = household ? `?household=${encodeURIComponent(household)}` : '';
  return json(await fetch(`/api/voice/clone${query}`, { cache: 'no-store' }), cloneStatusSchema);
}

async function post(path: string, body: unknown): Promise<CloneStatus> {
  return json(
    await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      cache: 'no-store',
    }),
    cloneStatusSchema,
  );
}

export const armClone = (speaker_name: string, household_id?: string) =>
  post('/api/voice/clone/arm', { speaker_name, consent: true, household_id });
export const activateClone = (household_id?: string) =>
  post('/api/voice/clone/activate', { household_id });
export const deactivateClone = (household_id?: string) =>
  post('/api/voice/clone/deactivate', { household_id });
export const deleteClone = (household_id?: string) =>
  post('/api/voice/clone/delete', { household_id });

/** A short sample sentence in the cloned voice, as a blob URL. */
export async function clonePreview(household?: string): Promise<string> {
  const query = household ? `?household=${encodeURIComponent(household)}` : '';
  const response = await fetch(`/api/voice/clone/preview${query}`, { cache: 'no-store' });
  if (!response.ok) throw new Error('The preview could not be generated.');
  return URL.createObjectURL(await response.blob());
}

export const recordingInfoSchema = z.object({
  available: z.boolean(),
  recording: z
    .object({ duration_seconds: z.number().nullable().optional(), channels: z.number().optional() })
    .nullable()
    .optional(),
});

export async function recordingInfo(sessionId: string) {
  const response = await fetch(`/api/session/${encodeURIComponent(sessionId)}/recording/info`, {
    cache: 'no-store',
  });
  if (!response.ok) return { available: false } as const;
  return recordingInfoSchema.parse(await response.json());
}

export async function recordingAudio(sessionId: string): Promise<string> {
  const response = await fetch(`/api/session/${encodeURIComponent(sessionId)}/recording`, {
    cache: 'no-store',
  });
  if (!response.ok) throw new Error('The recording could not be loaded.');
  return URL.createObjectURL(await response.blob());
}
