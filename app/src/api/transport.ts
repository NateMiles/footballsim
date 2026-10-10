/**
 * Carries JSON requests to a simulation core and its JSON responses back.
 *
 * The UI never knows where the core runs: in a web worker (WebAssembly),
 * natively behind the Tauri shell, or (later) on a league server. Each of
 * those is one implementation of this interface.
 */
export interface Transport {
  /** Sends one request (JSON) and resolves with the response (JSON). */
  call(request: string): Promise<string>;
  /** Releases the core. */
  close(): void;
}
