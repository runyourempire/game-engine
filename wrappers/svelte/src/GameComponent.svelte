<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';

  export let tag: string;
  export let src: string;
  export let params: Record<string, number> = {};

  const dispatch = createEventDispatcher<{ ready: HTMLElement }>();

  let containerEl: HTMLDivElement;
  let gameEl: HTMLElement | null = null;

  onMount(() => {
    // Load script
    const existing = document.querySelector(`script[data-game-src="${src}"]`);
    if (!existing) {
      const script = document.createElement('script');
      script.src = src;
      script.dataset.gameSrc = src;
      document.head.appendChild(script);
    }

    // Create element
    gameEl = document.createElement(tag);
    gameEl.style.width = '100%';
    gameEl.style.height = '100%';
    containerEl.appendChild(gameEl);

    customElements.whenDefined(tag).then(() => {
      if (gameEl) dispatch('ready', gameEl);
    });
  });

  onDestroy(() => {
    if (gameEl && containerEl.contains(gameEl)) {
      containerEl.removeChild(gameEl);
    }
    gameEl = null;
  });

  // Reactive params
  $: if (gameEl && params) {
    for (const [name, value] of Object.entries(params)) {
      (gameEl as any).setParam?.(name, value);
    }
  }

  export function setParam(name: string, value: number) {
    (gameEl as any)?.setParam(name, value);
  }

  export function getFrame(): ImageData | null {
    return (gameEl as any)?.getFrame?.() ?? null;
  }

  export function getFrameDataURL(type?: string): string | null {
    return (gameEl as any)?.getFrameDataURL?.(type) ?? null;
  }
</script>

<div bind:this={containerEl} style="width: 100%; height: 100%;">
</div>
