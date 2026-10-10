import { Client } from './client';
import { TauriTransport, inTauri } from './tauri-transport';
import type { Transport } from './transport';
import { WorkerTransport } from './worker-transport';

export * from './client';
export type * from './generated';
export type { Transport } from './transport';

/** The best transport for where the app is running. */
export function defaultTransport(): Transport {
  return inTauri() ? new TauriTransport() : new WorkerTransport();
}

/** A client connected to the best available core. */
export function connect(transport: Transport = defaultTransport()): Client {
  return new Client(transport);
}
