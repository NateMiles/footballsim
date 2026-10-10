<!-- Hosts a renderer and drives it from the playback clock. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { Canvas2DRenderer } from './canvas2d';
  import type { Playback } from './playback.svelte';
  import type { MatchRenderer, MatchScene } from './renderer';

  let {
    playback,
    scene,
    selected = null,
    renderer = () => new Canvas2DRenderer(),
  }: {
    playback: Playback;
    scene: MatchScene;
    selected?: number | null;
    renderer?: () => MatchRenderer;
  } = $props();

  let host: HTMLDivElement;

  onMount(() => {
    const r = renderer();
    r.mount(host);
    r.setScene(scene);
    let last = performance.now();
    let raf = requestAnimationFrame(function loop(now) {
      const dt = Math.min(0.25, (now - last) / 1000);
      last = now;
      const sample = playback.step(dt);
      if (sample) r.draw(sample, { selected });
      raf = requestAnimationFrame(loop);
    });
    return () => {
      cancelAnimationFrame(raf);
      r.destroy();
    };
  });
</script>

<div class="host" bind:this={host}></div>

<style>
  .host {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    overflow: hidden;
  }
</style>
