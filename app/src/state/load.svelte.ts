import { useGame } from './game.svelte';

/**
 * Loads a screen's view, and reloads it whenever the world changes or
 * anything `fetch` reads (such as a route parameter) changes. Call during
 * component setup.
 */
export function load<T>(fetch: () => Promise<T>): { readonly data: T | null } {
  const game = useGame();
  const state = $state<{ data: T | null }>({ data: null });
  $effect(() => {
    void game.revision;
    let stale = false;
    fetch()
      .then((d) => {
        if (!stale) state.data = d;
      })
      .catch(game.fail);
    return () => {
      stale = true;
    };
  });
  return state;
}
