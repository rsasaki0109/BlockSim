//! StateSpace blocks lower to existing exact hybrid primitives, never serialized expansion.
use super::*;
pub(super) fn validate(block: &Block, dt: f64) -> Result<()> {
    let BlockKind::StateSpace {
        a,
        b,
        c,
        d,
        initial,
        mode,
        sample_time,
    } = &block.params
    else {
        unreachable!()
    };
    let n = a.len();
    let p = c.len();
    let m = d.first().map_or(0, Vec::len);
    if n > 16
        || !(1..=8).contains(&m)
        || !(1..=8).contains(&p)
        || p != block.width
        || a.iter().any(|r| r.len() != n)
        || b.len() != n
        || b.iter().any(|r| r.len() != m)
        || c.iter().any(|r| r.len() != n)
        || d.len() != p
        || d.iter().any(|r| r.len() != m)
        || initial.len() != n
    {
        return Err("StateSpace requires A n×n, B n×m, C p×n, D p×m, initial n; n=0..16, m/p=1..8, output width=p".into());
    }
    for v in a
        .iter()
        .flatten()
        .chain(b.iter().flatten())
        .chain(c.iter().flatten())
        .chain(d.iter().flatten())
        .chain(initial)
    {
        finite("StateSpace coefficient/initial", *v)?;
    }
    positive("StateSpace sample time", *sample_time)?;
    if *mode == PidMode::Discrete {
        period(*sample_time, dt)?;
    }
    Ok(())
}
pub(super) fn lower(block: &Block, dt: f64, duration: f64) -> Result<GraphModel> {
    validate(block, dt)?;
    let BlockKind::StateSpace {
        a,
        b,
        c,
        d,
        initial,
        mode,
        sample_time,
    } = &block.params
    else {
        unreachable!()
    };
    let n = a.len();
    let m = d[0].len();
    let p = c.len();
    let mut g = GraphModel {
        version: 1,
        dt,
        duration,
        solver: None,
        blocks: vec![],
        edges: vec![],
    };
    let add = |g: &mut GraphModel, id: String, width, params| {
        g.blocks.push(Block {
            id,
            label: block.label.clone(),
            width,
            position: [0., 0.],
            params,
        });
    };
    let edge = |g: &mut GraphModel, from: String, to: String, port| {
        g.edges.push(Edge {
            from,
            from_port: 0,
            to,
            to_port: port,
        })
    };
    add(&mut g, "input".into(), m, BlockKind::Input { port: 0 });
    for j in 0..m {
        let id = format!("input-{j}");
        add(
            &mut g,
            id.clone(),
            1,
            BlockKind::Selector { indices: vec![j] },
        );
        edge(&mut g, "input".into(), id, 0);
    }
    for i in 0..n {
        let id = format!("state-{i}");
        add(
            &mut g,
            id.clone(),
            1,
            if *mode == PidMode::Continuous {
                BlockKind::Integrator {
                    initial: vec![initial[i]],
                }
            } else {
                BlockKind::Delay {
                    initial: vec![initial[i]],
                    sample_time: *sample_time,
                }
            },
        );
        let next = format!("rate-{i}");
        add(
            &mut g,
            next.clone(),
            1,
            BlockKind::Sum {
                signs: a[i].iter().chain(&b[i]).copied().collect(),
            },
        );
        for j in 0..n {
            edge(&mut g, format!("state-{j}"), next.clone(), j);
        }
        for j in 0..m {
            edge(&mut g, format!("input-{j}"), next.clone(), n + j);
        }
        edge(&mut g, next, id, 0);
    }
    for i in 0..p {
        let id = format!("output-{i}");
        let mut terms: Vec<_> = (0..n).map(|j| (format!("state-{j}"), c[i][j])).collect();
        // Keep every C state dependency (including zero) to retain hidden modes. Only nonzero D feeds through.
        terms.extend(
            (0..m)
                .filter(|j| d[i][*j] != 0.)
                .map(|j| (format!("input-{j}"), d[i][j])),
        );
        add(
            &mut g,
            id.clone(),
            1,
            if terms.is_empty() {
                BlockKind::Constant { value: vec![0.] }
            } else {
                BlockKind::Sum {
                    signs: terms.iter().map(|(_, v)| *v).collect(),
                }
            },
        );
        for (j, (from, _)) in terms.into_iter().enumerate() {
            edge(&mut g, from, id.clone(), j);
        }
        let from = if *mode == PidMode::Discrete {
            let held = format!("held-{i}");
            add(
                &mut g,
                held.clone(),
                1,
                BlockKind::DiscreteTransferFunction {
                    numerator: vec![1.],
                    denominator: vec![1.],
                    initial: vec![],
                    sample_time: *sample_time,
                },
            );
            edge(&mut g, id, held.clone(), 0);
            held
        } else {
            id
        };
        edge(&mut g, from, "vector".into(), i);
    }
    add(&mut g, "vector".into(), p, BlockKind::Mux { inputs: p });
    add(&mut g, "output".into(), p, BlockKind::Output { port: 0 });
    edge(&mut g, "vector".into(), "output".into(), 0);
    Ok(g)
}
