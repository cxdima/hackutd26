/**
 * Optional voice enrollment (proposed contract: frontend/docs/voice-enrollment-contract.md).
 * The server currently reports voice cloning as disabled, so the live console
 * shows that state and the flow runs only against an explicit, in-browser
 * example that never uploads anything.
 */

export type EnrollmentState =
  | 'none'
  | 'consented'
  | 'sample_ready'
  | 'processing'
  | 'verifying'
  | 'ready'
  | 'applied'
  | 'failed'
  | 'revoked';

export interface SampleRequirements {
  minSeconds: number;
  maxSeconds: number;
  formats: string[];
}

/** Example requirements for the in-browser walkthrough; a real provider sets its own. */
export const EXAMPLE_REQUIREMENTS: SampleRequirements = {
  minSeconds: 20,
  maxSeconds: 120,
  formats: ['audio/wav', 'audio/x-wav', 'audio/mpeg', 'audio/mp4', 'audio/webm', 'audio/ogg'],
};

export const CONSENT_STATEMENTS = [
  'This is my own voice, and I agree to FirstRing creating an assistant voice that sounds like me.',
  'I understand a voice can be recorded and imitated by others. A familiar-sounding voice never proves who is calling.',
  'I can turn this voice off and delete it at any time.',
] as const;

export interface SampleCheck {
  ok: boolean;
  message: string;
}

/** Validates a sample's type and length against the requirements. Pure; no upload. */
export function checkSample(
  type: string,
  seconds: number | undefined,
  req: SampleRequirements = EXAMPLE_REQUIREMENTS,
): SampleCheck {
  const base = type.split(';')[0].trim().toLowerCase();
  if (!req.formats.includes(base))
    return { ok: false, message: 'Use a WAV, MP3, M4A, WebM or OGG audio file.' };
  if (seconds === undefined || !Number.isFinite(seconds))
    return { ok: false, message: 'The recording length could not be read. Try another file.' };
  if (seconds < req.minSeconds)
    return {
      ok: false,
      message: `The sample is ${Math.round(seconds)} seconds. Record at least ${req.minSeconds} seconds of natural speech.`,
    };
  if (seconds > req.maxSeconds)
    return {
      ok: false,
      message: `The sample is ${Math.round(seconds)} seconds. Keep it under ${req.maxSeconds} seconds.`,
    };
  return { ok: true, message: `Sample length ${Math.round(seconds)} seconds. Ready.` };
}

export interface VoiceOption {
  id: string;
  label: string;
  provider: string;
  kind: 'preset' | 'own';
}
