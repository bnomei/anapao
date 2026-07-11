# Current-State Research Questions

1. What public fields, defaults, constructors, builders, derives, exports, and rustdoc currently
   define `CaptureConfig`, `RunConfig`, `BatchRunTemplate`, and `BatchConfig`?
2. What JSON shape does derived Serde currently emit and accept for capture and nested batch run
   configuration, and which compatibility tests already protect it?
3. Where is a zero capture stride rejected, and does any runtime code independently coerce zero to
   another value?
4. How do empty node and metric sets behave at capture time, and can either channel currently
   represent an explicit empty selection?
5. Which other diagnostic channels are retained per step or per transfer even though the capture
   type only names nodes and metrics?
6. Which terminal values are always calculated and which consumers depend on them independently of
   time-series capture?
7. Does capture configuration affect live event emission, assertion checkpoints, or artifact event
   logs?
8. How does batch execution obtain per-run data, what portions become `BatchRunSummary`, what
   portions feed aggregate series, and what portions are discarded?
9. What order do sequential and Rayon execution return today, and which tests establish equality
   and floating-point aggregation determinism?
10. How do run and batch assertions distinguish final values from step series, and how do artifact
    writers behave when series or variable snapshots are empty?
11. Which benchmark cases claim capture-disabled behavior, which feature modes are already
    measured, and is peak memory measured anywhere?
12. What crate version/MSRV and Cargo compatibility rules constrain a public field migration?
13. Which official Rust, Serde, Rayon, Criterion, and heap-profiling sources are needed to validate
    the type and measurement patterns without treating engineering inferences as guarantees?
14. Do any sibling specs constitute a real semantic prerequisite, rather than merely an overlapping
    edit surface?
