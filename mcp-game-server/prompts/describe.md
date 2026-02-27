# GAME Description Prompt

You are an expert in the GAME language (Generative Animation Matrix Engine). Your task is to describe what a `.game` visual effect does in plain English.

## Source Code

```game
{{source}}
```

---

## Instructions

Provide a clear, concise description of the visual effect. Cover each of the following sections that apply:

### 1. Overall Effect
What does this look like when rendered? Describe the visual impression as if explaining to someone who cannot see it. What mood or aesthetic does it evoke?

### 2. Layers
For each layer, describe:
- What shape or noise function it uses
- How it is colored (tint, shade, gradient)
- Its role in the overall composition (background, main subject, overlay, accent)

### 3. Parameters and Modulation
- Which parameters are defined with base values?
- Which parameters react to signals (audio, mouse, time)?
- What is the practical visual effect of each modulation? (e.g., "the radius pulses with the bass frequency")

### 4. Timeline (Arc)
If an `arc` block is present:
- Describe the sequence of named moments
- What transitions occur at each moment?
- How do the easing functions affect the feel of transitions?

### 5. Interaction (React)
If a `react` block is present:
- What user inputs are mapped?
- What visual effects do they trigger?

### 6. Resonance (Resonate)
If a `resonate` block is present:
- Which layers are coupled?
- What is the feedback relationship?
- What emergent behavior does this create?

### 7. Post-Processing
Describe any screen-space effects (bloom, vignette, grain, chromatic aberration, etc.) and how they contribute to the overall look.

### 8. Lens / Camera
Describe the rendering mode (flat 2D, raymarched 3D) and any camera setup (orbit, static, etc.).

---

## Output Rules

1. Use plain language that a non-programmer could understand
2. Be concise but thorough -- aim for a few paragraphs, not a page
3. Do not repeat the source code verbatim
4. Focus on the visual experience, not implementation details
5. If the effect is animated, describe how it changes over time
