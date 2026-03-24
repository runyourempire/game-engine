<script lang="ts">
  interface Props {
    tag: string;
    src: string;
    params?: Record<string, number>;
    className?: string;
    style?: string;
    onReady?: (el: HTMLElement) => void;
  }

  let { tag, src, params = {}, className, style, onReady }: Props = $props();

  let containerEl: HTMLDivElement;
  let gameEl: HTMLElement | null = $state(null);

  $effect(() => {
    // Load script
    const existing = document.querySelector(`script[data-game-src="${src}"]`);
    if (!existing) {
      const script = document.createElement('script');
      script.src = src;
      script.dataset.gameSrc = src;
      document.head.appendChild(script);
    }

    // Create element
    const el = document.createElement(tag);
    el.style.width = '100%';
    el.style.height = '100%';
    containerEl.appendChild(el);
    gameEl = el;

    customElements.whenDefined(tag).then(() => {
      if (gameEl && onReady) onReady(gameEl);
    });

    return () => {
      if (el && containerEl?.contains(el)) {
        containerEl.removeChild(el);
      }
      gameEl = null;
    };
  });

  // Reactive params
  $effect(() => {
    if (gameEl && params) {
      for (const [name, value] of Object.entries(params)) {
        (gameEl as any).setParam?.(name, value);
      }
    }
  });

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

<div bind:this={containerEl} class={className} {style} style:width="100%" style:height="100%">
</div>
