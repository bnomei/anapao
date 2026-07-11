//! Isolated DHAT heap measurements for capture-retention baseline comparisons.
//!
//! Each invocation accepts exactly one fixed case id so allocator statistics
//! cannot bleed between cases. The companion script runs the binary once per
//! case and records the emitted JSON with host and toolchain metadata.

use std::hint::black_box;

use anapao::testkit::fixture_scenario;
use anapao::types::{AggregationConfig, BatchConfig, EndConditionSpec};
use anapao::Simulator;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const CASE_IDS: [&str; 3] = ["single_full_256", "batch_none_256x256", "batch_all_256x256"];

fn main() {
    // `cargo test --all-targets` executes custom benchmark binaries without
    // forwarding a case ID. Treat that harness probe as a no-op; the companion
    // script always supplies one fixed case and therefore still isolates every
    // DHAT measurement in its own process.
    let Some(case_id) = std::env::args().nth(1) else {
        return;
    };
    if !CASE_IDS.contains(&case_id.as_str()) {
        usage_and_exit("unknown case id");
    }

    let dhat_output = std::env::var("ANAPAO_DHAT_OUTPUT")
        .unwrap_or_else(|_| format!("target/capture-memory/dhat-{case_id}.json"));
    if let Some(parent) = std::path::Path::new(&dhat_output).parent() {
        std::fs::create_dir_all(parent).expect("create DHAT output directory");
    }
    let profiler = dhat::Profiler::builder().file_name(dhat_output).build();
    let checksum = match case_id.as_str() {
        "single_full_256" => run_single_full(),
        "batch_none_256x256" => run_batch(AggregationConfig::none()),
        "batch_all_256x256" => run_batch(AggregationConfig::default()),
        _ => unreachable!("case id was checked above"),
    };
    let stats = dhat::HeapStats::get();
    drop(profiler);

    println!(
        concat!(
            "{{\"case_id\":\"{}\",\"total_bytes\":{},",
            "\"max_bytes\":{},\"current_bytes\":{},\"checksum\":{}}}"
        ),
        case_id, stats.total_bytes, stats.max_bytes, stats.curr_bytes, checksum
    );
}

fn run_single_full() -> u64 {
    let mut scenario = fixture_scenario();
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 256 }];
    let compiled = Simulator::compile(scenario).expect("compile deterministic fixture");
    let report = Simulator::run(
        &compiled,
        &anapao::types::RunConfig::for_seed(0x000A_11CE_55ED_u64).with_max_steps(256),
    )
    .expect("run deterministic fixture");
    let checksum =
        report.node_snapshots.iter().fold(report.steps_executed, |checksum, snapshot| {
            checksum
                .wrapping_mul(131)
                .wrapping_add(snapshot.step)
                .wrapping_add(snapshot.values.len() as u64)
        });
    let checksum = checksum.wrapping_add(report.variable_snapshots.len() as u64);
    let checksum = checksum.wrapping_add(report.transfers.len() as u64);
    black_box(checksum_run(checksum, report.final_metrics.values().copied()))
}

fn run_batch(aggregation: AggregationConfig) -> u64 {
    let mut scenario = fixture_scenario();
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 256 }];
    let compiled = Simulator::compile(scenario).expect("compile deterministic fixture");
    let report = Simulator::run_batch(
        &compiled,
        &BatchConfig::for_runs(256)
            .with_base_seed(0x000A_11CE_55ED_u64)
            .with_max_steps(256)
            .with_aggregation(aggregation),
    )
    .expect("run deterministic batch");

    let mut checksum = report.completed_runs;
    for run in report.runs {
        checksum = checksum.wrapping_mul(131).wrapping_add(run.seed);
        checksum = checksum.wrapping_mul(131).wrapping_add(run.steps_executed);
        checksum = checksum_run(checksum, run.final_metrics.values().copied());
    }
    for table in report.aggregate_series.values() {
        for point in &table.points {
            checksum = checksum.wrapping_mul(131).wrapping_add(point.step);
            checksum = checksum.wrapping_mul(131).wrapping_add(point.value.to_bits());
        }
    }
    black_box(checksum)
}

fn checksum_run(values_seed: u64, values: impl IntoIterator<Item = f64>) -> u64 {
    values.into_iter().fold(values_seed, |checksum, value| {
        checksum.wrapping_mul(131).wrapping_add(value.to_bits())
    })
}

fn usage_and_exit(reason: &str) -> ! {
    eprintln!("{reason}; expected one of: {}", CASE_IDS.join(", "));
    std::process::exit(2);
}
