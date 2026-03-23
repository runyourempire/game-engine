/// Root of a GAME program.
#[derive(Debug, Clone)]
pub struct Program {
    pub imports: Vec<Import>,
    pub cinematics: Vec<Cinematic>,
    pub breeds: Vec<BreedBlock>,
    pub projects: Vec<ProjectBlock>,
    pub fns: Vec<FnDef>,
    pub scenes: Vec<SceneBlock>,
    pub ifs_blocks: Vec<IfsBlock>,
    pub lsystem_blocks: Vec<LsystemBlock>,
    pub automaton_blocks: Vec<AutomatonBlock>,
    pub matrix_blocks: Vec<MatrixBlock>,
}

/// `fn name(params) { pipeline }`
#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stage>,
}

/// `import "path" as alias`
#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub alias: String,
}

/// `cinematic "name" { layers, arcs, resonates, listen, voice, score, gravity, react, swarm, flow }`
#[derive(Debug, Clone)]
pub struct Cinematic {
    pub name: String,
    pub layers: Vec<Layer>,
    pub arcs: Vec<ArcBlock>,
    pub resonates: Vec<ResonateBlock>,
    pub listen: Option<ListenBlock>,
    pub voice: Option<VoiceBlock>,
    pub score: Option<ScoreBlock>,
    pub gravity: Option<GravityBlock>,
    pub react: Option<ReactBlock>,
    pub swarm: Option<SwarmBlock>,
    pub flow: Option<FlowBlock>,
    pub particles: Option<ParticlesBlock>,
    /// Post-processing passes within this cinematic.
    pub passes: Vec<PassBlock>,
    /// References to other cinematics used as texture inputs.
    pub cinematic_uses: Vec<CinematicUse>,
    /// Coupling matrix for bidirectional parameter coupling.
    pub matrix_coupling: Option<MatrixCoupling>,
    /// Color matrix for 3x3 RGB color grading.
    pub matrix_color: Option<MatrixColor>,
    /// Component properties (string, number, event).
    pub props: Option<PropsBlock>,
    /// DOM overlay elements (text, positioned on top of canvas).
    pub dom: Option<DomBlock>,
    /// Event handlers (click, hover, etc.).
    pub events: Vec<EventHandler>,
    /// ARIA role for accessibility.
    pub role: Option<String>,
    /// 3D ray marching scene configuration.
    pub scene3d: Option<Scene3dBlock>,
    /// Texture inputs (external images bound to shader samplers).
    pub textures: Vec<TextureDecl>,
}

// ── 3D Ray Marching ─────────────────────────────────────

/// Camera mode for 3D scene rendering.
#[derive(Debug, Clone, PartialEq)]
pub enum CameraMode {
    /// Mouse-controlled orbit around the origin.
    Orbit,
    /// Fixed camera position (static view).
    Static,
    /// WASD fly-through camera.
    Fly,
}

/// `scene3d { camera: mode, fov: degrees, distance: units }`
#[derive(Debug, Clone)]
pub struct Scene3dBlock {
    pub camera: CameraMode,
    pub fov: f64,
    pub distance: f64,
}

/// `pass name { pipeline }` — post-processing pass within a cinematic.
#[derive(Debug, Clone)]
pub struct PassBlock {
    pub name: String,
    pub body: Vec<Stage>,
}

/// `use "cinematic-name" as alias` inside a cinematic block.
#[derive(Debug, Clone)]
pub struct CinematicUse {
    pub source: String,
    pub alias: String,
}

/// Layer blend mode for multi-layer compositing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    Add,
    Screen,
    Multiply,
    Overlay,
    /// Standard alpha blending: src*alpha + dst*(1-alpha).
    /// Creates opaque surfaces that mask what's underneath.
    Occlude,
}

/// `layer ident [(opts)] [memory: f] [opacity: f] [cast kind] [blend mode] [feedback: bool] { body }`
#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub opts: Vec<Param>,
    pub memory: Option<f64>,
    pub opacity: Option<f64>,
    pub cast: Option<String>,
    pub blend: BlendMode,
    pub feedback: bool,
    pub body: LayerBody,
}

/// A layer body is either a list of named params, a stage pipeline, or a conditional.
#[derive(Debug, Clone)]
pub enum LayerBody {
    Params(Vec<Param>),
    Pipeline(Vec<Stage>),
    Conditional {
        condition: Expr,
        then_branch: Vec<Stage>,
        else_branch: Vec<Stage>,
    },
}

/// `name: value [~ modulation] [temporal_ops]*`
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub value: Expr,
    pub modulation: Option<Expr>,
    pub temporal_ops: Vec<TemporalOp>,
}

