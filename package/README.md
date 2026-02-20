# game-components

GPU-accelerated visual components compiled from [`.game` files](https://github.com/runyourempire/game-engine). Zero dependencies. WebGPU-powered.

## Install

```bash
npm install game-components
```

## Usage

### Script tag (CDN)

```html
<script type="module" src="https://esm.sh/game-components/loading-ring"></script>
<loading-ring progress="0.75"></loading-ring>
```

### ES Module

```js
import 'game-components/loading-ring';

// Use in HTML
document.body.innerHTML = '<loading-ring></loading-ring>';

// Live data binding
const ring = document.querySelector('loading-ring');
ring.progress = 0.75;
```

### React

```jsx
import { LoadingRing, MetricRing, StatusPulse } from 'game-components/react';

function Dashboard() {
  return (
    <div>
      <LoadingRing progress={0.75} style={{ width: 64, height: 64 }} />
      <MetricRing value={0.85} style={{ width: 48, height: 48 }} />
      <StatusPulse health={1.0} style={{ width: 32, height: 32 }} />
    </div>
  );
}
```

## Components

| Component | Tag | Props | Description |
|-----------|-----|-------|-------------|
| `BreathingDot` | `<breathing-dot>` | — | Ambient presence indicator, pure time-driven |
| `LoadingRing` | `<loading-ring>` | `progress` (0-1) | Determinate loading ring with gold tint |
| `MetricRing` | `<metric-ring>` | `value` (0-1) | Arc gauge for displaying a metric |
| `GameSpinner` | `<game-spinner>` | — | Indeterminate rotating spinner |
| `StatusPulse` | `<status-pulse>` | `health` (0-1) | Health/status indicator that glows with intensity |

## Requirements

WebGPU-capable browser (Chrome 113+, Edge 113+, Firefox Nightly). Components show a graceful fallback message when WebGPU is unavailable.

## License

MIT
