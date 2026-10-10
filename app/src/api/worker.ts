/// <reference lib="webworker" />
/**
 * The web worker that hosts the WebAssembly core. It keeps the simulation
 * off the main thread, so the UI stays smooth while days are simulated.
 *
 * Messages in:  { id, request }   (request is protocol JSON)
 * Messages out: { id, response }  (response is protocol JSON)
 *               { id, fatal }     (the core crashed; reload needed)
 */
import init, { Engine } from '../wasm/pkg/fs_wasm.js';

export type WorkerIn = { id: number; request: string };
export type WorkerOut = { id: number; response: string } | { id: number; fatal: string };

const scope = self as unknown as DedicatedWorkerGlobalScope;
let engine: Engine | null = null;
let crashed: string | null = null;
const ready = init().then(() => {
  engine = new Engine();
});

scope.onmessage = async (event: MessageEvent<WorkerIn>) => {
  const { id, request } = event.data;
  try {
    await ready;
    if (crashed !== null || engine === null) throw new Error(crashed ?? 'engine not loaded');
    scope.postMessage({ id, response: engine.call(request) } satisfies WorkerOut);
  } catch (err) {
    // A Rust panic leaves the engine unusable: report it and refuse more.
    crashed = err instanceof Error ? err.message : String(err);
    scope.postMessage({ id, fatal: crashed } satisfies WorkerOut);
  }
};
