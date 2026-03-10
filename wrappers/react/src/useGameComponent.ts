import { useRef, useEffect, useCallback } from 'react';

interface GameElement extends HTMLElement {
  setParam(name: string, value: number): void;
  setAudioData(data: Record<string, number>): void;
  setAudioSource(bridge: { subscribe(cb: (data: Record<string, number>) => void): void }): void;
  getFrame(): ImageData | null;
  getFrameDataURL(type?: string): string | null;
}

interface UseGameComponentOptions {
  /** Path to the compiled .js component file */
  src: string;
  /** Initial uniform parameters */
  params?: Record<string, number>;
  /** Callback when component is ready */
  onReady?: (element: GameElement) => void;
}

export function useGameComponent(options: UseGameComponentOptions) {
  const ref = useRef<GameElement>(null);
  const scriptLoaded = useRef(false);

  // Load the component script on mount
  useEffect(() => {
    if (scriptLoaded.current) return;

    // Check if already loaded
    const existing = document.querySelector(`script[data-game-src="${options.src}"]`);
    if (!existing) {
      const script = document.createElement('script');
      script.src = options.src;
      script.dataset.gameSrc = options.src;
      document.head.appendChild(script);
      script.onload = () => {
        scriptLoaded.current = true;
      };
    } else {
      scriptLoaded.current = true;
    }
  }, [options.src]);

  // Apply params when they change
  useEffect(() => {
    const el = ref.current;
    if (!el || !options.params) return;
    for (const [name, value] of Object.entries(options.params)) {
      el.setParam(name, value);
    }
  }, [options.params]);

  // Fire onReady callback
  useEffect(() => {
    const el = ref.current;
    if (el && options.onReady) {
      // Wait for custom element to be defined
      customElements.whenDefined(el.tagName.toLowerCase()).then(() => {
        options.onReady?.(el);
      });
    }
  }, []);

  const setParam = useCallback((name: string, value: number) => {
    ref.current?.setParam(name, value);
  }, []);

  const getFrame = useCallback(() => {
    return ref.current?.getFrame() ?? null;
  }, []);

  const getFrameDataURL = useCallback((type?: string) => {
    return ref.current?.getFrameDataURL(type) ?? null;
  }, []);

  return { ref, setParam, getFrame, getFrameDataURL };
}
