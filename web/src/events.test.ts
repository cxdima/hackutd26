// The transport boundary must retain split UTF-8 text and all SSE data lines.
import { expect, it } from "vitest";
import { consumeEvents } from "./events";
import type { CallEvent } from "./types";
it("assembles chunked SSE frames and rejects malformed event contracts", async () => {
  const raw =
    'id: 7\r\ndata: {"id":"evt7","sequence":7,"session_id":"s1","leg_role":"caller",\r\ndata: "occurred_at":"2026-11-07T23:45:00Z","received_at":"2026-11-07T23:45:00Z","policy_epoch":0,"execution_mode":"replay","data_origin":"synthetic","type":"transcript.final","payload":{"text":"Sarah’s visit"}}\r\n\r\n';
  const bytes = new TextEncoder().encode(raw);
  const stream = new ReadableStream({
    start(controller) {
      for (let i = 0; i < bytes.length; i += 7)
        controller.enqueue(bytes.slice(i, i + 7));
      controller.close();
    },
  });
  const events: CallEvent[] = [];
  await consumeEvents(new Response(stream), (event) => events.push(event));
  expect(events).toHaveLength(1);
  expect(events[0].payload.text).toBe("Sarah’s visit");
  expect(events[0].execution_mode).toBe("replay");
  await expect(
    consumeEvents(
      new Response('data: {"type":"contact.acknowledged"}\n\n'),
      () => {},
    ),
  ).rejects.toThrow("Invalid event");
});
