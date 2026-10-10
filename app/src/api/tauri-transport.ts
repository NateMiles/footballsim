import type { Transport } from './transport';

/**
 * Talks to the native core inside the Tauri shell (desktop and iPad).
 * The shell exposes one command, `call`, backed by the same fs-api session.
 */
export class TauriTransport implements Transport {
  async call(request: string): Promise<string> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('call', { request });
  }

  close(): void {}
}

/** Whether the page is running inside the Tauri shell. */
export function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
