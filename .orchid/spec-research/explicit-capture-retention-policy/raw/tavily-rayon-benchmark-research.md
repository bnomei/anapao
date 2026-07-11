# Tavily Pro Research — Rayon, Floating Fold, And Measurement

- Date: 2026-07-11
- Request id: `e3d6336c-be05-4a18-936d-d636ba17f477`
- Model: `pro`
- Response time: 129.53 seconds
- Status: completed

The targeted prompt asked for primary documentation on Rayon indexed range collection, fixed-order
floating summation, Criterion throughput, DHAT peak live bytes, and the compact-sample architecture.

Useful findings:

- Rayon documentation/examples demonstrate ordered operations for indexed iterators and confirm
  indexed range support, including the project's Rayon version family.
- Tavily still did not find one normative sentence broad enough to treat all collected parallel
  iterators as an ordering guarantee. The design therefore explicitly sorts `IndexedBatchSample`
  values before folding.
- Compact samples plus a sequential index fold were again correctly labeled engineering inference;
  they must be validated by repository equality and performance tests.

Evidence gaps reported by Tavily:

- No sufficient primary floating-point, Criterion, or DHAT material was returned in the report.
  Those sources were opened directly in the primary-doc follow-up.

Primary URLs returned and retained:

- https://docs.rs/rayon/1.12.0/rayon/iter/trait.IndexedParallelIterator.html
- https://docs.rs/rayon/latest/rayon/range/struct.Iter.html
- https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html

The report's caution is part of the evidence: the spec owns ordering explicitly instead of
outsourcing it to an ambiguous external guarantee.
