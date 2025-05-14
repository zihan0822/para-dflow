**What was the goal?**

Build a parallel dataflow solver for analyses supporting bitset optimization.

**What did you do? (Include both the design and the implementation.)**

- We built a simple representation of Bril programs that assigns variables numbers instead of strings to make it easy to apply the bitset optimization.
- We applied Tarjan's algorithm to decompose a CFG into a DAG of SCCs. DAGs, of course, naturally lend themselves to parallelism.
Thus, we would apply the sequential implementation to each SCC and schedule dependent jobs for each SCC in a thread pool following the DAG dependencies.
- We had plans for more intelligent load balancing that were scrapped due to time constraints.

**What were the hardest parts to get right?**

Correctness in the implementation of the parallel dataflow solver.

**Were you successful? (Report rigorously on your empirical evaluation.)**

We tested on the core benchmarks and wrote a fuzz tester to automatically generate "interesting" CFGs to test with.
By test, we mean we implemented a sequential oracle solver and used it to gauge the correctness of the parallel solver.
