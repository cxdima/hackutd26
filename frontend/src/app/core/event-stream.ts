import { createParser } from 'eventsource-parser';
import { eventSchema, type CallEvent } from './schemas';

/**
 * Reads the authenticated SSE stream with fetch so the bearer token stays in a
 * header and never enters a URL (EventSource cannot set headers).
 */
export async function consumeEvents(
  response: Response,
  onEvent: (event: CallEvent) => void,
): Promise<void> {
  if (!response.body) throw new Error('Event stream unavailable');
  const parser = createParser({
    maxBufferSize: 256 * 1024,
    onEvent(frame) {
      let value: unknown;
      try {
        value = JSON.parse(frame.data);
      } catch {
        throw new Error('Invalid event data');
      }
      const parsed = eventSchema.safeParse(value);
      if (!parsed.success) throw new Error('Invalid event contract');
      onEvent(parsed.data);
    },
    onError(error) {
      if (error.type === 'max-buffer-size-exceeded') throw new Error('Event stream too large');
    },
  });
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  try {
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      parser.feed(decoder.decode(value, { stream: true }));
    }
    parser.feed(decoder.decode());
  } finally {
    await reader.cancel().catch(() => undefined);
    reader.releaseLock();
  }
}
