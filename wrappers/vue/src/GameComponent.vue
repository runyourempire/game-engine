<template>
  <div ref="containerRef" :class="className" :style="containerStyle">
    <component :is="tag" ref="gameRef" style="width: 100%; height: 100%" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed, CSSProperties } from 'vue';

interface Props {
  tag: string;
  src: string;
  params?: Record<string, number>;
  className?: string;
  style?: CSSProperties;
}

const props = withDefaults(defineProps<Props>(), {
  params: () => ({}),
});

const emit = defineEmits<{
  ready: [element: HTMLElement];
}>();

const containerRef = ref<HTMLDivElement>();
const gameRef = ref<HTMLElement>();

const containerStyle = computed<CSSProperties>(() => ({
  width: '100%',
  height: '100%',
  ...props.style,
}));

onMounted(() => {
  // Load component script
  const existing = document.querySelector(`script[data-game-src="${props.src}"]`);
  if (!existing) {
    const script = document.createElement('script');
    script.src = props.src;
    script.dataset.gameSrc = props.src;
    document.head.appendChild(script);
  }

  if (gameRef.value) {
    customElements.whenDefined(props.tag).then(() => {
      emit('ready', gameRef.value!);
    });
  }
});

// Watch params
watch(
  () => props.params,
  (params) => {
    const el = gameRef.value as any;
    if (!el?.setParam) return;
    for (const [name, value] of Object.entries(params || {})) {
      el.setParam(name, value);
    }
  },
  { deep: true }
);

// Expose methods
function setParam(name: string, value: number) {
  (gameRef.value as any)?.setParam(name, value);
}

function getFrame(): ImageData | null {
  return (gameRef.value as any)?.getFrame?.() ?? null;
}

function getFrameDataURL(type?: string): string | null {
  return (gameRef.value as any)?.getFrameDataURL?.(type) ?? null;
}

defineExpose({ setParam, getFrame, getFrameDataURL, element: gameRef });
</script>
