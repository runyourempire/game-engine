#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'fs';
import { resolve, join } from 'path';

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);

if (args.includes('--help') || args.includes('-h') || args.length === 0) {
  console.log(`
  create-game - Scaffold a new GAME component project

  Usage:
    npx create-game <project-name> [options]

  Options:
    --template <name>   Template to use (default: "hello")
                        Available: hello, loading-ring, dashboard-gauge, spectrum
    --react             Include a React wrapper example (App.jsx)
    --vue               Include a Vue wrapper example (App.vue)
    --svelte            Include a Svelte wrapper example (App.svelte)
    --help, -h          Show this help message

  Examples:
    npx create-game my-component
    npx create-game my-loader --template loading-ring
    npx create-game my-gauge --template dashboard-gauge --react
`);
  process.exit(0);
}

// Extract project name (first non-flag argument)
let projectName = null;
let template = 'hello';
let useReact = false;
let useVue = false;
let useSvelte = false;

for (let i = 0; i < args.length; i++) {
  const arg = args[i];
  if (arg === '--template') {
    template = args[++i];
  } else if (arg === '--react') {
    useReact = true;
  } else if (arg === '--vue') {
    useVue = true;
  } else if (arg === '--svelte') {
    useSvelte = true;
  } else if (!arg.startsWith('-')) {
    projectName = arg;
  }
}

if (!projectName) {
  console.error('Error: Please provide a project name.');
  console.error('  npx create-game my-component');
  process.exit(1);
}

