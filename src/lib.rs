// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Anapao-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Anapao and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Anapao — deterministic simulation testing utility.
//!
//! ## Concepts
//! - `ScenarioSpec`: declarative simulation graph (nodes, edges, end conditions, metrics).
//! - `RunConfig`: deterministic single-run controls (`seed`, `max_steps`, capture options).
//! - `BatchConfig`: deterministic Monte Carlo controls (`runs`, `base_seed`, execution mode).
//! - `Expectation`: typed assertions evaluated against run or batch reports.
//! - Artifacts: manifested CI-friendly outputs (`events.jsonl`, `series.csv`, `summary.csv`, ...).
//!
//! ## Deterministic Single Run
//! ```rust
//! use anapao::{Simulator, testkit};
//! use anapao::types::MetricKey;
//!
//! let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
//! let report = Simulator::run(&compiled, testkit::deterministic_run_config(), None).unwrap();
//!
//! assert!(report.completed);
//! assert_eq!(report.steps_executed, 3);
//! assert_eq!(report.final_metrics.get(&MetricKey::fixture("sink")), Some(&3.0));
//! ```
//!
//! ## Deterministic Batch (Monte Carlo)
//! ```rust
//! use anapao::{Simulator, testkit};
//! use anapao::types::MetricKey;
//!
//! let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
//! let batch = Simulator::run_batch(&compiled, testkit::deterministic_batch_config(), None).unwrap();
//!
//! assert_eq!(batch.completed_runs, batch.requested_runs);
//! assert!(batch.runs.windows(2).all(|window| window[0].run_index < window[1].run_index));
//! assert!(batch.aggregate_series.contains_key(&MetricKey::fixture("sink")));
//! ```
//!
//! ## Assertions Plus Event Stream
//! ```rust
//! use anapao::{Simulator, testkit};
//! use anapao::assertions::{Expectation, MetricSelector};
//! use anapao::events::VecEventSink;
//! use anapao::types::MetricKey;
//!
//! let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
//! let expectations = vec![Expectation::Equals {
//!     metric: MetricKey::fixture("sink"),
//!     selector: MetricSelector::Final,
//!     expected: 3.0,
//! }];
//!
//! let mut sink = VecEventSink::new();
//! let (_report, assertion_report) = Simulator::run_with_assertions(
//!     &compiled,
//!     testkit::deterministic_run_config(),
//!     &expectations,
//!     Some(&mut sink),
//! )
//! .unwrap();
//!
//! assert!(assertion_report.is_success());
//! assert!(sink
//!     .events()
//!     .iter()
//!     .any(|event| event.event_name() == "assertion_checkpoint"));
//! ```
//!
//! ## Full Playbook (Setup -> Run -> Assert -> Artifacts)
//! ```no_run
//! use anapao::{Simulator, testkit};
//! use anapao::artifact::write_run_artifacts_with_assertions;
//! use anapao::assertions::{Expectation, MetricSelector};
//! use anapao::events::VecEventSink;
//! use anapao::types::MetricKey;
//!
//! // 1) Setup scenario + compile.
//! let scenario = testkit::fixture_scenario();
//! let compiled = Simulator::compile(scenario).unwrap();
//!
//! // 2) Setup run config + expectations.
//! let run_config = testkit::deterministic_run_config();
//! let expectations = vec![Expectation::Equals {
//!     metric: MetricKey::fixture("sink"),
//!     selector: MetricSelector::Final,
//!     expected: 3.0,
//! }];
//!
//! // 3) Run simulation and evaluate assertions.
//! let mut sink = VecEventSink::new();
//! let (run_report, assertion_report) = Simulator::run_with_assertions(
//!     &compiled,
//!     run_config,
//!     &expectations,
//!     Some(&mut sink),
//! )
//! .unwrap();
//! assert!(assertion_report.is_success());
//!
//! // 4) Persist artifact pack for CI/debugging.
//! let output_dir = std::env::temp_dir().join("anapao-doc-playbook");
//! let manifest = write_run_artifacts_with_assertions(
//!     &output_dir,
//!     &run_report,
//!     sink.events(),
//!     Some(&assertion_report),
//! )
//! .unwrap();
//! assert!(manifest.artifacts.contains_key("manifest"));
//! ```

#![forbid(unsafe_code)]

pub mod artifact;
pub mod assertions;
pub mod batch;
pub mod engine;
pub mod error;
pub mod events;
pub mod expr;
pub mod rng;
pub mod simulator;
pub mod stats;
pub mod stochastic;
pub mod testkit;
pub mod types;
pub mod validation;

#[cfg(feature = "analysis-polars")]
pub mod analysis;

pub use simulator::Simulator;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_doctest {}
