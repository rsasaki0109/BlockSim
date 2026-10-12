use blocksim_core::graph::{simulate_graph, GraphModel};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "samples/graphs/mixed-pid.json".into());
    let model: GraphModel = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let result = simulate_graph(&model)?;
    println!(
        "{} points, {} scopes, {}",
        result.times.len(),
        result.traces.len(),
        result.solver
    );
    for trace in &result.traces {
        println!("{}: final={:?}", trace.id, trace.values.last().unwrap());
    }
    if let Some(output) = args.next() {
        std::fs::write(output, serde_json::to_string(&result)?)?;
    }
    Ok(())
}