const validTemplates = ['hello', 'loading-ring', 'dashboard-gauge', 'spectrum'];
if (!validTemplates.includes(template)) {
  console.error(`Error: Unknown template "${template}".`);
  console.error(`  Available templates: ${validTemplates.join(', ')}`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Template definitions
// ---------------------------------------------------------------------------

const templates = {
  hello: {
    tagName: 'game-hello',
    gameFile: `// ${projectName} — A glowing circle component
// Compiled with: game build .

@component ${projectName} {
  @canvas(400, 400)

  @state {
    time: 0
  }

  @vertex {
    // Full-screen quad
    let positions = array<vec2f>(
      vec2f(-1, -1), vec2f(1, -1), vec2f(1, 1),
      vec2f(-1, -1), vec2f(1, 1), vec2f(-1, 1)
    );
    output.position = vec4f(positions[vertex_index], 0, 1);
    output.uv = positions[vertex_index] * 0.5 + 0.5;
  }

  @fragment {
    let center = vec2f(0.5, 0.5);
    let dist = distance(input.uv, center);
    let glow = smoothstep(0.3, 0.0, dist);
    let pulse = sin(state.time * 2.0) * 0.5 + 0.5;
    let color = vec3f(0.2, 0.6, 1.0) * glow * (0.5 + pulse * 0.5);
    output.color = vec4f(color, glow);
  }

  @tick {
    state.time += dt;
  }
}
`,
    description: 'A glowing, pulsing circle with GPU-accelerated animation',
  },
  'loading-ring': {
    tagName: 'loading-ring',
    gameFile: `// ${projectName} — A loading ring component
// Compiled with: game build .

@component ${projectName} {
  @canvas(200, 200)

  @state {
    time: 0
    progress: 0
  }

  @vertex {
    let positions = array<vec2f>(
      vec2f(-1, -1), vec2f(1, -1), vec2f(1, 1),
      vec2f(-1, -1), vec2f(1, 1), vec2f(-1, 1)
    );
    output.position = vec4f(positions[vertex_index], 0, 1);
    output.uv = positions[vertex_index] * 0.5 + 0.5;
  }

  @fragment {
    let center = vec2f(0.5, 0.5);
    let uv = input.uv - center;
    let angle = atan2(uv.y, uv.x);
    let dist = length(uv);
    let ring = smoothstep(0.02, 0.0, abs(dist - 0.35));
    let sweep = step(angle, state.time * 3.0 - 3.14159);
    let color = vec3f(1.0, 1.0, 1.0) * ring * sweep;
    output.color = vec4f(color, ring * sweep);
  }

  @tick {
    state.time += dt;
  }
}
`,
    description: 'An animated loading ring with sweep animation',
  },
  'dashboard-gauge': {
    tagName: 'dashboard-gauge',
    gameFile: `// ${projectName} — A dashboard gauge component
// Compiled with: game build .

@component ${projectName} {
  @canvas(300, 300)

  @state {
    value: 0.65
    time: 0
  }

  @vertex {
    let positions = array<vec2f>(
      vec2f(-1, -1), vec2f(1, -1), vec2f(1, 1),
      vec2f(-1, -1), vec2f(1, 1), vec2f(-1, 1)
    );
    output.position = vec4f(positions[vertex_index], 0, 1);
    output.uv = positions[vertex_index] * 0.5 + 0.5;
  }

  @fragment {
    let center = vec2f(0.5, 0.35);
    let uv = input.uv - center;
    let angle = atan2(uv.y, uv.x);
    let dist = length(uv);
    let arc = smoothstep(0.02, 0.0, abs(dist - 0.4));
    let gauge_angle = -3.14159 + state.value * 3.14159;
    let fill = step(angle, gauge_angle) * step(-3.14159, angle);
    let color = mix(vec3f(0.2, 0.2, 0.2), vec3f(0.2, 0.8, 0.4), fill);
    output.color = vec4f(color * arc, arc);
  }

  @tick {
    state.time += dt;
  }
}
`,
    description: 'A semicircular gauge that displays a value from 0 to 1',
  },
  spectrum: {
    tagName: 'game-spectrum',
    gameFile: `// ${projectName} — A spectrum visualizer component
// Compiled with: game build .

@component ${projectName} {
  @canvas(600, 200)

  @state {
    time: 0
  }

  @vertex {
    let positions = array<vec2f>(
      vec2f(-1, -1), vec2f(1, -1), vec2f(1, 1),
      vec2f(-1, -1), vec2f(1, 1), vec2f(-1, 1)
    );
    output.position = vec4f(positions[vertex_index], 0, 1);
    output.uv = positions[vertex_index] * 0.5 + 0.5;
  }

  @fragment {
    let bars = 16.0;
    let bar_idx = floor(input.uv.x * bars);
    let bar_x = fract(input.uv.x * bars);
    let height = sin(bar_idx * 0.5 + state.time * 3.0) * 0.5 + 0.5;
    let bar = step(input.uv.y, height) * step(0.1, bar_x) * step(bar_x, 0.9);
    let hue = bar_idx / bars;
    let color = vec3f(
      sin(hue * 6.28) * 0.5 + 0.5,
      sin(hue * 6.28 + 2.09) * 0.5 + 0.5,
      sin(hue * 6.28 + 4.19) * 0.5 + 0.5
    );
    output.color = vec4f(color * bar, bar);
  }

  @tick {
    state.time += dt;
  }
}
`,
    description: 'An animated spectrum bar visualizer with rainbow colors',
  },
};

// ---------------------------------------------------------------------------
// Scaffold the project
// ---------------------------------------------------------------------------

const projectDir = resolve(process.cwd(), projectName);
const tpl = templates[template];

mkdirSync(projectDir, { recursive: true });

// .game file
writeFileSync(
  join(projectDir, `${projectName}.game`),
  tpl.gameFile
);

// package.json
const pkg = {
  name: projectName,
  version: '0.1.0',
  private: true,
  type: 'module',
  description: tpl.description,
  scripts: {
    build: 'game build .',
    dev: 'game dev .',
    preview: 'open index.html',
  },
  dependencies: {
    'game-components': '^0.3.0',
  },
};

if (useReact) {
  pkg.dependencies['react'] = '^18.0.0';
  pkg.dependencies['react-dom'] = '^18.0.0';
}
if (useVue) {
  pkg.dependencies['vue'] = '^3.0.0';
}
if (useSvelte) {
  pkg.dependencies['svelte'] = '^4.0.0';
}

writeFileSync(
  join(projectDir, 'package.json'),
  JSON.stringify(pkg, null, 2) + '\n'
);

// index.html
const indexHtml = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>${projectName}</title>
  <style>
    body {
      margin: 0;
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      background: #0a0a0a;
      color: #fff;
      font-family: system-ui, sans-serif;
    }
    .container {
      text-align: center;
    }
    h1 {
      font-size: 1.5rem;
      font-weight: 400;
      margin-bottom: 2rem;
      color: #a0a0a0;
    }
  </style>
</head>
<body>
  <div class="container">
    <h1>${projectName}</h1>
    <${tpl.tagName}></${tpl.tagName}>
  </div>
  <script type="module">
    import 'game-components/${template === 'hello' ? 'game-hello' : template === 'spectrum' ? 'game-spectrum' : template}';
  </script>
</body>
</html>
`;

writeFileSync(join(projectDir, 'index.html'), indexHtml);

// README.md
const readme = `# ${projectName}

${tpl.description}

Built with [GAME](https://github.com/runyourempire/game-engine) (GPU Accelerated Micro Effects).

## Getting started

\`\`\`bash
npm install
game build .
open index.html
\`\`\`

## Development

\`\`\`bash
game dev .
\`\`\`

This starts a dev server with hot-reload. Edit \`${projectName}.game\` and see changes instantly.

## Using in your project

\`\`\`html
<script type="module">
  import 'game-components/${template === 'hello' ? 'game-hello' : template === 'spectrum' ? 'game-spectrum' : template}';
</script>

<${tpl.tagName}></${tpl.tagName}>
\`\`\`
${useReact ? `
## React

\`\`\`jsx
import { ${toPascalCase(tpl.tagName)} } from 'game-components/react';

function App() {
  return <${toPascalCase(tpl.tagName)} />;
}
\`\`\`
` : ''}${useVue ? `
## Vue

\`\`\`vue
<script setup>
import { ${toPascalCase(tpl.tagName)} } from 'game-components/vue';
</script>

<template>
  <${toPascalCase(tpl.tagName)} />
</template>
\`\`\`
` : ''}${useSvelte ? `
## Svelte

\`\`\`svelte
<script>
  import { ${toCamelCase(tpl.tagName)} } from 'game-components/svelte';
</script>

<${tpl.tagName} use:${toCamelCase(tpl.tagName)} />
\`\`\`
` : ''}`;

writeFileSync(join(projectDir, 'README.md'), readme);

// React App.jsx (optional)
if (useReact) {
  const componentName = toPascalCase(tpl.tagName);
  const appJsx = `import React from 'react';
import { ${componentName} } from 'game-components/react';

export default function App() {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      minHeight: '100vh',
      background: '#0a0a0a',
    }}>
      <${componentName} />
    </div>
  );
}
`;
  writeFileSync(join(projectDir, 'App.jsx'), appJsx);
}

// Vue App.vue (optional)
if (useVue) {
  const componentName = toPascalCase(tpl.tagName);
  const appVue = `<script setup>
