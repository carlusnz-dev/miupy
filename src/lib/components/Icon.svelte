<script lang="ts">
  // Ícones SVG inline (24x24, traço). Sem biblioteca externa: o app é offline.
  type Props = { name: string; size?: number; fill?: boolean; class?: string };
  let { name, size = 20, fill = false, class: cls = '' }: Props = $props();

  const P: Record<string, string> = {
    shuffle: 'M4 7h3c5 0 5 10 10 10h3m-3-3 3 3-3 3M4 17h3c2 0 3-1.5 4-3M14 9c1-1.5 2-2 3-2h3m-3-3 3 3-3 3',
    home: 'M3 11.5 12 4l9 7.5M5.5 10v9.5h4.5V14h4v5.5h4.5V10',
    library: 'M4 5v14M9 5v14M14 6l5 13M14 6l-1 .3',
    playlist: 'M4 6h12M4 11h12M4 16h7M18 14v6m0-6 3 1M15 20a3 3 0 1 0 3-3',
    search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14ZM20 20l-4.2-4.2',
    download: 'M12 4v11m0 0-4-4m4 4 4-4M5 19h14',
    play: 'M8 5.5v13l11-6.5z',
    pause: 'M8 5v14M16 5v14',
    next: 'M6 5.5v13l9-6.5zM18 5v14',
    prev: 'M18 5.5v13L9 12zM6 5v14',
    heart: 'M12 20s-7.5-4.6-7.5-10A4.3 4.3 0 0 1 12 7.4 4.3 4.3 0 0 1 19.5 10c0 5.4-7.5 10-7.5 10Z',
    volume: 'M4 9.5v5h3.5L12 18.5v-13L7.5 9.5H4ZM15.5 9a4 4 0 0 1 0 6M18 6.5a8 8 0 0 1 0 11',
    mute: 'M4 9.5v5h3.5L12 18.5v-13L7.5 9.5H4ZM16 9.5l5 5m0-5-5 5',
    queue: 'M4 6h12M4 11h12M4 16h6M16 14l5 3-5 3z',
    user: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8ZM4.5 20a7.5 7.5 0 0 1 15 0',
    settings: 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM19.4 13.5l1.6 1.2-1.7 3-1.9-.7a7 7 0 0 1-1.7 1l-.3 2h-3.4l-.3-2a7 7 0 0 1-1.7-1l-1.9.7-1.7-3 1.6-1.2a7 7 0 0 1 0-2l-1.6-1.2 1.7-3 1.9.7a7 7 0 0 1 1.7-1l.3-2h3.4l.3 2a7 7 0 0 1 1.7 1l1.9-.7 1.7 3-1.6 1.2a7 7 0 0 1 0 2Z',
    stats: 'M5 20V11M12 20V4M19 20v-6',
    sparkle: 'M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8zM19 16l.8 2.2L22 19l-2.2.8L19 22l-.8-2.2L16 19l2.2-.8z',
    plus: 'M12 5v14M5 12h14',
    trash: 'M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13M10 11v6M14 11v6',
    edit: 'M4 20h4L19 9l-4-4L4 16v4ZM13.5 6.5l4 4',
    close: 'M6 6l12 12M18 6 6 18',
    'chevron-left': 'M15 6l-6 6 6 6',
    'chevron-right': 'M9 6l6 6-6 6',
    check: 'M5 12.5 10 17.5 19 7',
    down: 'M6 9l6 6 6-6',
    left: 'M15 5l-7 7 7 7',
    up: 'M6 15l6-6 6 6',
    more: 'M5 12h.01M12 12h.01M19 12h.01',
    music: 'M9 18V6l10-2v12M9 18a3 3 0 1 1-3-3 3 3 0 0 1 3 3Zm10-2a3 3 0 1 1-3-3 3 3 0 0 1 3 3Z',
    mic: 'M12 14a3 3 0 0 0 3-3V6a3 3 0 0 0-6 0v5a3 3 0 0 0 3 3ZM6 11a6 6 0 0 0 12 0M12 17v3',
    flame: 'M12 21c-4 0-7-2.6-7-6.3 0-2.6 1.6-4.4 3-6 .8-.9 1.5-2.2 1.5-4.2 3 1.2 4.5 3.8 4.5 6 1-.6 1.5-1.700 1.500-3 2 1.700 3.500 4.300 3.500 7.200C19 18.400 16 21 12 21Z',
    alert: 'M12 4 2.800 20h18.400L12 4ZM12 10v4.500M12 17.500h.01',
    refresh: 'M20 11a8 8 0 0 0-14-4.500L4 9M4 4v5h5M4 13a8 8 0 0 0 14 4.500L20 15M20 20v-5h-5',
    grip: 'M9 6h.01M9 12h.01M9 18h.01M15 6h.01M15 12h.01M15 18h.01',
    expand: 'M8 3H4v4M16 3h4v4M8 21H4v-4M16 21h4v-4',
    external: 'M14 4h6v6M20 4l-9 9M18 14v5H5V6h5',
    globe: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18ZM3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18',
  };
  // ícones preenchidos quando "fill"
  const filled = new Set(['play', 'next', 'prev', 'heart']);
</script>

<svg
  class={cls} width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false"
  fill={fill || (filled.has(name) && name !== 'heart') ? 'currentColor' : 'none'}
  stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"
>
  <path d={P[name] ?? ''} fill={name === 'heart' && fill ? 'currentColor' : undefined} />
</svg>
