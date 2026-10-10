import { getContext } from 'svelte';
import type { Client, Overview } from '../api';
import { AUTOSAVE, type SaveStore } from '../storage/saves';
import { Router } from './router.svelte';

/**
 * App-wide state: the connection to the core, the current overview, and
 * what the UI is busy with. Screens fetch their own views; they reload
 * when `revision` changes, which happens after anything that changes the
 * world.
 */
export class GameState {
  phase = $state<'loading' | 'title' | 'playing'>('loading');
  overview = $state<Overview | null>(null);
  /** What the core is doing, for a progress indicator. */
  busy = $state<string | null>(null);
  error = $state<string | null>(null);
  revision = $state(0);
  readonly router = new Router();

  constructor(
    readonly client: Client,
    readonly saves: SaveStore,
  ) {}

  async boot(): Promise<void> {
    await this.run('Loading', async () => {
      const { inGame } = await this.client.hello();
      if (inGame) await this.enter(await this.client.overview());
      else this.phase = 'title';
    });
  }

  /** Runs an operation, showing it as busy and reporting failures. */
  async run<T>(label: string, op: () => Promise<T>): Promise<T | undefined> {
    this.busy = label;
    this.error = null;
    try {
      return await op();
    } catch (err) {
      this.fail(err);
      return undefined;
    } finally {
      this.busy = null;
    }
  }

  fail = (err: unknown): void => {
    this.error = err instanceof Error ? err.message : String(err);
  };

  /** Starts showing a game. */
  async enter(overview: Overview): Promise<void> {
    this.overview = overview;
    this.phase = 'playing';
    this.changed();
  }

  /** Something in the world changed: reload views and the overview. */
  changed(overview?: Overview): void {
    if (overview) this.overview = overview;
    this.revision++;
  }

  async refresh(): Promise<void> {
    this.changed(await this.client.overview());
  }

  async continueOn(): Promise<void> {
    if (this.overview?.matchDay) {
      this.router.go({ name: 'match' });
      return;
    }
    await this.run('Simulating', async () => {
      const c = await this.client.continueOn();
      await this.refresh();
      if (c.stopped === 'matchDay') this.router.go({ name: 'match' });
      else if (c.stopped === 'messages') this.router.go({ name: 'inbox' });
    });
  }

  async autosave(): Promise<void> {
    try {
      await this.saves.put(AUTOSAVE, await this.client.save());
    } catch (err) {
      console.warn('Autosave failed', err);
    }
  }

  async quitToTitle(): Promise<void> {
    this.overview = null;
    this.phase = 'title';
    this.router.go({ name: 'home' });
  }
}

const KEY = Symbol('game');

/** The context to mount the app with, so every component can `useGame`. */
export function gameContext(game: GameState): Map<symbol, unknown> {
  return new Map([[KEY, game]]);
}

export function useGame(): GameState {
  return getContext<GameState>(KEY);
}
