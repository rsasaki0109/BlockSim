//! Continuous LTI response from exact component equations on s=jω.
//! This does not linearize nonlinear models or infer closed-loop stability.
use super::*;
use std::ops::{Add, Mul, Sub};
mod discrete;
mod margins;
pub use discrete::{
    analyze_discrete_frequency, validate_discrete_frequency, DiscreteFrequencyCheck,
    DiscreteFrequencyResult,
};
pub use margins::{GainCrossing, MarginAnalysis, MarginIssue, PhaseCrossing};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrequencyRequest {
    pub source: String,
    pub source_component: usize,
    pub scope: String,
    pub scope_component: usize,
    pub min_omega: f64,
    pub max_omega: f64,
    pub points: usize,
    /// Caller explicitly interprets the selected transfer as an open-loop L(s).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub open_loop: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrequencyCheck {
    pub source_label: String,
    pub scope_label: String,
    pub variables: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrequencyPoint {
    pub omega: f64,
    pub real: Option<f64>,
    pub imag: Option<f64>,
    pub magnitude: Option<f64>,
    pub magnitude_db: Option<f64>,
    pub phase_deg: Option<f64>,
    pub relative_residual: Option<f64>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrequencyResult {
    pub request: FrequencyRequest,
    pub check: FrequencyCheck,
    pub points: Vec<FrequencyPoint>,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margins: Option<MarginAnalysis>,
}
#[derive(Clone, Copy, Debug, Default)]
struct C {
    re: f64,
    im: f64,
}
impl C {
    const ONE: Self = Self { re: 1.0, im: 0.0 };
    fn real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    fn checked(self) -> Result<Self> {
        finite("Frequency real value", self.re)?;
        finite("Frequency imaginary value", self.im)?;
        Ok(self)
    }
    fn div(self, b: Self) -> Result<Self> {
        let scale = b.re.abs().max(b.im.abs());
        if scale == 0.0 {
            return Err("Frequency equation has a zero denominator".into());
        }
        let re = b.re / scale;
        let im = b.im / scale;
        let denominator = re * re + im * im;
        let ar = self.re / scale;
        let ai = self.im / scale;
        Self {
            re: (ar * re + ai * im) / denominator,
            im: (ai * re - ar * im) / denominator,
        }
        .checked()
    }
}
impl Add for C {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self {
            re: self.re + b.re,
            im: self.im + b.im,
        }
    }
}
impl Sub for C {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self {
            re: self.re - b.re,
            im: self.im - b.im,
        }
    }
}
impl Mul for C {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self {
            re: self.re * b.re - self.im * b.im,
            im: self.re * b.im + self.im * b.re,
        }
    }
}
type Node = (usize, usize);

