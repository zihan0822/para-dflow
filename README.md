# Parallel Dataflow Project for CS 6120

Note to reader:
Yes, the code quality is not the greatest.

## Crates

- [`bril`](./bril/): Flattened representation of Bril programs + shim to convert "official" Rust representation into our flattened representation
- [`bril-fuzzer`](./bril-fuzzer/): Generate Bril programs with varying degrees of nesting
- [`bril-analysis`](./bril-analysis/): Sequential and parallel implementations of bitset-optimized dataflow analysis
