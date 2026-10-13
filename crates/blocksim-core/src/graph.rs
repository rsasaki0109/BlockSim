//! Typed vector graphs, subsystem expansion and hybrid RK4/RK45 execution.
use crate::{
    finite, positive, ContinuousPid, DiscretePid, OutputLimits, PidConfig, PidMode, Result,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
mod adaptive;
pub mod discretize;
pub mod frequency;
pub mod pole_zero;
mod sampled;
pub mod state_space;
mod transfer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum SolverConfig {
    Rk4 {},
    Rk45 {
        abs_tolerance: f64,
        rel_tolerance: f64,
        min_step: f64,
        max_step: f64,
    },
}
impl SolverConfig {
    fn validate(&self) -> Result<()> {
        if let Self::Rk45 {
            abs_tolerance,
            rel_tolerance,
            min_step,
            max_step,
        } = self
        {
            positive("Absolute tolerance", *abs_tolerance)?;
            finite("Relative tolerance", *rel_tolerance)?;
            positive("Minimum step", *min_step)?;
            positive("Maximum step", *max_step)?;
            if *rel_tolerance < 0.0 || *rel_tolerance > 1.0 || min_step > max_step {
                return Err(
                    "RK45 requires 0 <= relative tolerance <= 1 and min_step <= max_step".into(),
                );
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverStatistics {
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub rate_evaluations: usize,
    pub output_evaluations: usize,
    pub min_step: f64,
    pub max_step: f64,
    pub max_error_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphModel {
    pub version: u32,
    pub dt: f64,
    pub duration: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solver: Option<SolverConfig>,
    pub blocks: Vec<Block>,
    pub edges: Vec<Edge>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub id: String,
    pub label: String,
    pub width: usize,
    pub position: [f64; 2],
    pub params: BlockKind,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockKind {
    Constant {
        value: Vec<f64>,
    },
    SampledInput {
        values: Vec<Vec<f64>>,
        sample_time: f64,
    },
    Step {
        before: Vec<f64>,
        after: Vec<f64>,
        step_time: f64,
    },
    Sine {
        amplitude: f64,
        frequency: f64,
        phase: f64,
        offset: f64,
    },
    Gain {
        gain: f64,
    },
    Sum {
        signs: Vec<f64>,
    },
    Product {
        inputs: usize,
    },
    Saturation {
        limits: OutputLimits,
    },
    Integrator {
        initial: Vec<f64>,
    },
    FirstOrder {
        gain: f64,
        tau: f64,
        initial: Vec<f64>,
    },
    TransferFunction {
        numerator: Vec<f64>,
        denominator: Vec<f64>,
        initial: Vec<f64>,
    },
    DiscreteTransferFunction {
        numerator: Vec<f64>,
        denominator: Vec<f64>,
        initial: Vec<f64>,
        sample_time: f64,
    },
    Delay {
        initial: Vec<f64>,
        sample_time: f64,
    },
    Pid {
        config: PidConfig,
        mode: PidMode,
        sample_time: f64,
    },
    Mux {
        inputs: usize,
    },
    Selector {
        indices: Vec<usize>,
    },
    Scope {},
    Input {
        port: usize,
    },
    Output {
        port: usize,
    },
    Subsystem {
        model: Box<GraphModel>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub from_port: usize,
    pub to: String,
    pub to_port: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub id: String,
    pub label: String,
    pub values: Vec<Vec<f64>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResult {
    pub times: Vec<f64>,
    pub traces: Vec<Trace>,
    pub solver: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics: Option<SolverStatistics>,
}
#[derive(Clone)]
struct Flat {
    blocks: Vec<Block>,
    edges: Vec<Edge>,
}
#[derive(Clone)]
struct Boundary {
    inputs: Vec<String>,
    outputs: Vec<String>,
}
struct Compiled {
    flat: Flat,
    incoming: Vec<Vec<usize>>,
    order: Vec<usize>,
    scopes: Vec<usize>,
}

impl BlockKind {
    fn inputs(&self) -> usize {
        match self {
            Self::Constant { .. }
            | Self::SampledInput { .. }
            | Self::Step { .. }
            | Self::Sine { .. } => 0,
            Self::Sum { signs } => signs.len(),
            Self::Product { inputs } | Self::Mux { inputs } => *inputs,
            _ => 1,
        }
    }
    fn state_output(&self) -> bool {
        matches!(
            self,
            Self::Integrator { .. } | Self::FirstOrder { .. } | Self::Delay { .. }
        ) || matches!(self, Self::TransferFunction { numerator, denominator, .. } | Self::DiscreteTransferFunction { numerator, denominator, .. }
            if denominator.len() > 1 && (numerator.len() < denominator.len() || numerator.first() == Some(&0.0)))
    }
}
fn identifier(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
fn vector(name: &str, v: &[f64], width: usize) -> Result<()> {
    if v.len() != width {
        return Err(format!("{name}: expected vector width {width}"));
    }
    for x in v {
        finite(name, *x)?;
    }
    Ok(())
}
fn count(name: &str, n: usize) -> Result<()> {
    if (1..=64).contains(&n) {
        Ok(())
    } else {
        Err(format!("{name}: expected 1..64"))
    }
}
fn period(sample_time: f64, dt: f64) -> Result<usize> {
    positive("Sample time", sample_time)?;
    let ticks = sample_time / dt;
    if !(1.0..=100_000.0).contains(&ticks) || (ticks - ticks.round()).abs() > 1e-8 {
        return Err("Sample time must be an integer multiple of graph dt".into());
    }
    Ok(ticks.round() as usize)
}
fn validate_block(b: &Block, dt: f64, adaptive: bool) -> Result<()> {
    count("Signal width", b.width)?;
    if b.label.len() > 128 {
        return Err("Block label exceeds 128 bytes".into());
    }
    for p in b.position {
        finite("Block position", p)?;
        if !(0.0..=10000.0).contains(&p) {
            return Err("Block position must be within 0..10000".into());
        }
    }
    match &b.params {
        BlockKind::Constant { value } => vector("Constant", value, b.width)?,
        BlockKind::SampledInput {
            values,
            sample_time,
        } => {
            if !(1..=2001).contains(&values.len()) {
                return Err("SampledInput values must contain 1..2001 rows".into());
            }
            for row in values {
                vector("SampledInput row", row, b.width)?;
            }
            period(*sample_time, dt)?;
        }
        BlockKind::Step {
            before,
            after,
            step_time,
        } => {
            vector("Step before", before, b.width)?;
            vector("Step after", after, b.width)?;
            finite("Step time", *step_time)?;
            if *step_time < 0.0 {
                return Err("Step time must be nonnegative".into());
            }
        }
        BlockKind::Sine {
            amplitude,
            frequency,
            phase,
            offset,
        } => {
            for v in [amplitude, frequency, phase, offset] {
                finite("Sine parameter", *v)?;
            }
            if *frequency < 0.0 {
                return Err("Frequency must be nonnegative".into());
            }
        }
        BlockKind::Gain { gain } => finite("Gain", *gain)?,
        BlockKind::Sum { signs } => {
            count("Sum inputs", signs.len())?;
            for s in signs {
                finite("Sum coefficient", *s)?;
            }
        }
        BlockKind::Product { inputs } | BlockKind::Mux { inputs } => count("Inputs", *inputs)?,
        BlockKind::Saturation { limits } => limits.validate()?,
        BlockKind::Integrator { initial } => vector("Initial state", initial, b.width)?,
        BlockKind::FirstOrder { gain, tau, initial } => {
            finite("Plant gain", *gain)?;
            positive("Plant tau", *tau)?;
            vector("Initial state", initial, b.width)?;
            if !adaptive && dt > tau / 2.0 {
                return Err("RK4 requires dt <= plant tau/2".into());
            }
        }
        BlockKind::TransferFunction {
            numerator,
            denominator,
            initial,
        } => {
            transfer::coefficients(numerator, denominator)?;
            vector(
                "TransferFunction initial state",
                initial,
                b.width * (denominator.len() - 1),
            )?;
        }
        BlockKind::Delay {
            initial,
            sample_time,
        } => {
            vector("Initial delay", initial, b.width)?;
            period(*sample_time, dt)?;
        }
        BlockKind::DiscreteTransferFunction {
            numerator,
            denominator,
            initial,
            sample_time,
        } => {
            transfer::coefficients(numerator, denominator)
                .map_err(|e| e.replace("TransferFunction", "DiscreteTransferFunction"))?;
            vector(
                "DiscreteTransferFunction initial state",
                initial,
                b.width * (denominator.len() - 1),
            )?;
            period(*sample_time, dt)?;
        }
        BlockKind::Pid {
            config,
            mode,
            sample_time,
        } => {
            config.validate()?;
            positive("PID sample time", *sample_time)?;
            if *mode == PidMode::Discrete {
                period(*sample_time, dt)?;
            } else if !adaptive && dt > config.derivative_tau / 2.0 {
                return Err("RK4 requires dt <= derivative_tau/2".into());
            }
        }
        BlockKind::Selector { indices } => {
            if indices.len() != b.width {
                return Err("Selector indices must match output width".into());
            }
        }
        BlockKind::Scope {} | BlockKind::Input { .. } | BlockKind::Output { .. } => (),
        BlockKind::Subsystem { .. } => unreachable!("expanded before parameter validation"),
    }
    Ok(())
}
fn sorted_interface(ports: &BTreeMap<usize, String>, name: &str) -> Result<Vec<String>> {
    if ports.keys().copied().ne(0..ports.len()) {
        return Err(format!("{name} ports must be contiguous from zero"));
    }
    Ok(ports.values().cloned().collect())
}
fn expand(
    model: &GraphModel,
    prefix: &str,
    depth: usize,
    flat: &mut Flat,
    dt: f64,
    solver: &SolverConfig,
) -> Result<Boundary> {
    if depth > 8 {
        return Err("Subsystem nesting exceeds 8 levels".into());
    }
    if model.version != 1 {
        return Err("Unsupported graph version".into());
    }
    if depth > 0 && model.solver.as_ref().is_some_and(|child| child != solver) {
        return Err("Subsystem solver must inherit or match the root solver".into());
    }
    // Child clocks are required to match, so saved models have unambiguous semantics.
    if depth > 0 && (model.dt != dt || model.duration <= 0.0 || !model.duration.is_finite()) {
        return Err("Subsystem dt must match root dt and duration must be positive".into());
    }
    if model.blocks.len() > 256 || model.edges.len() > 4096 {
        return Err("Graph size limit exceeded".into());
    }
    let mut mappings = BTreeMap::<String, Boundary>::new();
    let mut ins = BTreeMap::new();
    let mut outs = BTreeMap::new();
    for b in &model.blocks {
        if !identifier(&b.id) {
            return Err(format!("Invalid block id: {}", b.id));
        }
        if mappings.contains_key(&b.id) {
            return Err(format!("Duplicate block id: {}", b.id));
        }
        let id = format!("{prefix}{}", b.id);
        let boundary = if let BlockKind::Subsystem { model: inner } = &b.params {
            count("Subsystem display width", b.width)?;
            for p in b.position {
                finite("Subsystem position", p)?;
                if !(0.0..=10000.0).contains(&p) {
                    return Err("Subsystem position must be within 0..10000".into());
                }
            }
            if b.label.len() > 128 {
                return Err("Subsystem label exceeds 128 bytes".into());
            }
            let boundary = expand(inner, &format!("{id}/"), depth + 1, flat, dt, solver)?;
            if boundary.outputs.is_empty() {
                return Err(format!("{id}: subsystem needs an Output"));
            }
            boundary
        } else {
            validate_block(b, dt, matches!(solver, SolverConfig::Rk45 { .. }))
                .map_err(|e| format!("{id}: {e}"))?;
            if let BlockKind::Input { port } = b.params {
                if depth == 0 {
                    return Err("Input blocks belong inside subsystems".into());
                }
                if ins.insert(port, id.clone()).is_some() {
                    return Err("Duplicate subsystem Input port".into());
                }
            }
            if let BlockKind::Output { port } = b.params {
                if depth == 0 {
                    return Err("Output blocks belong inside subsystems; use Scope at root".into());
                }
                if outs.insert(port, id.clone()).is_some() {
                    return Err("Duplicate subsystem Output port".into());
                }
            }
            let mut block = b.clone();
            block.id = id.clone();
            flat.blocks.push(block);
            Boundary {
                inputs: (0..b.params.inputs()).map(|_| id.clone()).collect(),
                outputs: vec![id],
            }
        };
        mappings.insert(b.id.clone(), boundary);
        if flat.blocks.len() > 256 {
            return Err("Expanded graph exceeds 256 blocks".into());
        }
        let input_cells: usize = flat
            .blocks
            .iter()
            .map(|b| match &b.params {
                BlockKind::SampledInput { values, .. } => values.len() * b.width,
                _ => 0,
            })
            .sum();
        if input_cells > 2_000_000 {
            return Err("SampledInput tables exceed 2000000 cells".into());
        }
    }
    let mut occupied = BTreeSet::new();
    for e in &model.edges {
        let source = mappings
            .get(&e.from)
            .ok_or_else(|| format!("Missing source block: {}", e.from))?;
        let target = mappings
            .get(&e.to)
            .ok_or_else(|| format!("Missing target block: {}", e.to))?;
        let from = source
            .outputs
            .get(e.from_port)
            .ok_or_else(|| format!("{}: invalid output port", e.from))?;
        let to = target
            .inputs
            .get(e.to_port)
            .ok_or_else(|| format!("{}: invalid input port", e.to))?;
        if !occupied.insert((e.to.clone(), e.to_port)) {
            return Err(format!("{}:{}: input already connected", e.to, e.to_port));
        }
        let target_is_subsystem = model
            .blocks
            .iter()
            .any(|b| b.id == e.to && matches!(b.params, BlockKind::Subsystem { .. }));
        flat.edges.push(Edge {
            from: from.clone(),
            from_port: 0,
            to: to.clone(),
            to_port: if target_is_subsystem { 0 } else { e.to_port },
        });
    }
    Ok(Boundary {
        inputs: sorted_interface(&ins, "Input")?,
        outputs: sorted_interface(&outs, "Output")?,
    })
}
fn structure(model: &GraphModel) -> Result<Flat> {
    let solver = model.solver.as_ref().unwrap_or(&SolverConfig::Rk4 {});
    solver.validate()?;
    positive("Graph dt", model.dt)?;
    positive("Graph duration", model.duration)?;
    if model.dt > model.duration || model.duration / model.dt > 100_000.0 {
        return Err("Graph requires 1..100000 time steps".into());
    }
    let ticks = model.duration / model.dt;
    if (ticks - ticks.round()).abs() > 1e-8 {
        return Err("Duration must be an integer multiple of dt".into());
    }
    let mut flat = Flat {
        blocks: vec![],
        edges: vec![],
    };
    expand(model, "", 0, &mut flat, model.dt, solver)?;
    Ok(flat)
}
/// Parameter and edge checks for editor drafts; incomplete connections are allowed.
pub fn validate_draft(model: &GraphModel) -> Result<()> {
    structure(model).map(|_| ())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    Structure,
    EmptyGraph,
    DuplicateInput,
    UnconnectedInput,
    SignalWidth,
    MuxWidth,
    SelectorIndex,
    AlgebraicLoop,
    MissingScope,
    WorkLimit,
    RecordingLimit,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphDiagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub block_id: Option<String>,
    pub port: Option<usize>,
    pub source_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphValidation {
    pub runnable: bool,
    pub diagnostics: Vec<GraphDiagnostic>,
}
fn issue(
    code: DiagnosticCode,
    message: String,
    block_id: Option<&str>,
    port: Option<usize>,
    source_id: Option<&str>,
) -> GraphDiagnostic {
    GraphDiagnostic {
        code,
        message,
        block_id: block_id.map(str::to_owned),
        port,
        source_id: source_id.map(str::to_owned),
    }
}
struct Analysis {
    incoming: Vec<Vec<Option<usize>>>,
    order: Vec<usize>,
    scopes: Vec<usize>,
    diagnostics: Vec<GraphDiagnostic>,
}
fn analyze(flat: &Flat, model: &GraphModel) -> Analysis {
    use DiagnosticCode::*;
    let mut diagnostics = vec![];
    if flat.blocks.is_empty() {
        return Analysis {
            incoming: vec![],
            order: vec![],
            scopes: vec![],
            diagnostics: vec![issue(
                EmptyGraph,
                "Graph has no blocks".into(),
                None,
                None,
                None,
            )],
        };
    }
    let map: BTreeMap<_, _> = flat
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id.as_str(), i))
        .collect();
    let mut incoming: Vec<Vec<Option<usize>>> = flat
        .blocks
        .iter()
        .map(|b| vec![None; b.params.inputs()])
        .collect();
    for e in &flat.edges {
        let from = map[e.from.as_str()];
        let to = map[e.to.as_str()];
        if incoming[to][e.to_port].replace(from).is_some() {
            diagnostics.push(issue(
                DuplicateInput,
                format!("{}: duplicate input", e.to),
                Some(&e.to),
                Some(e.to_port),
                Some(&e.from),
            ));
        }
    }
    for (i, ports) in incoming.iter().enumerate() {
        let b = &flat.blocks[i];
        for (port, source) in ports.iter().enumerate() {
            if source.is_none() {
                diagnostics.push(issue(
                    UnconnectedInput,
                    format!("{}:{port}: unconnected input", b.id),
                    Some(&b.id),
                    Some(port),
                    None,
                ));
            }
        }
        match &b.params {
            BlockKind::Mux { .. } => {
                let total: usize = ports.iter().flatten().map(|s| flat.blocks[*s].width).sum();
                if total > b.width || (ports.iter().all(Option::is_some) && total != b.width) {
                    diagnostics.push(issue(
                        MuxWidth,
                        format!("{}: Mux output width must equal sum of input widths", b.id),
                        Some(&b.id),
                        None,
                        None,
                    ));
                }
            }
            BlockKind::Selector { indices } => {
                if let Some(source) = ports[0] {
                    if indices.iter().any(|v| *v >= flat.blocks[source].width) {
                        diagnostics.push(issue(
                            SelectorIndex,
                            format!("{}: Selector index outside source width", b.id),
                            Some(&b.id),
                            Some(0),
                            Some(&flat.blocks[source].id),
                        ));
                    }
                }
            }
            _ => {
                for (port, source) in ports.iter().enumerate() {
                    if let Some(source) = source {
                        if flat.blocks[*source].width != b.width {
                            diagnostics.push(issue(
                                SignalWidth,
                                format!("{}: signal width mismatch", b.id),
                                Some(&b.id),
                                Some(port),
                                Some(&flat.blocks[*source].id),
                            ));
                        }
                    }
                }
            }
        }
    }
    // Preserve the existing greedy execution order for valid graphs. Missing
    // connections do not block this analysis: they already have their own issues.
    let mut order = vec![];
    let mut done = BTreeSet::new();
    while order.len() < flat.blocks.len() {
        let mut progress = false;
        for (i, b) in flat.blocks.iter().enumerate() {
            if !done.contains(&i)
                && (b.params.state_output()
                    || incoming[i].iter().flatten().all(|s| done.contains(s)))
            {
                order.push(i);
                done.insert(i);
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    // Only actual cycle participants are blamed, not downstream Scope blocks.
    for (i, b) in flat.blocks.iter().enumerate() {
        if done.contains(&i) {
            continue;
        }
        let mut stack = vec![i];
        let mut seen = BTreeSet::new();
        let mut cycle = false;
        while let Some(node) = stack.pop() {
            if !seen.insert(node) || flat.blocks[node].params.state_output() {
                continue;
            }
            for source in incoming[node].iter().flatten() {
                if *source == i {
                    cycle = true;
                    break;
                }
                stack.push(*source);
            }
            if cycle {
                break;
            }
        }
        if cycle {
            diagnostics.push(issue(
                AlgebraicLoop,
                format!(
                    "Algebraic loop: {}. Insert Integrator, FirstOrder or Delay",
                    b.id
                ),
                Some(&b.id),
                None,
                None,
            ));
        }
    }
    let scopes: Vec<_> = flat
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| matches!(b.params, BlockKind::Scope {}))
        .map(|(i, _)| i)
        .collect();
    if scopes.is_empty() {
        diagnostics.push(issue(
            MissingScope,
            "Add a Scope to record results".into(),
            None,
            None,
            None,
        ));
    }
    let samples = (model.duration / model.dt).round() as usize + 1;
    let work: usize = flat.blocks.iter().map(|b| b.width).sum();
    if work * samples > 20_000_000 {
        diagnostics.push(issue(
            WorkLimit,
            "Graph exceeds 20000000 block-component steps; reduce graph size or time steps".into(),
            None,
            None,
            None,
        ));
    }
    let recorded_width: usize = scopes.iter().map(|i| flat.blocks[*i].width).sum();
    if recorded_width * samples > 2_000_000 {
        diagnostics.push(issue(
            RecordingLimit,
            "Recorded signals exceed 2000000 cells; reduce duration or Scope width".into(),
            None,
            None,
            None,
        ));
    }
    Analysis {
        incoming,
        order,
        scopes,
        diagnostics,
    }
}
/// Structured preflight checks; no simulation state or recorded samples are allocated.
pub fn diagnose_graph(model: &GraphModel) -> GraphValidation {
    let diagnostics = match structure(model) {
        Ok(flat) => analyze(&flat, model).diagnostics,
        Err(message) => vec![issue(DiagnosticCode::Structure, message, None, None, None)],
    };
    GraphValidation {
        runnable: diagnostics.is_empty(),
        diagnostics,
    }
}
fn compile(model: &GraphModel) -> Result<Compiled> {
    let flat = structure(model)?;
    let analysis = analyze(&flat, model);
    if let Some(first) = analysis.diagnostics.first() {
        return Err(first.message.clone());
    }
    let incoming = analysis
        .incoming
        .into_iter()
        .map(|ports| {
            ports
                .into_iter()
                .map(|p| p.expect("missing inputs are diagnostics"))
                .collect()
        })
        .collect();
    Ok(Compiled {
        flat,
        incoming,
        order: analysis.order,
        scopes: analysis.scopes,
    })
}

pub fn validate_graph(model: &GraphModel) -> Result<()> {
    compile(model).map(|_| ())
}

#[derive(Clone)]
struct Runtime {
    state: Vec<Vec<f64>>,
    sampled_state: Vec<Vec<f64>>,
    held: Vec<Vec<f64>>,
    pending: Vec<Option<Vec<f64>>>,
    discrete: Vec<Vec<DiscretePid>>,
    transfer: Vec<Option<transfer::Coefficients>>,
}
fn runtime(graph: &Compiled) -> Result<Runtime> {
    let mut rt = Runtime {
        state: vec![],
        sampled_state: vec![],
        held: vec![],
        pending: vec![],
        discrete: vec![],
        transfer: vec![],
    };
    for b in &graph.flat.blocks {
        let state = match &b.params {
            BlockKind::Integrator { initial } | BlockKind::FirstOrder { initial, .. } => {
                initial.clone()
            }
            BlockKind::TransferFunction { initial, .. } => initial.clone(),
            BlockKind::Pid {
                mode: PidMode::Continuous,
                ..
            } => vec![0.0; b.width * 2],
            _ => vec![],
        };
        rt.state.push(state);
        rt.sampled_state.push(
            if let BlockKind::DiscreteTransferFunction { initial, .. } = &b.params {
                initial.clone()
            } else {
                vec![]
            },
        );
        rt.transfer.push(
            if let BlockKind::TransferFunction {
                numerator,
                denominator,
                ..
            }
            | BlockKind::DiscreteTransferFunction {
                numerator,
                denominator,
                ..
            } = &b.params
            {
                Some(transfer::coefficients(numerator, denominator)?)
            } else {
                None
            },
        );
        rt.held
            .push(if let BlockKind::Delay { initial, .. } = &b.params {
                initial.clone()
            } else {
                vec![0.0; b.width]
            });
        rt.pending.push(None);
        rt.discrete.push(
            if let BlockKind::Pid {
                config,
                mode: PidMode::Discrete,
                ..
            } = b.params
            {
                (0..b.width)
                    .map(|_| DiscretePid::new(config))
                    .collect::<Result<Vec<_>>>()?
            } else {
                vec![]
            },
        );
    }
    Ok(rt)
}
fn evaluate(
    graph: &Compiled,
    rt: &mut Runtime,
    time: f64,
    tick: Option<usize>,
    dt: f64,
    left_limit: bool,
) -> Result<Vec<Vec<f64>>> {
    let mut values: Vec<Vec<f64>> = graph
        .flat
        .blocks
        .iter()
        .map(|b| vec![0.0; b.width])
        .collect();
    for &i in &graph.order {
        let b = &graph.flat.blocks[i];
        let inputs = &graph.incoming[i];
        let first = || values[inputs[0]].clone();
        let value = match &b.params {
            BlockKind::Constant { value } => value.clone(),
            BlockKind::SampledInput {
                values,
                sample_time,
            } => {
                if let Some(t) = tick {
                    let p = period(*sample_time, dt)?;
                    if t.is_multiple_of(p) {
                        rt.held[i] = values[(t / p).min(values.len() - 1)].clone();
                    }
                }
                rt.held[i].clone()
            }
            BlockKind::Step {
                before,
                after,
                step_time,
            } => {
                if time > *step_time || (!left_limit && time == *step_time) {
                    after.clone()
                } else {
                    before.clone()
                }
            }
            BlockKind::Sine {
                amplitude,
                frequency,
                phase,
                offset,
            } => vec![
                offset + amplitude * (std::f64::consts::TAU * frequency * time + phase).sin();
                b.width
            ],
            BlockKind::Gain { gain } => first().iter().map(|v| v * gain).collect(),
            BlockKind::Sum { signs } => (0..b.width)
                .map(|j| {
                    inputs
                        .iter()
                        .zip(signs)
                        .map(|(s, k)| values[*s][j] * k)
                        .sum()
                })
                .collect(),
            BlockKind::Product { .. } => (0..b.width)
                .map(|j| inputs.iter().map(|s| values[*s][j]).product())
                .collect(),
            BlockKind::Saturation { limits } => first().iter().map(|v| limits.clamp(*v)).collect(),
            BlockKind::Integrator { .. } | BlockKind::FirstOrder { .. } => rt.state[i].clone(),
            BlockKind::TransferFunction { .. } => {
                let coefficients = rt.transfer[i].as_ref().unwrap();
                (0..b.width)
                    .map(|c| {
                        coefficients.output(
                            &rt.state[i],
                            c,
                            if coefficients.direct == 0.0 {
                                0.0
                            } else {
                                values[inputs[0]][c]
                            },
                        )
                    })
                    .collect()
            }
            BlockKind::Delay { .. } => rt.held[i].clone(),
            BlockKind::DiscreteTransferFunction { sample_time, .. } => {
                if tick.is_some_and(|t| t % period(*sample_time, dt).unwrap() == 0) {
                    let coefficients = rt.transfer[i].as_ref().unwrap();
                    rt.held[i] = (0..b.width)
                        .map(|c| {
                            coefficients.output(
                                &rt.sampled_state[i],
                                c,
                                if coefficients.direct == 0.0 {
                                    0.0
                                } else {
                                    values[inputs[0]][c]
                                },
                            )
                        })
                        .collect();
                }
                rt.held[i].clone()
            }
            BlockKind::Pid {
                config,
                mode: PidMode::Continuous,
                ..
            } => first()
                .iter()
                .enumerate()
                .map(|(j, e)| {
                    let pid = ContinuousPid {
                        config: *config,
                        state: [rt.state[i][2 * j], rt.state[i][2 * j + 1]],
                    };
                    pid.output(*e)
                })
                .collect::<Result<Vec<_>>>()?,
            BlockKind::Pid {
                mode: PidMode::Discrete,
                sample_time,
                ..
            } => {
                if tick.is_some_and(|t| {
                    t % period(*sample_time, dt).expect("validated sample time") == 0
                }) {
                    let error = first();
                    let v = rt.discrete[i]
                        .iter_mut()
                        .zip(error)
                        .map(|(p, e)| p.step(e, *sample_time))
                        .collect::<Result<Vec<_>>>()?;
                    rt.held[i] = v;
                }
                rt.held[i].clone()
            }
            BlockKind::Mux { .. } => inputs.iter().flat_map(|s| values[*s].clone()).collect(),
            BlockKind::Selector { indices } => {
                indices.iter().map(|j| values[inputs[0]][*j]).collect()
            }
            BlockKind::Scope {} | BlockKind::Input { .. } | BlockKind::Output { .. } => first(),
            BlockKind::Subsystem { .. } => unreachable!(),
        };
        for v in &value {
            finite(&format!("{} at t={time}", b.id), *v)?;
        }
        values[i] = value;
    }
    Ok(values)
}
fn rates(
    graph: &Compiled,
    rt: &mut Runtime,
    time: f64,
    dt: f64,
    left_limit: bool,
) -> Result<Vec<Vec<f64>>> {
    let values = evaluate(graph, rt, time, None, dt, left_limit)?;
    graph
        .flat
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let input = || &values[graph.incoming[i][0]];
            let rate: Vec<f64> = match &b.params {
                BlockKind::Integrator { .. } => input().clone(),
                BlockKind::FirstOrder { gain, tau, .. } => input()
                    .iter()
                    .zip(&rt.state[i])
                    .map(|(u, y)| (gain * u - y) / tau)
                    .collect(),
                BlockKind::TransferFunction { .. } => {
                    let coefficients = rt.transfer[i].as_ref().unwrap();
                    coefficients.rates(&rt.state[i], input())
                }
                BlockKind::Pid {
                    config,
                    mode: PidMode::Continuous,
                    ..
                } => input()
                    .iter()
                    .enumerate()
                    .flat_map(|(j, e)| {
                        let pid = ContinuousPid {
                            config: *config,
                            state: [rt.state[i][2 * j], rt.state[i][2 * j + 1]],
                        };
                        pid.rate_at(*e, pid.state)
                    })
                    .collect(),
                _ => vec![],
            };
            for v in &rate {
                finite(&format!("{} derivative", b.id), *v)?;
            }
            Ok(rate)
        })
        .collect()
}
fn offset(base: &[Vec<f64>], rate: &[Vec<f64>], scale: f64) -> Vec<Vec<f64>> {
    base.iter()
        .zip(rate)
        .map(|(s, k)| s.iter().zip(k).map(|(s, k)| s + scale * k).collect())
        .collect()
}
fn integrate(graph: &Compiled, rt: &mut Runtime, time: f64, end: f64, dt: f64) -> Result<()> {
    let h = end - time;
    let base = rt.state.clone();
    let k1 = rates(graph, rt, time, dt, false)?;
    rt.state = offset(&base, &k1, h / 2.0);
    let k2 = rates(graph, rt, time + h / 2.0, dt, false)?;
    rt.state = offset(&base, &k2, h / 2.0);
    let k3 = rates(graph, rt, time + h / 2.0, dt, false)?;
    rt.state = offset(&base, &k3, h);
    // At a discontinuity, the left-side value belongs to the preceding interval.
    let k4 = rates(graph, rt, end, dt, true)?;
    rt.state = base
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.iter()
                .enumerate()
                .map(|(j, v)| v + h * (k1[i][j] + 2.0 * k2[i][j] + 2.0 * k3[i][j] + k4[i][j]) / 6.0)
                .collect()
        })
        .collect();
    for s in &rt.state {
        for v in s {
            finite("Integrated state", *v)?;
        }
    }
    Ok(())
}
/// Fixed recording clock, synchronous discrete updates and coupled RK4 or RK45.
pub fn simulate_graph(model: &GraphModel) -> Result<GraphResult> {
    simulate_graph_inner(model, None)
}
/// Shared evaluation allowance for batches; never changes the model or legacy JSON.
pub fn simulate_graph_with_budget(model: &GraphModel, limit: usize) -> Result<GraphResult> {
    simulate_graph_inner(model, Some(limit.min(20_000_000)))
}
fn simulate_graph_inner(model: &GraphModel, limit: Option<usize>) -> Result<GraphResult> {
    let graph = compile(model)?;
    if let Some(config @ SolverConfig::Rk45 { .. }) = &model.solver {
        return adaptive::simulate(&graph, model, config, limit.unwrap_or(20_000_000));
    }
    let width: usize = graph.flat.blocks.iter().map(|b| b.width).sum();
    let mut used = 0;
    let mut rt = runtime(&graph)?;
    let steps = (model.duration / model.dt).round() as usize;
    let mut result = GraphResult {
        times: vec![],
        traces: graph
            .scopes
            .iter()
            .map(|i| {
                let b = &graph.flat.blocks[*i];
                Trace {
                    id: b.id.clone(),
                    label: b.label.clone(),
                    values: vec![],
                }
            })
            .collect(),
        solver: "fixed-step hybrid RK4 / synchronous sample-and-hold".into(),
        statistics: model.solver.as_ref().map(|_| SolverStatistics {
            accepted_steps: 0,
            rejected_steps: 0,
            rate_evaluations: 0,
            output_evaluations: 0,
            min_step: 0.0,
            max_step: 0.0,
            max_error_ratio: 0.0,
        }),
    };
    let step_events: Vec<_> = graph
        .flat
        .blocks
        .iter()
        .filter_map(|b| {
            if let BlockKind::Step { step_time, .. } = b.params {
                Some(step_time)
            } else {
                None
            }
        })
        .collect();
    for tick in 0..=steps {
        let time = tick as f64 * model.dt;
        // Commit delayed samples before evaluating any direct-feedthrough blocks.
        sampled::commit(&graph, &mut rt, tick, model.dt)?;
        used += width;
        if limit.is_some_and(|cap| used > cap) {
            return Err("Batch exceeds remaining block-component evaluations".into());
        }
        let values = evaluate(&graph, &mut rt, time, Some(tick), model.dt, false)?;
        if let Some(stats) = &mut result.statistics {
            stats.output_evaluations += 1;
        }
        sampled::capture(&graph, &mut rt, &values, tick, model.dt)?;
        result.times.push(time);
        for (trace, i) in result.traces.iter_mut().zip(&graph.scopes) {
            trace.values.push(values[*i].clone());
        }
        if tick == steps {
            break;
        }
        // Resolve Step discontinuities exactly even when they fall between base ticks.
        let end = (tick + 1) as f64 * model.dt;
        let mut events: Vec<_> = step_events
            .iter()
            .copied()
            .filter(|t| *t > time && *t < end)
            .collect();
        events.push(end);
        events.sort_by(f64::total_cmp);
        events.dedup();
        let mut start = time;
        for event in events {
            used += 4 * width;
            if limit.is_some_and(|cap| used > cap) {
                return Err("Batch exceeds remaining block-component evaluations".into());
            }
            integrate(&graph, &mut rt, start, event, model.dt)?;
            if let Some(stats) = &mut result.statistics {
                let h = event - start;
                stats.accepted_steps += 1;
                stats.rate_evaluations += 4;
                stats.min_step = if stats.accepted_steps == 1 {
                    h
                } else {
                    stats.min_step.min(h)
                };
                stats.max_step = stats.max_step.max(h);
            }
            start = event;
        }
    }
    Ok(result)
}
