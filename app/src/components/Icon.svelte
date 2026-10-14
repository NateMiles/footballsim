<!-- Line icons on a 24px grid, drawn with the current text colour. -->
<script lang="ts" module>
  export type IconName =
    | 'home'
    | 'inbox'
    | 'squad'
    | 'tactics'
    | 'training'
    | 'dressing'
    | 'transfers'
    | 'staff'
    | 'finances'
    | 'fixtures'
    | 'table'
    | 'stats'
    | 'cups'
    | 'competitions'
    | 'saves'
    | 'jobs'
    | 'more'
    | 'arrow'
    | 'calendar'
    | 'ball'
    | 'news'
    | 'mic';

  const PATHS: Record<IconName, string> = {
    home: 'M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z',
    inbox: 'M3 6.5A1.5 1.5 0 0 1 4.5 5h15A1.5 1.5 0 0 1 21 6.5v11a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5z M3.5 6.5 12 13l8.5-6.5',
    squad:
      'M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z M2.5 20c.6-3.4 3.2-5.5 6.5-5.5s5.9 2.1 6.5 5.5 M16 4.3a3.5 3.5 0 0 1 0 6.4 M18 14.8c2 .7 3.3 2.5 3.6 5.2',
    tactics:
      'M4 4h16v16H4z M12 4v16 M12 9.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z M4 9h3v6H4 M20 9h-3v6h3',
    training: 'M5 21V4 M5 4h11l-2 4 2 4H5',
    dressing:
      'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z M8.5 14.5c.9 1.3 2.1 2 3.5 2s2.6-.7 3.5-2 M9 9.5h.01 M15 9.5h.01',
    transfers: 'M4 8h13 M13 4l4 4-4 4 M20 16H7 M11 12l-4 4 4 4',
    staff:
      'M9 6V4.5A1.5 1.5 0 0 1 10.5 3h3A1.5 1.5 0 0 1 15 4.5V6 M3.5 7h17a.5.5 0 0 1 .5.5V19a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7.5a.5.5 0 0 1 .5-.5z M3 12.5h18',
    finances:
      'M3 7.5A1.5 1.5 0 0 1 4.5 6h15A1.5 1.5 0 0 1 21 7.5v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 16.5z M12 14.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z M6.5 12h.01 M17.5 12h.01',
    fixtures:
      'M4 6.5A1.5 1.5 0 0 1 5.5 5h13A1.5 1.5 0 0 1 20 6.5v12a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 18.5z M4 10h16 M8.5 3v4 M15.5 3v4 M8 14h2 M14 14h2',
    table: 'M9 6h11 M9 12h11 M9 18h11 M4 6h1 M4 12h1 M4 18h1',
    stats: 'M4 20V10 M10 20V4 M16 20v-7 M21 20H3',
    cups:
      'M8 4h8v5a4 4 0 0 1-8 0z M8 6H5a1 1 0 0 0-1 1c0 2 1.5 3.5 4 3.8 M16 6h3a1 1 0 0 1 1 1c0 2-1.5 3.5-4 3.8 M12 13v4 M8.5 20h7 M10 17h4',
    competitions:
      'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z M3 12h18 M12 3c2.5 2.5 3.8 5.5 3.8 9s-1.3 6.5-3.8 9c-2.5-2.5-3.8-5.5-3.8-9S9.5 5.5 12 3z',
    saves: 'M5 4h11l4 4v11a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z M8 4v5h7V4 M8 20v-6h8v6',
    jobs: 'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14z M21 21l-5-5',
    more: 'M5 12h.01 M12 12h.01 M19 12h.01',
    arrow: 'M5 12h14 M13 6l6 6-6 6',
    calendar: 'M4 6.5A1.5 1.5 0 0 1 5.5 5h13A1.5 1.5 0 0 1 20 6.5v12a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 18.5z M4 10h16 M8.5 3v4 M15.5 3v4',
    news: 'M4 5.5A1.5 1.5 0 0 1 5.5 4h11A1.5 1.5 0 0 1 18 5.5V19a1 1 0 0 0 2 0V9h-2 M18 19.5H6A2 2 0 0 1 4 17.5v-12 M7.5 8h7 M7.5 11.5h7 M7.5 15h4',
    mic: 'M12 3a3 3 0 0 0-3 3v5a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3z M5.5 11a6.5 6.5 0 0 0 13 0 M12 17.5V21 M8.5 21h7',
    ball: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z M12 7.5l3.8 2.8-1.4 4.5H9.6l-1.4-4.5z M12 3v4.5 M15.8 10.3l4.6-1.5 M14.4 14.8l2.9 3.9 M9.6 14.8l-2.9 3.9 M8.2 10.3 3.6 8.8',
  };
</script>

<script lang="ts">
  let { name, size = 20, stroke = 1.8 }: { name: IconName; size?: number; stroke?: number } = $props();
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={name === 'more' ? 3 : stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={PATHS[name]} />
</svg>

<style>
  .icon {
    flex: none;
    display: block;
  }
</style>
