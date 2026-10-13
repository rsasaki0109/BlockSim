//! Selected multi-input/multi-output ancestor union; exact simulation unit bases.
use super::*;
use frequency::{FrequencyPoint, C};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub block: String,
    pub component: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MimoRequest {
    pub inputs: Vec<Channel>,
    pub outputs: Vec<Channel>,
    pub discrete: bool,
    pub min_omega: f64,
    pub max_omega: f64,
    pub points: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimoCheck {
    pub variables: usize,
    pub states: usize,
    pub inputs: usize,
    pub outputs: usize,
    pub sample_time: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimoResponse {
    pub input: usize,
    pub output: usize,
    pub points: Vec<FrequencyPoint>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimoResult {
    pub request: MimoRequest,
    pub check: MimoCheck,
    pub states: Vec<StateInfo>,
    pub a: Vec<Vec<f64>>,
    pub b: Vec<Vec<f64>>,
    pub c: Vec<Vec<f64>>,
    pub d: Vec<Vec<f64>>,
    pub poles: PolynomialRoots,
    pub matrix_residuals: Vec<f64>,
    pub stability: StateSpaceStability,
    pub structure: StateSpaceStructure,
    pub responses: Vec<MimoResponse>,
    pub method: String,
}
fn prepare_mimo(model: &GraphModel, q: &MimoRequest) -> Result<(Prepared, MimoCheck)> {
    for (name, channels) in [("inputs", &q.inputs), ("outputs", &q.outputs)] {
        if !(1..=8).contains(&channels.len()) {
            return Err(format!("MIMO {name} require 1..8 selected components"));
        }
        let unique: BTreeSet<_> = channels.iter().map(|s| (&s.block, s.component)).collect();
        if unique.len() != channels.len() {
            return Err(format!("MIMO {name} must not contain duplicate components"));
        }
    }
    if !(2..=512).contains(&q.points)
        || !(1e-9..=1e9).contains(&q.min_omega)
        || !q.max_omega.is_finite()
        || q.max_omega > 1e9
        || q.max_omega <= q.min_omega
        || (q.max_omega.ln() - q.min_omega.ln()) / (q.points - 1) as f64 <= 64. * f64::EPSILON
    {
        return Err(
            "MIMO requires 1e-9 <= min < max <= 1e9 rad/s and 2..512 resolved logarithmic points"
                .into(),
        );
    }
    let deps = if q.discrete {
        discrete::dependencies
    } else {
        frequency::dependencies
    };
    // Validate each input with the same shared flattening/component rules; collect each output once.
    let query = |input: &Channel, output: &Channel| frequency::FrequencyRequest {
        source: input.block.clone(),
        source_component: input.component,
        scope: output.block.clone(),
        scope_component: output.component,
        min_omega: 0.1,
        max_omega: 10.,
        points: 2,
        open_loop: false,
    };
    let mut source_nodes = vec![];
    let mut original = None;
    for input in &q.inputs {
        let (graph, _, _, source) =
            frequency::network_with_dependencies(model, &query(input, &q.outputs[0]), deps)?;
        source_nodes.push((source, input.component));
        original = Some(graph);
    }
    let mut nodes = vec![];
    let mut seen = BTreeSet::new();
    let mut output_nodes = vec![];
    for output in &q.outputs {
        let (_, selected, _, _) =
            frequency::network_with_dependencies(model, &query(&q.inputs[0], output), deps)?;
        output_nodes.push(selected[0]);
        for node in selected {
            if seen.insert(node) {
                nodes.push(node);
            }
        }
    }
    if nodes.len() > 128 {
        return Err("MIMO exceeds 128 union ancestor component variables".into());
    }
    let p = scalarize(
        original.unwrap(),
        nodes,
        &source_nodes,
        &output_nodes,
        "selected inputs".into(),
        "selected outputs".into(),
        q.discrete,
    )?;
    let sample_time = if q.discrete {
        Some(discrete::common_clock(&p, model.dt)?)
    } else {
        None
    };
    if sample_time.is_some_and(|ts| q.max_omega > std::f64::consts::PI / ts) {
        return Err("MIMO discrete maximum frequency exceeds Nyquist pi/Ts".into());
    }
    let work = p.states.len().max(1).pow(3) * q.points * q.inputs.len();
    if work > 20_000_000 {
        return Err("MIMO frequency solve budget exceeds 20000000".into());
    }
    let check = MimoCheck {
        variables: p.check.variables,
        states: p.states.len(),
        inputs: q.inputs.len(),
        outputs: q.outputs.len(),
        sample_time,
    };
    Ok((p, check))
}
pub fn validate_mimo(model: &GraphModel, q: &MimoRequest) -> Result<MimoCheck> {
    prepare_mimo(model, q).map(|(_, check)| check)
}
pub fn analyze_mimo(model: &GraphModel, q: &MimoRequest) -> Result<MimoResult> {
    let (mut p, check) = prepare_mimo(model, q)?;
    let n = p.states.len();
    let m = q.inputs.len();
    let o = q.outputs.len();
    let base = runtime(&p.graph)?;
    let mut a = vec![vec![0.; n]; n];
    let mut b = vec![vec![0.; m]; n];
    let mut c = vec![vec![0.; n]; o];
    let mut d = vec![vec![0.; m]; o];
    for column in 0..n + m {
        for (j, input) in p.inputs.iter().enumerate() {
            if let Some(input) = input {
                if let BlockKind::Constant { value } = &mut p.graph.flat.blocks[*input].params {
                    value[0] = if column == n + j { 1. } else { 0. };
                }
            }
        }
        let mut rt = base.clone();
        if column < n {
            let (i, s) = p.offsets[column];
            if !q.discrete {
                rt.state[i][s] = 1.;
            } else {
                match p.graph.flat.blocks[i].params {
                    BlockKind::Delay { .. } => rt.held[i][0] = 1.,
                    BlockKind::DiscreteTransferFunction { .. } => rt.sampled_state[i][s] = 1.,
                    BlockKind::Pid { .. } => {
                        let mut v = [0.; 3];
                        v[s] = 1.;
                        rt.discrete[i][0].set_analysis_state(v);
                    }
                    _ => unreachable!(),
                }
            }
        }
        let values = evaluate(
            &p.graph,
            &mut rt,
            0.,
            q.discrete.then_some(0),
            model.dt,
            false,
        )?;
        let derivatives = if !q.discrete {
            Some(rates(&p.graph, &mut rt, 0., model.dt, false)?)
        } else {
            None
        };
        for (row, &(i, s)) in p.offsets.iter().enumerate() {
            let v = if let Some(ref derivatives) = derivatives {
                derivatives[i][s]
            } else {
                match p.graph.flat.blocks[i].params {
                    BlockKind::Delay { .. } => values[p.graph.incoming[i][0]][0],
                    BlockKind::DiscreteTransferFunction { .. } => {
                        sampled::next_transfer(&p.graph, &rt, &values, i)?[s]
                    }
                    BlockKind::Pid { .. } => rt.discrete[i][0].analysis_state()[s],
                    _ => unreachable!(),
                }
            };
            finite("MIMO next state", v)?;
            if column < n {
                a[row][column] = v;
            } else {
                b[row][column - n] = v;
            }
        }
        for (row, &output) in p.outputs.iter().enumerate() {
            if column < n {
                c[row][column] = values[output][0];
            } else {
                d[row][column - n] = values[output][0];
            }
        }
    }
    let (characteristic, scale, _, poles, matrix_residuals, continuous_stability) =
        spectrum::analyze(&a);
    let mut stability = if q.discrete {
        discrete::unit_circle::classify(characteristic.as_deref(), scale)
    } else {
        continuous_stability.into()
    };
    stability
        .notes
        .retain(|s| !s.contains("single-channel realization"));
    stability.notes.push("Internal modes of the union of selected output ancestors; all states retained; not whole-model stability".into());
    let structure = structure::analyze_multi(&a, &b, &c);
    let responses = frequency_response(q, &check, &a, &b, &c, &d);
    Ok(MimoResult{request:q.clone(),check,states:p.states,a,b,c,d,poles,matrix_residuals,stability,structure,responses,method:"MIMO union ancestors / exact simulation unit basis / zero other sources and initial conditions / no state removal / H=C(sI-A)^-1B+D or C(zI-A)^-1B+D".into()})
}
fn frequency_response(
    q: &MimoRequest,
    check: &MimoCheck,
    a: &[Vec<f64>],
    b: &[Vec<f64>],
    c: &[Vec<f64>],
    d: &[Vec<f64>],
) -> Vec<MimoResponse> {
    let n = a.len();
    let mut result: Vec<_> = (0..q.outputs.len())
        .flat_map(|output| {
            (0..q.inputs.len()).map(move |input| MimoResponse {
                input,
                output,
                points: vec![],
            })
        })
        .collect();
    for k in 0..q.points {
        let omega = if k == 0 {
            q.min_omega
        } else if k + 1 == q.points {
            q.max_omega
        } else {
            (q.min_omega.ln()
                + (q.max_omega.ln() - q.min_omega.ln()) * k as f64 / (q.points - 1) as f64)
                .exp()
        };
        let z = if let Some(ts) = check.sample_time {
            C {
                re: (omega * ts).cos(),
                im: (omega * ts).sin(),
            }
        } else {
            C { re: 0., im: omega }
        };
        for input in 0..q.inputs.len() {
            let solved = if n == 0 {
                Ok((vec![], 0.))
            } else {
                frequency::solve(
                    (0..n)
                        .map(|i| {
                            let mut row: Vec<_> = (0..n)
                                .map(|j| {
                                    if i == j {
                                        z - C::real(a[i][j])
                                    } else {
                                        C::real(-a[i][j])
                                    }
                                })
                                .collect();
                            row.push(C::real(b[i][input]));
                            row
                        })
                        .collect(),
                )
            };
            for output in 0..q.outputs.len() {
                let response = solved
                    .as_ref()
                    .map_err(Clone::clone)
                    .and_then(|(x, residual)| {
                        let h = c[output]
                            .iter()
                            .zip(x)
                            .try_fold(C::real(d[output][input]), |sum, (c, x)| {
                                (sum + C::real(*c) * *x).checked()
                            })?;
                        let magnitude = h.abs();
                        finite("MIMO magnitude", magnitude)?;
                        Ok(FrequencyPoint {
                            omega,
                            real: Some(h.re),
                            imag: Some(h.im),
                            magnitude: Some(magnitude),
                            magnitude_db: if magnitude > 0. {
                                Some(20. * magnitude.log10())
                            } else {
                                None
                            },
                            phase_deg: if magnitude > 0. {
                                Some(h.im.atan2(h.re).to_degrees())
                            } else {
                                None
                            },
                            relative_residual: Some(*residual),
                            error: None,
                        })
                    });
                result[output * q.inputs.len() + input]
                    .points
                    .push(response.unwrap_or_else(|error| FrequencyPoint {
                        omega,
                        real: None,
                        imag: None,
                        magnitude: None,
                        magnitude_db: None,
                        phase_deg: None,
                        relative_residual: None,
                        error: Some(error),
                    }));
            }
        }
    }
    result
}