pub(super) fn dependencies(graph: &Compiled, node: Node) -> Result<Vec<Node>> {
    let (i, c) = node;
    let b = &graph.flat.blocks[i];
    let incoming = &graph.incoming[i];
    let one = || vec![(incoming[0], c)];
    let unsupported = || {
        format!("{}: frequency analysis requires continuous linear blocks; bounded PID/Saturation, Product and sampled blocks are unsupported",b.id)
    };
    Ok(match &b.params {
        BlockKind::Constant { .. } | BlockKind::Step { .. } | BlockKind::Sine { .. } => vec![],
        BlockKind::Sum { .. } => incoming.iter().map(|i| (*i, c)).collect(),
        BlockKind::Mux { .. } => {
            let mut component = c;
            let mut selected = None;
            for &source in incoming {
                let width = graph.flat.blocks[source].width;
                if component < width {
                    selected = Some((source, component));
                    break;
                }
                component -= width;
            }
            vec![selected.ok_or("Mux frequency component is out of range")?]
        }
        BlockKind::Selector { indices } => vec![(incoming[0], indices[c])],
        BlockKind::Pid {
            config,
            mode: PidMode::Continuous,
            ..
        } if config.limits.lower.is_none() && config.limits.upper.is_none() => one(),
        BlockKind::Saturation { limits } if limits.lower.is_none() && limits.upper.is_none() => {
            one()
        }
        BlockKind::Product { inputs: 1 } => one(),
        BlockKind::Gain { .. }
        | BlockKind::FirstOrder { .. }
        | BlockKind::Integrator { .. }
        | BlockKind::TransferFunction { .. }
        | BlockKind::Scope {}
        | BlockKind::Input { .. }
        | BlockKind::Output { .. } => one(),
        _ => return Err(unsupported()),
    })
}
pub(super) fn network(
    model: &GraphModel,
    request: &FrequencyRequest,
) -> Result<(Compiled, Vec<Node>, FrequencyCheck, usize)> {
    network_with_dependencies(model, request, dependencies)
}
pub(super) fn network_with_dependencies(
    model: &GraphModel,
    request: &FrequencyRequest,
    dependencies: fn(&Compiled, Node) -> Result<Vec<Node>>,
) -> Result<(Compiled, Vec<Node>, FrequencyCheck, usize)> {
    if !(1e-9..=1e9).contains(&request.min_omega)
        || !request.max_omega.is_finite()
        || request.max_omega > 1e9
        || request.max_omega <= request.min_omega
        || !(2..=2048).contains(&request.points)
    {
        return Err(
            "Frequency range requires 1e-9 <= min < max <= 1e9 rad/s and 2..2048 points".into(),
        );
    }
    let log_step = (request.max_omega.ln() - request.min_omega.ln()) / (request.points - 1) as f64;
    if log_step <= 64.0 * f64::EPSILON {
        return Err(
            "Frequency logarithmic grid is too narrow for floating-point resolution".into(),
        );
    }
    let graph = compile(model)?;
    let source = graph
        .flat
        .blocks
        .iter()
        .position(|b| b.id == request.source)
        .ok_or("Frequency source does not exist")?;
    let scope = graph
        .flat
        .blocks
        .iter()
        .position(|b| b.id == request.scope && matches!(b.params, BlockKind::Scope {}))
        .ok_or("Frequency Scope does not exist")?;
    let source_block = &graph.flat.blocks[source];
    if !matches!(
        source_block.params,
        BlockKind::Constant { .. } | BlockKind::Step { .. } | BlockKind::Sine { .. }
    ) || request.source_component >= source_block.width
        || request.scope_component >= graph.flat.blocks[scope].width
    {
        return Err(
            "Frequency input must be a source component; source/Scope components must be in range"
                .into(),
        );
    }
    fn visit(
        graph: &Compiled,
        node: Node,
        nodes: &mut Vec<Node>,
        seen: &mut BTreeSet<Node>,
        dependencies: fn(&Compiled, Node) -> Result<Vec<Node>>,
    ) -> Result<()> {
        if !seen.insert(node) {
            return Ok(());
        }
        if nodes.len() >= 128 {
            return Err("Frequency response exceeds 128 relevant component variables".into());
        }
        nodes.push(node);
        for dependency in dependencies(graph, node)? {
            visit(graph, dependency, nodes, seen, dependencies)?;
        }
        Ok(())
    }
    let mut nodes = vec![];
    visit(
        &graph,
        (scope, request.scope_component),
        &mut nodes,
        &mut BTreeSet::new(),
        dependencies,
    )?;
    if nodes.len().pow(3) > 20_000_000 / request.points {
        return Err(
            "Frequency dense-solve budget exceeds 20000000; reduce points or simplify the model"
                .into(),
        );
    }
    let check = FrequencyCheck {
        source_label: source_block.label.clone(),
        scope_label: graph.flat.blocks[scope].label.clone(),
        variables: nodes.len(),
    };
    Ok((graph, nodes, check, source))
}
pub fn validate_frequency(
    model: &GraphModel,
    request: &FrequencyRequest,
) -> Result<FrequencyCheck> {
    network(model, request).map(|(_, _, check, _)| check)
}
fn polynomial(coefficients: &[f64], s: C, scale: f64) -> Result<C> {
    coefficients.iter().try_fold(C::default(), |acc, value| {
        (acc * s + C::real(*value / scale)).checked()
    })
}
fn weights(kind: &BlockKind, s: C) -> Result<Vec<C>> {
    Ok(match kind {
        BlockKind::Constant { .. } | BlockKind::Step { .. } | BlockKind::Sine { .. } => vec![],
        BlockKind::Sum { signs } => signs.iter().map(|v| C::real(*v)).collect(),
        BlockKind::Gain { gain } => vec![C::real(*gain)],
        BlockKind::FirstOrder { gain, tau, .. } => {
            vec![C::real(*gain).div(C::ONE + C::real(*tau) * s)?]
        }
        BlockKind::Integrator { .. } => vec![C::ONE.div(s)?],
        BlockKind::TransferFunction {
            numerator,
            denominator,
            ..
        } => vec![polynomial(numerator, s, denominator[0])?.div(polynomial(
            denominator,
            s,
            denominator[0],
        )?)?],
        BlockKind::Pid { config, .. } => vec![(C::real(config.kp)
            + C::real(config.ki).div(s)?
            + (C::real(config.kd) * s).div(C::ONE + C::real(config.derivative_tau) * s)?)
        .checked()?],
        _ => vec![C::ONE],
    })
}
fn solve(mut matrix: Vec<Vec<C>>) -> Result<(Vec<C>, f64)> {
    let n = matrix.len();
    for row in &mut matrix {
        let scale = row[..n].iter().map(|v| v.abs()).fold(0.0, f64::max);
        if !scale.is_finite() || scale == 0.0 {
            return Err("Frequency matrix has invalid row scaling".into());
        }
        for value in row {
            *value = value.div(C::real(scale))?;
        }
    }
    let original = matrix.clone();
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|a, b| {
                matrix[*a][column]
                    .abs()
                    .total_cmp(&matrix[*b][column].abs())
            })
            .unwrap();
        if matrix[pivot][column].abs() < 1e-12 {
            return Err("Frequency matrix is singular or ill-conditioned".into());
        }
        matrix.swap(column, pivot);
        for row in column + 1..n {
            let factor = matrix[row][column].div(matrix[column][column])?;
            matrix[row][column] = C::default();
            for j in column + 1..=n {
                matrix[row][j] = (matrix[row][j] - factor * matrix[column][j]).checked()?;
            }
        }
    }
    let mut values = vec![C::default(); n];
    for i in (0..n).rev() {
        let rhs = (i + 1..n).try_fold(matrix[i][n], |rhs, j| {
            (rhs - matrix[i][j] * values[j]).checked()
        })?;
        values[i] = rhs.div(matrix[i][i])?;
    }
    let scale = values.iter().map(|v| v.abs()).fold(1.0, f64::max);
    if !scale.is_finite() {
        return Err("Frequency response magnitude is outside finite range".into());
    }
    let normalized: Vec<_> = values
        .iter()
        .map(|v| C {
            re: v.re / scale,
            im: v.im / scale,
        })
        .collect();
    let mut residual: f64 = 0.0;
    for row in original {
        let rhs = row[n].div(C::real(scale))?;
        let sum = row[..n]
            .iter()
            .zip(&normalized)
            .try_fold(C::default(), |sum, (a, x)| (sum + *a * *x).checked())?;
        let norm = row[..n].iter().map(|v| v.abs()).sum::<f64>() + rhs.abs();
        residual = residual.max((sum - rhs).abs() / norm.max(f64::MIN_POSITIVE));
    }
    if !residual.is_finite() || residual > 1e-10 {
        return Err("Frequency matrix residual exceeds numerical tolerance".into());
    }
    Ok((values, residual))
}
pub fn analyze_frequency(
    model: &GraphModel,
    request: &FrequencyRequest,
) -> Result<FrequencyResult> {
    let (graph, nodes, check, source) = network(model, request)?;
    let indices: BTreeMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (*node, i))
        .collect();
    let evaluate = |omega: f64| {
        let n = nodes.len();
        let mut matrix = vec![vec![C::default(); n + 1]; n];
        let s = C { re: 0.0, im: omega };
        for (row, &node) in nodes.iter().enumerate() {
            matrix[row][row] = C::ONE;
            let dependencies = dependencies(&graph, node)?;
            for (dependency, weight) in dependencies
                .iter()
                .zip(weights(&graph.flat.blocks[node.0].params, s)?)
            {
                let col = indices[dependency];
                matrix[row][col] = (matrix[row][col] - weight).checked()?;
            }
            if node == (source, request.source_component) {
                matrix[row][n] = C::ONE;
            }
        }
        let (values, residual) = solve(matrix)?;
        Ok::<_, String>((values[0], residual))
    };
    let points = sample_points(request, &evaluate);
    let margins = request.open_loop.then(|| {
        margins::analyze(
            &points,
            (20_000_000 / nodes.len().pow(3) - request.points).min(4096),
            evaluate,
        )
    });
    Ok(FrequencyResult {request:request.clone(),check,points,margins,method:"continuous LTI complex component equations / scaled partial-pivot solve / sampled phase unwrap".into()})
}

