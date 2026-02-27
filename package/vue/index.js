import { defineComponent, ref, onMounted, h, watch, toRefs } from 'vue';

/**
 * Creates a Vue 3 wrapper for a GAME Web Component.
 * Uses Composition API with lazy loading and property forwarding.
 */
function createGameComponent(tagName, displayName) {
  let loaded = false;

  return defineComponent({
    name: displayName,
    inheritAttrs: false,
    props: {
      // Generic pass-through — all GAME components accept arbitrary properties
      // Users pass them as props and they get forwarded to the custom element
    },
    setup(props, { attrs, slots }) {
      const el = ref(null);

      onMounted(async () => {
        if (!loaded) {
          loaded = true;
          await import(`../dist/${tagName}.js`);
        }
        syncProps(el.value, attrs);
      });

      // Watch for attribute changes and sync to element properties
      watch(
        () => ({ ...attrs }),
        (newAttrs) => {
          syncProps(el.value, newAttrs);
        },
        { deep: true }
      );

      function syncProps(element, properties) {
        if (!element) return;
        for (const [key, value] of Object.entries(properties)) {
          if (key === 'class' || key === 'style') continue;
          element[key] = value;
        }
      }

      return () => {
        return h(tagName, {
          ref: el,
          class: attrs.class,
          style: attrs.style,
        }, slots.default ? slots.default() : undefined);
      };
    }
  });
}

export const ArcDemo = createGameComponent('arc-demo', 'ArcDemo');
export const AudioLayers = createGameComponent('audio-layers', 'AudioLayers');
export const AudioReactive = createGameComponent('audio-reactive', 'AudioReactive');
export const AudioSpectrum = createGameComponent('audio-spectrum', 'AudioSpectrum');
export const AudioHello = createGameComponent('audio-hello', 'AudioHello');
export const BreathingDot = createGameComponent('breathing-dot', 'BreathingDot');
export const CinematicArc = createGameComponent('cinematic-arc', 'CinematicArc');
export const DashboardGauge = createGameComponent('dashboard-gauge', 'DashboardGauge');
export const GameGalaxy = createGameComponent('game-galaxy', 'GameGalaxy');
export const GameHello = createGameComponent('game-hello', 'GameHello');
export const GameInteractive = createGameComponent('game-interactive', 'GameInteractive');
export const GameKaleidoscope = createGameComponent('game-kaleidoscope', 'GameKaleidoscope');
export const GameResonance = createGameComponent('game-resonance', 'GameResonance');
export const GameShowcase = createGameComponent('game-showcase', 'GameShowcase');
export const GameSpectrum = createGameComponent('game-spectrum', 'GameSpectrum');
export const GameSpinner = createGameComponent('game-spinner', 'GameSpinner');
export const GameStarfield = createGameComponent('game-starfield', 'GameStarfield');
export const LayeredScene = createGameComponent('layered-scene', 'LayeredScene');
export const LoadingRing = createGameComponent('loading-ring', 'LoadingRing');
export const LoadingStages = createGameComponent('loading-stages', 'LoadingStages');
export const MetricRing = createGameComponent('metric-ring', 'MetricRing');
export const MouseFollow = createGameComponent('mouse-follow', 'MouseFollow');
export const NeonRing = createGameComponent('neon-ring', 'NeonRing');
export const StatusPulse = createGameComponent('status-pulse', 'StatusPulse');
