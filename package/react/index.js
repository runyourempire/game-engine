import { createElement, useRef, useEffect, forwardRef } from 'react';

// Lazy-import the Web Components so they self-register on first use
const componentModules = {
  'arc-demo': () => import('../dist/arc-demo.js'),
  'audio-layers': () => import('../dist/audio-layers.js'),
  'audio-reactive': () => import('../dist/audio-reactive.js'),
  'audio-spectrum': () => import('../dist/audio-spectrum.js'),
  'audio-hello': () => import('../dist/audio-hello.js'),
  'breathing-dot': () => import('../dist/breathing-dot.js'),
  'cinematic-arc': () => import('../dist/cinematic-arc.js'),
  'dashboard-gauge': () => import('../dist/dashboard-gauge.js'),
  'game-galaxy': () => import('../dist/game-galaxy.js'),
  'game-hello': () => import('../dist/game-hello.js'),
  'game-interactive': () => import('../dist/game-interactive.js'),
  'game-kaleidoscope': () => import('../dist/game-kaleidoscope.js'),
  'game-resonance': () => import('../dist/game-resonance.js'),
  'game-showcase': () => import('../dist/game-showcase.js'),
  'game-spectrum': () => import('../dist/game-spectrum.js'),
  'game-spinner': () => import('../dist/game-spinner.js'),
  'game-starfield': () => import('../dist/game-starfield.js'),
  'layered-scene': () => import('../dist/layered-scene.js'),
  'loading-ring': () => import('../dist/loading-ring.js'),
  'loading-stages': () => import('../dist/loading-stages.js'),
  'metric-ring': () => import('../dist/metric-ring.js'),
  'mouse-follow': () => import('../dist/mouse-follow.js'),
  'neon-ring': () => import('../dist/neon-ring.js'),
  'status-pulse': () => import('../dist/status-pulse.js'),
};

/**
 * Creates a React wrapper for a GAME Web Component.
 * Forwards all props as element properties (not attributes) for live data binding.
 */
function createGameComponent(tagName, displayName) {
  const loaded = { current: false };

  const Component = forwardRef(function GameComponent(props, forwardedRef) {
    const innerRef = useRef(null);
    const ref = forwardedRef || innerRef;

    // Ensure the custom element is registered
    useEffect(() => {
      if (!loaded.current && componentModules[tagName]) {
        loaded.current = true;
        componentModules[tagName]();
      }
    }, []);

    // Forward props as properties to the custom element
    useEffect(() => {
      const el = typeof ref === 'function' ? null : ref.current;
      if (!el) return;

      for (const [key, value] of Object.entries(props)) {
        if (key === 'style' || key === 'className' || key === 'children') continue;
        el[key] = value;
      }
    });

    const { style, className, children, ...rest } = props;

    // Filter out data props — they're set via properties above
    const htmlProps = { ref, style, className };

    return createElement(tagName, htmlProps, children);
  });

  Component.displayName = displayName;
  return Component;
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