/// Temporal operator applied to a parameter value.
#[derive(Debug, Clone)]
pub enum TemporalOp {
    /// `>> duration` — delay via ring buffer
    Delay(Duration),
    /// `<> duration` — exponential moving average smoothing
    Smooth(Duration),
    /// `!! duration` — edge-detect trigger with decay envelope
    Trigger(Duration),
    /// `.. [min, max]` — range clamp
    Range(Expr, Expr),
}

/// A single stage in a pipeline: `name(args)`
#[derive(Debug, Clone)]
pub struct Stage {
    pub name: String,
    pub args: Vec<Arg>,
}

/// An argument — optionally named.
#[derive(Debug, Clone)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Expr,
}

/// Lifecycle state for an arc block.
#[derive(Debug, Clone, PartialEq)]
pub enum ArcState {
    /// Plays once on element connect, then holds final value.
    Enter,
    /// Plays once on programmatic trigger, then holds final value.
    Exit,
    /// Plays on mouseenter, reverses on mouseleave.
    Hover,
    /// Loops continuously (default for unnamed arc blocks).
    Idle,
}

/// `arc [state] { entries }`
#[derive(Debug, Clone)]
pub struct ArcBlock {
    /// Optional lifecycle state. `None` means backward-compatible looping (same as Idle).
    pub state: Option<ArcState>,
    pub entries: Vec<ArcEntry>,
}

/// `target: from -> to over duration [easing]`
#[derive(Debug, Clone)]
pub struct ArcEntry {
    pub target: String,
    pub from: Expr,
    pub to: Expr,
    pub duration: Duration,
    pub easing: Option<String>,
}

/// `resonate { entries }`
#[derive(Debug, Clone)]
pub struct ResonateBlock {
    pub entries: Vec<ResonateEntry>,
}

/// `source -> target.field * weight`
#[derive(Debug, Clone)]
pub struct ResonateEntry {
    pub source: String,
    pub target: String,
    pub field: String,
    pub weight: Expr,
}

/// Time durations supported by the language.
#[derive(Debug, Clone, PartialEq)]
pub enum Duration {
    Seconds(f64),
    Millis(f64),
    Bars(i64),
}

/// Binary operators.
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    NotEq,
}

/// Expression tree.
#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    String(String),
    Ident(String),
    /// RGB color from hex literal (0.0-1.0 each)
    Color(f64, f64, f64),
    DottedIdent {
        object: String,
        field: String,
    },
    Array(Vec<Expr>),
    Paren(Box<Expr>),
    Neg(Box<Expr>),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Arg>,
    },
    Duration(Duration),
}

// ── Phase 3: Audio blocks ────────────────────────────────

/// `listen { signal_name: algorithm(params) ... }`
#[derive(Debug, Clone)]
pub struct ListenBlock {
    pub signals: Vec<ListenSignal>,
}

/// A named audio signal with its DSP algorithm.
#[derive(Debug, Clone)]
pub struct ListenSignal {
    pub name: String,
    pub algorithm: String,
    pub params: Vec<Param>,
}

/// `voice { oscillators, filters, output chain }`
#[derive(Debug, Clone)]
pub struct VoiceBlock {
    pub nodes: Vec<VoiceNode>,
}

/// A node in the voice synthesis graph.
#[derive(Debug, Clone)]
pub struct VoiceNode {
    pub name: String,
    pub kind: String,
    pub params: Vec<Param>,
}

// ── Phase 4: Composition blocks ──────────────────────────

/// `score tempo(BPM) { motifs, phrases, sections, arrange }`
#[derive(Debug, Clone)]
pub struct ScoreBlock {
    pub tempo_bpm: f64,
    pub motifs: Vec<Motif>,
    pub phrases: Vec<Phrase>,
    pub sections: Vec<Section>,
    pub arrange: Vec<String>,
}

/// `motif name { target: from -> to over duration }`
#[derive(Debug, Clone)]
pub struct Motif {
    pub name: String,
    pub entries: Vec<ArcEntry>,
}

/// `phrase name = motif1 | motif2 | ...`
#[derive(Debug, Clone)]
pub struct Phrase {
    pub name: String,
    pub motifs: Vec<String>,
}

/// `section name = phrase1 phrase2 ...`
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub phrases: Vec<String>,
}

// ── Phase 4B: Breed ──────────────────────────────────────

/// `breed "name" from "parent1" + "parent2" { rules }`
#[derive(Debug, Clone)]
pub struct BreedBlock {
    pub name: String,
    pub parents: Vec<String>,
    pub inherit_rules: Vec<InheritRule>,
    pub mutations: Vec<Mutation>,
}

/// `inherit layers|params: mix(weight)`
#[derive(Debug, Clone)]
pub struct InheritRule {
    pub target: String,
    pub strategy: String,
    pub weight: f64,
}

