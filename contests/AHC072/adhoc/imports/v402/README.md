# v402 frozen experiment evidence

Result: rejected. One In100 evaluation, GCC15.3/Rust1.89, LOCAL 1.90/1.930, j8. The implementation was not modified after evaluation. See notes/experiments/v402.md.

`reproduction.zip` contains official per-case records, independent replay summaries and per-case output hashes, numerical equality summaries, stage-level observations, preprocessing differences and hashes, the frozen source/note, and v402 scripts. Full preprocessing `.ii`, compiled binaries, duplicated extracted repair files and numeric text are omitted from this compact archive; their SHA256 manifest is retained and scripts reproduce preprocessing/probes. All raw output/stderr (including raw and accepted repair plans) and bulk evidence remain under results/analysis/v402 and the separately delivered full ZIP.

The recorded cloud toolchain helpers assume the original cloud installation under /tmp; on another environment provide GCC15.3 and Rust1.89. Standard scoring entry: `CXX=g++-15 python3 scripts/eval.py v402_initial_nn_repair tools/in -j 8 --label <new-authorized-label>`. Do not run again without a new explicit user instruction. Source materialization and numerical probes also require the frozen v401/v106 parent sources from this commit.

`run_v402_checks.sh` expects the original cloud helper path. Exact original helpers are archived under reproduction_helpers; they are evidence, not installed software. Existing repo scripts/build_solver.sh and scripts/eval.py remain unchanged.

No AtCoder submission or public repository upload occurred. The private main publication is coordinated separately.