fn sample_points(
    request: &FrequencyRequest,
    evaluate: &impl Fn(f64) -> Result<(C, f64)>,
) -> Vec<FrequencyPoint> {
    let mut points = Vec::with_capacity(request.points);
    let mut previous_phase: Option<f64> = None;
    for index in 0..request.points {
        let omega = if index == 0 {
            request.min_omega
        } else if index + 1 == request.points {
            request.max_omega
        } else {
            (request.min_omega.ln()
                + (request.max_omega.ln() - request.min_omega.ln()) * index as f64
                    / (request.points - 1) as f64)
                .exp()
        };
        let response = evaluate(omega);
        match response {
            Ok((value, residual)) => {
                let magnitude = value.abs();
                let (db, phase) = if magnitude == 0.0 {
                    previous_phase = None;
                    (None, None)
                } else {
                    let mut phase = value.im.atan2(value.re).to_degrees();
                    if let Some(previous) = previous_phase {
                        phase += 360.0 * ((previous - phase) / 360.0).round();
                    }
                    previous_phase = Some(phase);
                    (Some(20.0 * magnitude.log10()), Some(phase))
                };
                points.push(FrequencyPoint {
                    omega,
                    real: Some(value.re),
                    imag: Some(value.im),
                    magnitude: Some(magnitude),
                    magnitude_db: db,
                    phase_deg: phase,
                    relative_residual: Some(residual),
                    error: None,
                });
            }
            Err(error) => {
                previous_phase = None;
                points.push(FrequencyPoint {
                    omega,
                    real: None,
                    imag: None,
                    magnitude: None,
                    magnitude_db: None,
                    phase_deg: None,
                    relative_residual: None,
                    error: Some(error),
                });
            }
        }
    }
    points
}