import { ${componentName} } from 'game-components/vue';
</script>

<template>
  <div class="app">
    <${componentName} />
  </div>
</template>

<style>
.app {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  background: #0a0a0a;
}
</style>
`;
  writeFileSync(join(projectDir, 'App.vue'), appVue);
}

// Svelte App.svelte (optional)
if (useSvelte) {
  const actionName = toCamelCase(tpl.tagName);
  const appSvelte = `<script>
  import { ${actionName} } from 'game-components/svelte';
</script>

<div class="app">
  <${tpl.tagName} use:${actionName} />
</div>

<style>
  .app {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: #0a0a0a;
  }
</style>
`;
  writeFileSync(join(projectDir, 'App.svelte'), appSvelte);
}

// ---------------------------------------------------------------------------
// Success message
// ---------------------------------------------------------------------------

console.log('');
console.log(`  GAME  Created ${projectName} with "${template}" template`);
console.log('');
console.log(`  cd ${projectName}`);
console.log('  npm install');
console.log('  game dev .');
console.log('');
if (useReact) console.log('  React wrapper: App.jsx');
if (useVue) console.log('  Vue wrapper:   App.vue');
if (useSvelte) console.log('  Svelte wrapper: App.svelte');
if (useReact || useVue || useSvelte) console.log('');

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function toPascalCase(kebab) {
  return kebab
    .split('-')
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
    .join('');
}

function toCamelCase(kebab) {
  const pascal = toPascalCase(kebab);
  return pascal.charAt(0).toLowerCase() + pascal.slice(1);
}
