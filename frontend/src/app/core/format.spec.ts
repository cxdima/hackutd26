import { formatDuration, formatTime, relativeAge, relativeUntil, zoneLabel } from './format';

describe('time formatting', () => {
  it('formats in the profile timezone and never throws on bad input', () => {
    const at = '2026-11-08T14:30:00Z';
    expect(formatTime(at, 'America/Chicago')).toBe('8:30 AM');
    expect(formatTime(at, 'America/New_York')).toBe('9:30 AM');
    expect(formatTime(at, 'Not/AZone')).toMatch(/\d/);
    expect(formatTime('nonsense', 'America/Chicago')).toBe('Time not received');
    expect(zoneLabel('America/Chicago', at)).toBe('CST');
  });

  it('describes durations and relative times in plain words', () => {
    expect(formatDuration(29)).toBe('29 s');
    expect(formatDuration(125)).toBe('2 min 05 s');
    expect(formatDuration(undefined)).toBe('—');
    const now = Date.parse('2026-11-08T14:30:00Z');
    expect(relativeAge('2026-11-08T12:30:00Z', now)).toBe('2 h ago');
    expect(relativeUntil('2026-11-09T12:30:00Z', now)).toBe('in 22 h');
    expect(relativeUntil('2026-11-08T12:30:00Z', now)).toBe('expired');
  });
});
