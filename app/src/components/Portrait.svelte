<!-- A person drawn from their look: head and shoulders in a shirt, with an
     expression. Everything is vector, so it is sharp at any size. -->
<script lang="ts" module>
  export type Expression =
    | 'neutral'
    | 'happy'
    | 'delighted'
    | 'determined'
    | 'annoyed'
    | 'angry'
    | 'sad'
    | 'sceptical'
    | 'talking';

  const SKIN = ['#f6d7c3', '#efc3a4', '#e2ad88', '#cc9068', '#ab714b', '#8b5637', '#6c4028', '#4b2c1c'];
  const SHADE = ['#e3b8a0', '#d9a483', '#c98f6b', '#b07550', '#8f5a38', '#714428', '#55311e', '#3a2115'];
  const HAIR: Record<string, string> = {
    black: '#1d1a19',
    darkBrown: '#3a271c',
    brown: '#5d3d25',
    lightBrown: '#8c653f',
    blond: '#d8b46a',
    ginger: '#b65a2a',
    grey: '#9c9c9c',
    white: '#e8e6e2',
  };
  const EYES: Record<string, string> = {
    darkBrown: '#3a2416',
    brown: '#6b4423',
    green: '#4d7a48',
    blue: '#4a79b2',
  };
</script>

<script lang="ts">
  import type { Look } from '../api';

  let {
    look,
    size = 48,
    expression = 'neutral',
    shirt = '#2a3a52',
    trim = '#dfe6ee',
    ring = false,
    title = '',
  }: {
    look: Look;
    size?: number;
    expression?: Expression;
    /** Shirt and collar colours, usually the club's. */
    shirt?: string;
    trim?: string;
    /** A round frame behind the bust. */
    ring?: boolean;
    title?: string;
  } = $props();
  const uid = $props.id();

  const skin = $derived(SKIN[look.skin] ?? SKIN[3]);
  const shade = $derived(SHADE[look.skin] ?? SHADE[3]);
  const hair = $derived(HAIR[look.hairColor] ?? HAIR.black);
  const iris = $derived(EYES[look.eyes] ?? EYES.brown);

  // Head: half widths at the cheekbones and the jaw.
  const W = $derived([17, 19, 21, 20][look.face] ?? 19);
  const J = $derived([10, 12, 14.5, 16][look.face] ?? 12);
  const head = $derived(
    `M50 17 C${50 + W + 3} 17 ${50 + W} 30 ${50 + W} 42 C${50 + W} 54 ${50 + J + 4} 62 ${50 + J} 64 ` +
      `Q50 70 ${50 - J} 64 C${50 - J - 4} 62 ${50 - W} 54 ${50 - W} 42 C${50 - W} 30 ${50 - W - 3} 17 50 17Z`,
  );

  // A cap of hair: crown at `top`, hairline at `line` in the middle and
  // `temple` at the sides.
  function cap(top: number, line: number, temple: number, widen = 0): string {
    const w = W + widen;
    return (
      `M${50 - w} ${temple} C${50 - w} ${top + 8} ${50 - w + 6} ${top} 50 ${top} ` +
      `C${50 + w - 6} ${top} ${50 + w} ${top + 8} ${50 + w} ${temple} ` +
      `C${50 + w - 3} ${line + 4} ${50 + w - 7} ${line} 50 ${line} ` +
      `C${50 - w + 7} ${line} ${50 - w + 3} ${line + 4} ${50 - w} ${temple}Z`
    );
  }

  const front = $derived.by(() => {
    switch (look.hair) {
      case 'buzz':
        return { d: cap(16, 25, 38), opacity: 0.8 };
      case 'short':
        return { d: cap(13, 26, 40, 1.5), opacity: 1 };
      case 'sidePart':
        return {
          d:
            cap(13, 25, 40, 1.5) +
            ` M${50 - 6} 25 C${50 - 2} 21 ${50 + 10} 21 ${50 + W + 1} 32 C${50 + 8} 24 50 26 ${50 - 6} 25Z`,
          opacity: 1,
        };
      case 'curly': {
        // Curls along the crown, all wound the same way as the cap.
        let d = cap(15, 26, 40, 2);
        for (let i = 0; i < 7; i++) {
          const a = Math.PI * (0.08 + (0.84 * i) / 6);
          const x = 50 - Math.cos(a) * (W + 1);
          const y = 30 - Math.sin(a) * 15;
          d += ` M${x - 4.5} ${y} a4.5 4.5 0 1 1 9 0 a4.5 4.5 0 1 1 -9 0`;
        }
        return { d, opacity: 1 };
      }
      case 'afro':
        return { d: cap(10, 26, 42, 4), opacity: 1 };
      case 'long':
        return {
          d: cap(13, 24, 44, 2) + ` M50 13 L50 24`,
          opacity: 1,
        };
      case 'bun':
        return { d: cap(15, 25, 38, 1) + ` M56 11 a6 6 0 1 0 -12 0 a6 6 0 1 0 12 0`, opacity: 1 };
      case 'receding':
        return {
          d:
            `M${50 - W - 0.5} 42 C${50 - W - 0.5} 34 ${50 - W + 1} 28 ${50 - W + 4} 25 C${50 - W + 4} 30 ${50 - W + 3} 35 ${50 - W + 2} 42Z ` +
            `M${50 + W + 0.5} 42 C${50 + W + 0.5} 34 ${50 + W - 1} 28 ${50 + W - 4} 25 C${50 + W - 4} 30 ${50 + W - 3} 35 ${50 + W - 2} 42Z ` +
            `M${50 - 7} 18.5 C${50 - 3} 16.5 ${50 + 3} 16.5 ${50 + 7} 18.5 C${50 + 3} 19.6 ${50 - 3} 19.6 ${50 - 7} 18.5Z`,
          opacity: 0.9,
        };
      case 'mohawk':
        return {
          d: cap(17, 26, 38) + ` M44 26 C44 14 47 7 50 6 C53 7 56 14 56 26 C53 24 47 24 44 26Z`,
          opacity: 1,
        };
      default:
        return null;
    }
  });

  /** Hair behind the head. */
  const back = $derived.by(() => {
    switch (look.hair) {
      case 'afro':
        return `M50 6 a${W + 10} ${W + 8} 0 1 1 -0.1 0Z`;
      case 'long':
        return `M${50 - W - 3} 30 C${50 - W - 5} 50 ${50 - W - 6} 66 ${50 - W + 2} 74 L${50 + W - 2} 74 C${50 + W + 6} 66 ${50 + W + 5} 50 ${50 + W + 3} 30 C${50 + W} 14 ${50 - W} 14 ${50 - W - 3} 30Z`;
      case 'curly':
        return `M${50 - W - 2} 36 C${50 - W - 3} 18 ${50 + W + 3} 18 ${50 + W + 2} 36Z`;
      default:
        return null;
    }
  });

  // Eyes, brows and mouth by expression.
  const smileEyes = $derived(expression === 'delighted');
  const eyeRy = $derived(expression === 'angry' || expression === 'determined' ? 1.5 : expression === 'annoyed' ? 1.8 : 2.3);
  function brow(side: -1 | 1): string {
    const x = 50 + side * 8;
    const inner = x - side * 4.5;
    const outer = x + side * 4.5;
    let yi = 37.5;
    let yo = 37.2;
    switch (expression) {
      case 'angry':
        yi = 40.4;
        yo = 35.6;
        break;
      case 'annoyed':
      case 'determined':
        yi = 38.8;
        yo = 37;
        break;
      case 'sad':
        yi = 34.8;
        yo = 38.6;
        break;
      case 'sceptical':
        if (side === -1) {
          yi -= 2.2;
          yo -= 2.6;
        } else {
          yi += 0.8;
        }
        break;
      case 'delighted':
      case 'happy':
        yi -= 0.8;
        yo -= 0.8;
        break;
    }
    return `M${inner} ${yi} Q${x} ${Math.min(yi, yo) - 1.6} ${outer} ${yo}`;
  }
  const mw = $derived([4.5, 5.5, 6.5][look.mouth] ?? 5.5);
  const mouth = $derived.by(() => {
    const l = 50 - mw;
    const r = 50 + mw;
    switch (expression) {
      case 'happy':
        return { d: `M${l - 0.5} 57.2 Q50 62.6 ${r + 0.5} 57.2`, open: false };
      case 'delighted':
        return { d: `M${l - 0.5} 57 Q50 65 ${r + 0.5} 57 Q50 58.4 ${l - 0.5} 57Z`, open: true };
      case 'sad':
        return { d: `M${l} 60.2 Q50 55.6 ${r} 60.2`, open: false };
      case 'annoyed':
        return { d: `M${l} 59 Q50 57.6 ${r} 59`, open: false };
      case 'angry':
        return { d: `M${l + 0.5} 59.6 Q50 56.8 ${r - 0.5} 59.6 Q50 58.2 ${l + 0.5} 59.6Z`, open: true };
      case 'sceptical':
        return { d: `M${l} 58.8 Q${50 + 1} 58.6 ${r} 57.2`, open: false };
      case 'talking':
        return { d: `M${l + 1} 57.8 Q50 61.2 ${r - 1} 57.8 Q50 57 ${l + 1} 57.8Z`, open: true };
      case 'determined':
        return { d: `M${l} 58.4 L${r} 58.4`, open: false };
      default:
        return { d: `M${l} 58.2 Q50 59.4 ${r} 58.2`, open: false };
    }
  });
  const nose = $derived(
    [
      'M50.5 44.5 L48.6 51.4 Q50 52.6 51.8 51.6',
      'M49.6 44.5 Q51.8 49 52 51.4 Q50.2 52.8 48 51.6',
      'M50 45 L48 51.2 Q50 53.4 52.4 51.2',
    ][look.nose] ?? '',
  );
  const browWidth = $derived([1.3, 1.9, 2.6][look.brows] ?? 1.9);

  const beard = $derived.by(() => {
    const jaw =
      `M${50 - W} 45 C${50 - W} 57 ${50 - J - 3} 66 50 71 C${50 + J + 3} 66 ${50 + W} 57 ${50 + W} 45 ` +
      `L${50 + W - 2.5} 45 C${50 + W - 3} 53 ${50 + 8} 55.5 50 55.5 C${50 - 8} 55.5 ${50 - W + 3} 53 ${50 - W + 2.5} 45Z`;
    const tache = `M${50 - mw - 1} 57 Q50 53 ${50 + mw + 1} 57 Q50 55.6 ${50 - mw - 1} 57Z`;
    switch (look.beard) {
      case 'stubble':
        return { d: jaw + ' ' + tache, opacity: 0.22 };
      case 'short':
        return { d: jaw + ' ' + tache, opacity: 0.75 };
      case 'full':
        return {
          d:
            `M${50 - W} 40 C${50 - W} 57 ${50 - J - 3} 66 50 71.5 C${50 + J + 3} 66 ${50 + W} 57 ${50 + W} 40 ` +
            `L${50 + W - 3} 41 C${50 + W - 4} 50 ${50 + 7} 54 50 54 C${50 - 7} 54 ${50 - W + 4} 50 ${50 - W + 3} 41Z ` +
            tache,
          opacity: 0.95,
        };
      case 'moustache':
        return { d: tache, opacity: 0.95 };
      case 'goatee':
        return { d: tache + ` M46 61.5 Q50 60.4 54 61.5 L53 66.5 Q50 68.6 47 66.5Z`, opacity: 0.9 };
      default:
        return null;
    }
  });
