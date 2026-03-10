import { ref, onMounted, onUnmounted, watch, Ref } from 'vue';

interface GameElement extends HTMLElement {
  setParam(name: string, value: number): void;
  setAudioData(data: Record<string, number>): void;
  getFrame(): ImageData | null;
  getFrameDataURL(type?: string): string | null;
}

interface UseGameComponentOptions {
  src: string;
  params?: Ref<Record<string, number>> | Record<string, number>;
}

export function useGameComponent(options: UseGameComponentOptions) {
  const elementRef = ref<GameElement | null>(null);
  const isReady = ref(false);

  onMounted(() => {
    const existing = document.querySelector(`script[data-game-src="${options.src}"]`);
    if (!existing) {
      const script = document.createElement('script');
      script.src = options.src;
      script.dataset.gameSrc = options.src;
      document.head.appendChild(script);
      script.onload = () => { isReady.value = true; };
    } else {
      isReady.value = true;
    }
  });

  // Watch params
  if (options.params) {
    watch(
      () => typeof options.params === 'object' && 'value' in options.params
        ? (options.params as Ref).value
        : options.params,
      (params) => {
        const el = elementRef.value;
        if (!el || !params) return;
        for (const [name, value] of Object.entries(params as Record<string, number>)) {
          el.setParam(name, value);
        }
      },
      { deep: true }
    );
  }

  function setParam(name: string, value: number) {
    elementRef.value?.setParam(name, value);
  }

  function getFrame(): ImageData | null {
    return elementRef.value?.getFrame() ?? null;
  }

  function getFrameDataURL(type?: string): string | null {
    return elementRef.value?.getFrameDataURL(type) ?? null;
  }

  return { elementRef, isReady, setParam, getFrame, getFrameDataURL };
}
