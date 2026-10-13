//! Exact linear unit-basis extraction of the selected Scope ancestor realization.
//! No finite differencing, cancellation, minimal realization or whole-model claim.
use super::*;
use pole_zero::{PolynomialRoots, Stability};
pub(super) mod discrete;
mod spectrum;
pub use discrete::{analyze_discrete_state_space, validate_discrete_state_space};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateSpaceRequest {
    pub source: String,
    pub source_component: usize,
    pub scope: String,
    pub scope_component: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInfo {
    pub block: String,
    pub label: String,
    pub component: usize,
    pub state: usize,
    pub description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSpaceCheck {
    pub source_label: String,
    pub scope_label: String,
    pub variables: usize,
    pub states: usize,
    pub source_related: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSpaceStability {
    pub classification: String,
    pub right_half_plane_poles: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside_unit_circle_poles: Option<usize>,
    pub tolerance: f64,
    pub method: String,
    pub notes: Vec<String>,
}
impl From<Stability> for StateSpaceStability {
    fn from(s: Stability) -> Self {
        Self {
            classification: s.classification,
            right_half_plane_poles: s.right_half_plane_poles,
            outside_unit_circle_poles: None,
            tolerance: s.tolerance,
            method: s.method,
            notes: s.notes,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSpaceResult {
    pub request: StateSpaceRequest,
    pub check: StateSpaceCheck,
    pub states: Vec<StateInfo>,
    pub a: Vec<Vec<f64>>,
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    pub d: f64,
    /// det(zI-A/pole_scale). Positive variable scaling preserves RHP counts.
    pub characteristic: Option<Vec<f64>>,
    pub pole_scale: f64,
    pub characteristic_residual: Option<f64>,
    pub poles: PolynomialRoots,
    /// Independent row-scaled determinant residuals, aligned with physical poles.
    pub matrix_residuals: Vec<f64>,
    pub stability: StateSpaceStability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_time: Option<f64>,
    pub method: String,
}
struct Prepared {
    graph: Compiled,
    input: Option<usize>,
    offsets: Vec<(usize, usize)>,
    states: Vec<StateInfo>,
    check: StateSpaceCheck,
}
fn prepare(model: &GraphModel, q: &StateSpaceRequest) -> Result<Prepared> {
    prepare_with_domain(model, q, false)
}
fn prepare_with_domain(
    model: &GraphModel,
    q: &StateSpaceRequest,
    discrete: bool,
) -> Result<Prepared> {
    // Shared exact component mapping; each domain supplies its supported dependencies.
    let request = frequency::FrequencyRequest {
        source: q.source.clone(),
        source_component: q.source_component,
        scope: q.scope.clone(),
        scope_component: q.scope_component,
        min_omega: 0.1,
        max_omega: 10.0,
        points: 2,
        open_loop: false,
    };
    let (original, nodes, check, source) = frequency::network_with_dependencies(
        model,
        &request,
        if discrete {
            discrete::dependencies
        } else {
            frequency::dependencies
        },
    )
    .map_err(|e| {
        e.replace("Frequency", "State-space")
            .replace("frequency analysis", "state-space analysis")
    })?;
    let indices: BTreeMap<_, _> = nodes.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let mut blocks = vec![];
    let mut incoming = vec![];
    let mut offsets = vec![];
    let mut states = vec![];
    for (i, &node) in nodes.iter().enumerate() {
        let original_block = &original.flat.blocks[node.0];
        let mut b = original_block.clone();
        b.id = format!("{}[{}]", original_block.id, node.1);
        b.width = 1;
        let count = match &b.params {
            BlockKind::Integrator { .. } | BlockKind::FirstOrder { .. } => 1,
            BlockKind::TransferFunction { denominator, .. }
            | BlockKind::DiscreteTransferFunction { denominator, .. } => denominator.len() - 1,
            BlockKind::Pid { .. } => {
                if discrete {
                    3
                } else {
                    2
                }
            }
            BlockKind::Delay { .. } => 1,
            _ => 0,
        };
        for state in 0..count {
            if states.len() >= 8 {
                return Err(format!("State-space analysis supports at most 8 related {} states; no states are silently removed", if discrete { "discrete" } else { "continuous" }));
            }
            let description = match b.params {
                BlockKind::Pid { .. } if discrete => match state {
                    0 => "weighted PID integral",
                    1 => "previous PID error",
                    _ => "PID derivative filter",
                },
                BlockKind::Delay { .. } => "previous sampled input",
                BlockKind::DiscreteTransferFunction { .. } => "discrete transfer companion state",
                BlockKind::Pid { .. } => {
                    if state == 0 {
                        "weighted PID integral"
                    } else {
                        "PID error filter"
                    }
                }
                BlockKind::Integrator { .. } => "integrated input",
                BlockKind::FirstOrder { .. } => "first-order output",
                _ => "transfer companion state",
            };
            states.push(StateInfo {
                block: original_block.id.clone(),
                label: original_block.label.clone(),
                component: node.1,
                state,
                description: description.into(),
            });
            offsets.push((i, state));
        }
        b.params = match b.params {
            BlockKind::Constant { .. }
            | BlockKind::SampledInput { .. }
            | BlockKind::Step { .. }
            | BlockKind::Sine { .. } => BlockKind::Constant { value: vec![0.0] },
            BlockKind::Integrator { .. } => BlockKind::Integrator { initial: vec![0.0] },
            BlockKind::FirstOrder { gain, tau, .. } => BlockKind::FirstOrder {
                gain,
                tau,
                initial: vec![0.0],
            },
            BlockKind::TransferFunction {
                numerator,
                denominator,
                ..
            } => BlockKind::TransferFunction {
                numerator,
                denominator,
                initial: vec![0.0; count],
            },
            BlockKind::DiscreteTransferFunction {
                numerator,
                denominator,
                sample_time,
                ..
            } => BlockKind::DiscreteTransferFunction {
                numerator,
                denominator,
                sample_time,
                initial: vec![0.0; count],
            },
            BlockKind::Delay { sample_time, .. } => BlockKind::Delay {
                sample_time,
                initial: vec![0.0],
            },
            BlockKind::Mux { .. }
            | BlockKind::Selector { .. }
            | BlockKind::Input { .. }
            | BlockKind::Output { .. }
            | BlockKind::Saturation { .. }
            | BlockKind::Product { inputs: 1 } => BlockKind::Gain { gain: 1.0 },
            other => other,
        };
        incoming.push(
            (if discrete {
                discrete::dependencies
            } else {
                frequency::dependencies
            })(&original, node)?
            .iter()
            .map(|n| indices[n])
            .collect(),
        );
        blocks.push(b);
    }
    let order = original
        .order
        .iter()
        .flat_map(|b| {
            nodes
                .iter()
                .enumerate()
                .filter_map(move |(i, n)| (n.0 == *b).then_some(i))
        })
        .collect();
    let input = indices.get(&(source, q.source_component)).copied();
    let check = StateSpaceCheck {
        source_label: check.source_label,
        scope_label: check.scope_label,
        variables: nodes.len(),
        states: states.len(),
        source_related: input.is_some(),
    };
    Ok(Prepared {
        graph: Compiled {
            flat: Flat {
                blocks,
                edges: vec![],
            },
            incoming,
            order,
            scopes: vec![0],
        },
        input,
        offsets,
        states,
        check,
    })
}
pub fn validate_state_space(model: &GraphModel, q: &StateSpaceRequest) -> Result<StateSpaceCheck> {
    prepare(model, q).map(|p| p.check)
}
pub fn analyze_state_space(model: &GraphModel, q: &StateSpaceRequest) -> Result<StateSpaceResult> {
    let mut p = prepare(model, q)?;
    let n = p.states.len();
    let base = runtime(&p.graph)?;
    let mut a = vec![vec![0.0; n]; n];
    let mut b = vec![0.0; n];
    let mut c = vec![0.0; n];
    let mut d = 0.0;
    for (column, output_coefficient) in c.iter_mut().chain(std::iter::once(&mut d)).enumerate() {
        if let Some(input) = p.input {
            if let BlockKind::Constant { value } = &mut p.graph.flat.blocks[input].params {
                value[0] = if column == n { 1.0 } else { 0.0 };
            }
        }
        let mut rt = base.clone();
        if column < n {
            let (block, index) = p.offsets[column];
            rt.state[block][index] = 1.0;
        }
        let values = evaluate(&p.graph, &mut rt, 0.0, None, model.dt, false)?;
        let derivatives = rates(&p.graph, &mut rt, 0.0, model.dt, false)?;
        for (row, &(block, index)) in p.offsets.iter().enumerate() {
            if column < n {
                a[row][column] = derivatives[block][index];
            } else {
                b[row] = derivatives[block][index];
            }
        }
        *output_coefficient = values[0][0];
    }
    let (
        characteristic,
        pole_scale,
        characteristic_residual,
        poles,
        matrix_residuals,
        mut stability,
    ) = spectrum::analyze(&a);
    // Do not inherit the single-TransferFunction scope wording from the shared Routh routine.
    stability
        .notes
        .retain(|s| !s.contains("single-channel realization"));
    stability.notes.push("Numerical internal stability of the selected Scope ancestor realization; unrelated branches are excluded; no cancellation or minimal realization".into());
    if !p.check.source_related {
        stability.notes.push("Selected source component is outside the Scope ancestry: B and D are zero; ancestor internal modes are still retained".into());
    }
    Ok(StateSpaceResult{request:q.clone(),check:p.check,states:p.states,a,b,c,d,characteristic,pole_scale,characteristic_residual,poles,matrix_residuals,stability:stability.into(),sample_time:None,method:"continuous LTI Scope ancestry / simulation-equation unit basis / xdot=Ax+Bu,y=Cx+Du / scaled characteristic polynomial and independent matrix determinant residuals / Routh / no state removal".into()})
}