</script>

<svg
  class="portrait"
  width={size}
  height={size}
  viewBox="9 5 82 82"
  role="img"
  aria-label={title || undefined}
  aria-hidden={title ? undefined : 'true'}
>
  {#if title}<title>{title}</title>{/if}
  <defs>
    <clipPath id="{uid}-frame"><circle cx="50" cy="46" r="41" /></clipPath>
    <clipPath id="{uid}-head"><path d={head} /></clipPath>
    <radialGradient id="{uid}-light" cx="0.38" cy="0.3" r="0.8">
      <stop offset="0" stop-color="#fff" stop-opacity="0.18" />
      <stop offset="0.6" stop-color="#fff" stop-opacity="0" />
      <stop offset="1" stop-color="#000" stop-opacity="0.18" />
    </radialGradient>
  </defs>
  <g clip-path={ring ? `url(#${uid}-frame)` : undefined}>
    {#if ring}
      <circle cx="50" cy="46" r="41" fill="var(--bg-elevated, #172131)" />
      <circle cx="50" cy="46" r="41" fill={shirt} opacity="0.18" />
    {/if}
    {#if back}<path d={back} fill={hair} />{/if}

    <!-- Shoulders and neck. -->
    <path d="M12 104 C14 86 28 79 41 76 L50 84 L59 76 C72 79 86 86 88 104Z" fill={shirt} />
    <path d="M41 76 L50 86.5 L59 76 L56.5 75 L50 82 L43.5 75Z" fill={trim} />
    <path d="M41.5 58 L41.5 75 Q50 83 58.5 75 L58.5 58Z" fill={shade} />

    <!-- Ears, head and its light. -->
    <ellipse cx={50 - W} cy="45" rx="3.6" ry="6" fill={shade} />
    <ellipse cx={50 + W} cy="45" rx="3.6" ry="6" fill={shade} />
    <path d={head} fill={skin} />
    <path d={head} fill="url(#{uid}-light)" />

    {#if beard}
      <g clip-path="url(#{uid}-head)">
        <path d={beard.d} fill={hair} opacity={beard.opacity} fill-rule="evenodd" />
      </g>
    {/if}

    <!-- Eyes. -->
    {#each [-1, 1] as side (side)}
      {@const x = 50 + side * 8}
      {#if smileEyes}
        <path d="M{x - 3.2} 44 Q{x} 41 {x + 3.2} 44" fill="none" stroke="#2a1d16" stroke-width="1.3" stroke-linecap="round" />
      {:else}
        <ellipse cx={x} cy="43.4" rx="3.3" ry={eyeRy} fill="#f7f4f0" />
        <circle cx={x + 0.3} cy="43.4" r={Math.min(1.75, eyeRy)} fill={iris} />
        <circle cx={x + 0.3} cy="43.4" r="0.8" fill="#120c09" />
        <circle cx={x + 0.9} cy="42.8" r="0.45" fill="#fff" />
      {/if}
      <path
        d={brow(side as -1 | 1)}
        fill="none"
        stroke={look.hair === 'bald' && look.hairColor !== 'black' ? shade : hair}
        stroke-width={browWidth}
        stroke-linecap="round"
      />
    {/each}

    <path d={nose} fill="none" stroke={shade} stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" />

    {#if mouth.open}
      <path d={mouth.d} fill="#5a2620" />
      {#if expression === 'delighted'}
        <path d="M{50 - mw + 0.5} 57.6 Q50 59.6 {50 + mw - 0.5} 57.6Z" fill="#fff" opacity="0.9" />
      {/if}
    {:else}
      <path d={mouth.d} fill="none" stroke="#7a3b30" stroke-width="1.4" stroke-linecap="round" />
    {/if}

    {#if front}
      {#if look.hair === 'mohawk'}<path d={cap(16, 25, 38)} fill={hair} opacity="0.35" />{/if}
      <path d={front.d} fill={hair} opacity={front.opacity} />
    {/if}
  </g>
</svg>

<style>
  .portrait {
    display: inline-block;
    flex: none;
    vertical-align: middle;
  }
</style>