/// `mutate param: +/-range`
#[derive(Debug, Clone)]
pub struct Mutation {
    pub target: String,
    pub range: f64,
}

// ── Phase 5: Physics + Spatial ───────────────────────────

/// `gravity { rule, damping, bounds }`
#[derive(Debug, Clone)]
pub struct GravityBlock {
    pub force_law: Expr,
    pub damping: f64,
    pub bounds: BoundsMode,
}

/// How particles interact with boundaries.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundsMode {
    Reflect,
    Wrap,
    None,
}

/// `project mode(params) { source, warp }`
#[derive(Debug, Clone)]
pub struct ProjectBlock {
    pub mode: ProjectMode,
    pub source: String,
    pub params: Vec<Param>,
}

/// Projection target surface type.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectMode {
    Flat,
    Dome,
    Cube,
    Led,
}

// ── Phase 6: Emergent Systems ────────────────────────────

/// `react { feed, kill, diffuse_a, diffuse_b, seed }`
/// Gray-Scott reaction-diffusion on ping-pong textures.
#[derive(Debug, Clone)]
pub struct ReactBlock {
    pub feed: f64,
    pub kill: f64,
    pub diffuse_a: f64,
    pub diffuse_b: f64,
    pub seed: SeedMode,
}

/// Initial perturbation for reaction-diffusion.
#[derive(Debug, Clone, PartialEq)]
pub enum SeedMode {
    /// Centered blob of chemical B
    Center(f64),
    /// Random scattered points
    Scatter(u32),
    /// Uniform random field
    Random(f64),
}

/// `swarm { agents, sensor_angle, sensor_dist, turn_angle, step, deposit, decay, bounds }`
/// Physarum polycephalum stigmergic agent simulation.
#[derive(Debug, Clone)]
pub struct SwarmBlock {
    pub agents: u32,
    pub sensor_angle: f64,
    pub sensor_dist: f64,
    pub turn_angle: f64,
    pub step_size: f64,
    pub deposit: f64,
    pub decay: f64,
    pub diffuse: u32,
    pub bounds: BoundsMode,
}

/// `particles { count, emit, lifetime, speed, spread, gravity, size, fade, color }`
/// General-purpose GPU-accelerated particle system.
#[derive(Debug, Clone)]
pub struct ParticlesBlock {
    pub count: u32,
    pub emit: EmitMode,
    pub lifetime: f64,
    pub speed: f64,
    pub spread: f64, // degrees
    pub gravity: f64,
    pub size: f64,
    pub fade: bool,
    pub color: String, // palette name or "white"
}

/// Particle emission origin mode.
#[derive(Debug, Clone, PartialEq)]
pub enum EmitMode {
    /// Emit from center of canvas.
    Center,
    /// Emit from random positions.
    Random,
    /// Emit from a ring of given radius (0.0-1.0).
    Ring(f64),
    /// Emit from a specific point (x, y) in UV space.
    Point(f64, f64),
}

/// `flow { type, scale, speed, octaves, strength, bounds }`
/// Curl noise vector field for particle advection.
#[derive(Debug, Clone)]
pub struct FlowBlock {
    pub flow_type: FlowType,
    pub scale: f64,
    pub speed: f64,
    pub octaves: u32,
    pub strength: f64,
    pub bounds: BoundsMode,
}

/// Vector field generation algorithm.
#[derive(Debug, Clone, PartialEq)]
pub enum FlowType {
    Curl,
    Perlin,
    Simplex,
    Vortex,
}

// ── Phase v0.6: Matrix Generation ────────────────────────

/// `ifs { transforms, iterations, color_mode }`
/// Iterated Function System — chaos game fractal generation.
#[derive(Debug, Clone)]
pub struct IfsBlock {
    pub transforms: Vec<IfsTransform>,
    pub iterations: u32,
    pub color_mode: IfsColorMode,
}

/// A single affine transform in an IFS.
#[derive(Debug, Clone)]
pub struct IfsTransform {
    pub name: String,
    pub matrix: [f64; 6],
    pub weight: f64,
}

/// How to color the IFS fractal.
#[derive(Debug, Clone, PartialEq)]
pub enum IfsColorMode {
    Transform,
    Depth,
    Position,
}

/// `lsystem { axiom, rules, angle, iterations, step }`
/// L-system string rewriting for generative geometry.
#[derive(Debug, Clone)]
pub struct LsystemBlock {
    pub axiom: String,
    pub rules: Vec<LsystemRule>,
    pub angle: f64,
    pub iterations: u32,
    pub step: f64,
}

/// A single L-system rewriting rule.
#[derive(Debug, Clone)]
pub struct LsystemRule {
    pub symbol: char,
    pub replacement: String,
}

