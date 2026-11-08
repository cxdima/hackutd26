/** Time formatting in a profile's configured IANA timezone. Invalid input never throws. */

function safeZone(timeZone: string | undefined): string | undefined {
  if (!timeZone) return undefined;
  try {
    new Intl.DateTimeFormat('en-US', { timeZone }).format(0);
    return timeZone;
  } catch {
    return undefined;
  }
}

function parse(value: string | number | undefined): Date | undefined {
  if (value === undefined) return undefined;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? undefined : date;
}

export function formatTime(
  value: string | number | undefined,
  timeZone?: string,
  seconds = false,
): string {
  const date = parse(value);
  if (!date) return 'Time not received';
  return new Intl.DateTimeFormat('en-US', {
    hour: 'numeric',
    minute: '2-digit',
    ...(seconds ? { second: '2-digit' } : {}),
    timeZone: safeZone(timeZone),
  }).format(date);
}

export function formatDateTime(value: string | number | undefined, timeZone?: string): string {
  const date = parse(value);
  if (!date) return 'Time not received';
  return new Intl.DateTimeFormat('en-US', {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    timeZone: safeZone(timeZone),
  }).format(date);
}

export function formatShortDateTime(value: string | number | undefined, timeZone?: string): string {
  const date = parse(value);
  if (!date) return 'Time not received';
  return new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    timeZone: safeZone(timeZone),
  }).format(date);
}

export function formatDay(value: string | number | undefined, timeZone?: string): string {
  const date = parse(value);
  if (!date) return 'Date not received';
  return new Intl.DateTimeFormat('en-US', {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
    timeZone: safeZone(timeZone),
  }).format(date);
}

export function zoneLabel(timeZone: string | undefined, at: string | number = Date.now()): string {
  const zone = safeZone(timeZone);
  if (!zone) return 'local time';
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: zone,
    timeZoneName: 'short',
  }).formatToParts(new Date(at));
  return parts.find((part) => part.type === 'timeZoneName')?.value ?? zone;
}

export function formatDuration(seconds: number | undefined): string {
  if (seconds === undefined || !Number.isFinite(seconds) || seconds < 0) return '—';
  const whole = Math.round(seconds);
  const minutes = Math.floor(whole / 60);
  const rest = whole % 60;
  return minutes ? `${minutes} min ${rest.toString().padStart(2, '0')} s` : `${rest} s`;
}

export function formatMs(ms: number | undefined): string {
  if (ms === undefined || !Number.isFinite(ms)) return '—';
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)} s` : `${Math.round(ms)} ms`;
}

export function relativeAge(value: string | undefined, now: number): string {
  const date = parse(value);
  if (!date) return '';
  const minutes = Math.round((now - date.getTime()) / 60000);
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.round(hours / 24);
  return `${days} day${days === 1 ? '' : 's'} ago`;
}

export function relativeUntil(value: string | undefined, now: number): string {
  const date = parse(value);
  if (!date) return '';
  const minutes = Math.round((date.getTime() - now) / 60000);
  if (minutes <= 0) return 'expired';
  if (minutes < 60) return `in ${minutes} min`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `in ${hours} h`;
  return `in ${Math.round(hours / 24)} days`;
}

/** "+12145550134" → "+1 (214) 555-0134". Anything else is returned unchanged. */
export function formatPhone(e164: string): string {
  const us = /^\+1(\d{3})(\d{3})(\d{4})$/.exec(e164);
  return us ? `+1 (${us[1]}) ${us[2]}-${us[3]}` : e164;
}

export function isE164(value: string | undefined | null): value is string {
  return !!value && /^\+[1-9]\d{6,14}$/.test(value);
}
