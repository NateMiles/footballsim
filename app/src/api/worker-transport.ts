import type { Transport } from './transport';
import type { WorkerIn, WorkerOut } from './worker';

/** Runs the WebAssembly core in a dedicated web worker. */
export class WorkerTransport implements Transport {
  private worker: Worker;
  private next = 1;
  private pending = new Map<number, { resolve: (r: string) => void; reject: (e: Error) => void }>();

  constructor() {
    this.worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
    this.worker.onmessage = (event: MessageEvent<WorkerOut>) => {
      const msg = event.data;
      const waiter = this.pending.get(msg.id);
      if (!waiter) return;
      this.pending.delete(msg.id);
      if ('fatal' in msg) waiter.reject(new Error(`The simulation stopped: ${msg.fatal}`));
      else waiter.resolve(msg.response);
    };
    this.worker.onerror = (event) => {
      const error = new Error(`The simulation failed to load: ${event.message}`);
      for (const w of this.pending.values()) w.reject(error);
      this.pending.clear();
    };
  }

  call(request: string): Promise<string> {
    const id = this.next++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.worker.postMessage({ id, request } satisfies WorkerIn);
    });
  }

  close(): void {
    this.worker.terminate();
  }
}
