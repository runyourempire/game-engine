import { createElement, useRef, useEffect, forwardRef } from 'react';

// Lazy-import the Web Components so they self-register on first use
const componentModules = {
  'breathing-dot': () => import('../dist/breathing-dot.js'),
  'loading-ring': () => import('../dist/loading-ring.js'),
  'metric-ring': () => import('../dist/metric-ring.js'),
  'game-spinner': () => import('../dist/game-spinner.js'),
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

export const BreathingDot = createGameComponent('breathing-dot', 'BreathingDot');
export const LoadingRing = createGameComponent('loading-ring', 'LoadingRing');
export const MetricRing = createGameComponent('metric-ring', 'MetricRing');
export const GameSpinner = createGameComponent('game-spinner', 'GameSpinner');
export const StatusPulse = createGameComponent('status-pulse', 'StatusPulse');
