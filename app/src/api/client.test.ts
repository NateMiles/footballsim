import { describe, expect, it } from 'vitest';
import { ApiError, Client } from './client';
import type { Transport } from './transport';

/** A transport that answers from a function, after a delay. */
function fake(answer: (req: { type: string }) => unknown, delay = (_: { type: string }) => 0): Transport & { seen: string[] } {
  const seen: string[] = [];
  return {
    seen,
    async call(json: string) {
      const req = JSON.parse(json);
      seen.push(req.type);
      await new Promise((r) => setTimeout(r, delay(req)));
      return JSON.stringify(answer(req));
    },
    close() {},
  };
}

describe('Client', () => {
  it('rejects with ApiError on an error response', async () => {
    const c = new Client(fake(() => ({ type: 'error', error: { code: 'noGame', message: 'no game loaded' } })));
    const err = await c.overview().catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err.code).toBe('noGame');
  });

  it('rejects an unexpected response type', async () => {
    const c = new Client(fake(() => ({ type: 'inbox', inbox: { messages: [], unread: 0 } })));
    await expect(c.overview()).rejects.toThrow(/Expected a overview response/);
  });

  it('sends requests one at a time, in order', async () => {
    const order: string[] = [];
    const t = fake(
      (req) => {
        order.push(req.type);
        return req.type === 'continue'
          ? { type: 'continued', continued: { date: '2026-07-02', stopped: 'week', newMessages: 0, matchDay: null } }
          : { type: 'inbox', inbox: { messages: [], unread: 0 } };
      },
      (req) => (req.type === 'continue' ? 30 : 0),
    );
    const c = new Client(t);
    await Promise.all([c.continueOn(), c.inbox()]);
    expect(order).toEqual(['continue', 'inbox']);
  });

  it('checks the protocol version', async () => {
    const c = new Client(fake(() => ({ type: 'hello', protocol: 999, version: 'x', inGame: false })));
    await expect(c.hello()).rejects.toThrow(/protocol 999/);
  });
});
