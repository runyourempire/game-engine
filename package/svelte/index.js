/**
 * Svelte wrappers for GAME Web Components.
 *
 * These are Svelte actions (use:directive) that lazy-load the Web Component
 * module and sync parameters to element properties.
 *
 * Usage:
 *   <script>
 *     import { breathingDot } from 'game-components/svelte';
 *   </script>
 *   <breathing-dot use:breathingDot={{ speed: 2, color: '#ff0' }} />
 */

/**
 * Creates a Svelte action that lazy-loads a GAME Web Component and
 * forwards parameters as element properties.
 */
function createGameAction(tagName, modulePath) {
  let loaded = false;

  return function gameAction(node, params) {
    if (!loaded) {
      loaded = true;
      import(modulePath).then(() => {
        syncProps(node, params);
      });
    } else {
      syncProps(node, params);
    }

    function syncProps(element, properties) {
      if (!element || !properties) return;
      for (const [key, value] of Object.entries(properties)) {
        element[key] = value;
      }
    }

    return {
      update(newParams) {
        syncProps(node, newParams);
      },
      destroy() {
        // Cleanup if needed
      }
    };
  };
}

export const arcDemo = createGameAction('arc-demo', '../dist/arc-demo.js');
export const audioLayers = createGameAction('audio-layers', '../dist/audio-layers.js');
export const audioReactive = createGameAction('audio-reactive', '../dist/audio-reactive.js');
export const audioSpectrum = createGameAction('audio-spectrum', '../dist/audio-spectrum.js');
export const audioHello = createGameAction('audio-hello', '../dist/audio-hello.js');
export const breathingDot = createGameAction('breathing-dot', '../dist/breathing-dot.js');
export const cinematicArc = createGameAction('cinematic-arc', '../dist/cinematic-arc.js');
export const dashboardGauge = createGameAction('dashboard-gauge', '../dist/dashboard-gauge.js');
export const gameGalaxy = createGameAction('game-galaxy', '../dist/game-galaxy.js');
export const gameHello = createGameAction('game-hello', '../dist/game-hello.js');
export const gameInteractive = createGameAction('game-interactive', '../dist/game-interactive.js');
export const gameKaleidoscope = createGameAction('game-kaleidoscope', '../dist/game-kaleidoscope.js');
export const gameResonance = createGameAction('game-resonance', '../dist/game-resonance.js');
export const gameShowcase = createGameAction('game-showcase', '../dist/game-showcase.js');
export const gameSpectrum = createGameAction('game-spectrum', '../dist/game-spectrum.js');
export const gameSpinner = createGameAction('game-spinner', '../dist/game-spinner.js');
export const gameStarfield = createGameAction('game-starfield', '../dist/game-starfield.js');
export const layeredScene = createGameAction('layered-scene', '../dist/layered-scene.js');
export const loadingRing = createGameAction('loading-ring', '../dist/loading-ring.js');
export const loadingStages = createGameAction('loading-stages', '../dist/loading-stages.js');
export const metricRing = createGameAction('metric-ring', '../dist/metric-ring.js');
export const mouseFollow = createGameAction('mouse-follow', '../dist/mouse-follow.js');
export const neonRing = createGameAction('neon-ring', '../dist/neon-ring.js');
export const statusPulse = createGameAction('status-pulse', '../dist/status-pulse.js');