/// `automaton { states, neighborhood, rule, seed, speed }`
/// Cellular automaton on a 2D grid.
#[derive(Debug, Clone)]
pub struct AutomatonBlock {
    pub states: u32,
    pub neighborhood: Neighborhood,
    pub rule: String,
    pub seed: AutomatonSeed,
    pub speed: u32,
}

/// Neighborhood type for cellular automata.
#[derive(Debug, Clone, PartialEq)]
pub enum Neighborhood {
    Moore,
    VonNeumann,
}

/// Initial seed pattern for cellular automata.
#[derive(Debug, Clone, PartialEq)]
pub enum AutomatonSeed {
    Random(f64),
    Center,
    Pattern(String),
}

// ── Phase v0.7: Matrix System ────────────────────────────

/// A `matrix` block — one of three matrix forms.
#[derive(Debug, Clone)]
pub enum MatrixBlock {
    /// `matrix coupling { sources, targets, weights, damping, depth }`
    Coupling(MatrixCoupling),
    /// `matrix color { 3x3 values }`
    Color(MatrixColor),
    /// `matrix transitions "name" { states, weights, hold }`
    Transitions(MatrixTransitions),
}

/// Coupling matrix — bidirectional NxM parameter coupling grid.
#[derive(Debug, Clone)]
pub struct MatrixCoupling {
    pub sources: Vec<String>,
    pub targets: Vec<MatrixTarget>,
    pub weights: Vec<f64>,
    pub damping: f64,
    pub depth: u32,
}

/// A coupling target: `layer.field`
#[derive(Debug, Clone)]
pub struct MatrixTarget {
    pub layer: String,
    pub field: String,
}

/// Color matrix — 3x3 RGB color grading transform.
#[derive(Debug, Clone)]
pub struct MatrixColor {
    pub values: [f64; 9],
}

/// Transition matrix — Markov chain probabilistic scene sequencing.
#[derive(Debug, Clone)]
pub struct MatrixTransitions {
    pub name: String,
    pub states: Vec<String>,
    pub weights: Vec<f64>,
    pub hold: Duration,
}

// ── Phase v0.8: Component UI Layer ──────────────────────

/// `props { name: default, ... }` — typed component properties.
#[derive(Debug, Clone)]
pub struct PropsBlock {
    pub props: Vec<PropDef>,
}

/// A single property definition.
#[derive(Debug, Clone)]
pub struct PropDef {
    pub name: String,
    /// Default value — `Expr::String` for string props, `Expr::Number` for number props.
    pub default: Expr,
    /// True if declared as an event (no default, emits custom events).
    pub is_event: bool,
}

/// `dom { text "name" { at: x y, style: "css", bind: "prop" } ... }`
#[derive(Debug, Clone)]
pub struct DomBlock {
    pub elements: Vec<DomElement>,
}

/// A positioned DOM element overlaid on the GPU canvas.
#[derive(Debug, Clone)]
pub struct DomElement {
    /// Element type: "text", "div", etc.
    pub tag: String,
    /// Identifier for this element.
    pub name: String,
    /// X position as CSS value (e.g. "72px", "50%").
    pub x: String,
    /// Y position as CSS value (e.g. "12px", "25%").
    pub y: String,
    /// CSS style string.
    pub style: String,
    /// Prop name whose value is bound to textContent.
    pub bind: Option<String>,
    /// Width constraint as CSS value (e.g. "200px", "100%").
    pub width: Option<String>,
    /// Text alignment: "left", "center", or "right".
    pub align: Option<String>,
}

/// `on "click" { emit: "dismiss" }` — event handler declaration.
#[derive(Debug, Clone)]
pub struct EventHandler {
    /// DOM event name (click, mouseenter, mouseleave, etc.)
    pub event: String,
    /// Custom event to dispatch when triggered.
    pub emit: Option<String>,
}

// ── Texture inputs ───────────────────────────────────────

/// `texture "name" [from "url"]` — external image/texture input.
#[derive(Debug, Clone)]
pub struct TextureDecl {
    pub name: String,
    /// Optional URL or path — can also be set from JavaScript at runtime.
    pub source: Option<String>,
}

// ── Phase v0.5: Scene sequencing ─────────────────────────

/// `scene "name" { play/transition entries }`
#[derive(Debug, Clone)]
pub struct SceneBlock {
    pub name: String,
    pub entries: Vec<SceneEntry>,
}

/// A scene timeline entry — either play a cinematic or transition between them.
#[derive(Debug, Clone)]
pub enum SceneEntry {
    Play {
        cinematic: String,
        duration: Duration,
    },
    Transition {
        kind: TransitionKind,
        duration: Duration,
    },
}

/// Transition visual effect between cinematics.
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionKind {
    Dissolve,
    Fade,
    Wipe,
    Morph,
}
