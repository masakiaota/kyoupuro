#!/usr/bin/env python3
"""Restore the frozen standalone v319; never execute solvers or register evals."""
from pathlib import Path
import hashlib
ROOT=Path(__file__).resolve().parents[2]
FILES={111:('v111_weight_portfolio','dd45427b0e3e791bf40c1a7f7560c734abde1d20452ffa6c34cd878fd8c02316'),318:('v318_bit_parallel_history','24de5a028ff3fcb97d1c1b6afbc1f9b3255865644a22057fb13243d15b99999d'),405:('v405_actor_projection_cache','c5d20ebe6e5d8315fdab8592779b43345767cd7d04492feae5aeab8b083d5ef9')}
EXPECTED='931b5b40e4595c9a500f1d5a2f978e067fc867a26137d8b9a55f4d13e5ee8cbb'
def sha(data):return hashlib.sha256(data).hexdigest()
def namespace(t):
 a=t.index('namespace nn {');b=t.index('} // namespace nn',a)+len('} // namespace nn');return t[a:b]
def packed(t):
 a=t.index('static constexpr char nn_packed[] =');b=t.index(';\n',a)+1;return t[a:b]
s={}
for version,(name,expected) in FILES.items():
 p=ROOT/'src/bin'/f'{name}.cpp';raw=p.read_bytes()
 if sha(raw)!=expected:raise RuntimeError(f'Parent differs: {p}')
 s[version]=raw.decode()
newnn=namespace(s[405]).replace(packed(s[405]),packed(s[111]),1)
combined=s[318].replace(namespace(s[318]),newnn,1)
a=combined.index('#include <bits/stdc++.h>')
combined='''// v319_integrated_learned_racing.cpp
// Experiment v319; main source snapshot 0cceb2ed9cb377582f378455e7a9fe17022e9d8e.
// Direct parents: v111 (fixed learned weights), v318 (v315 racing and bit histories),
// and v405 (incremental CNN and actor endpoint projection caches).
// Keep the 26,866 learned parameters exactly; no training or requantization.
// Each initial plan uses one policy. Candidate limits, RNG, acceptance, local
// search transitions, cooling, LOCAL ratio and non-LOCAL clocks are inherited.
// Cache scope is one NN search; homecoming invalidates all spatial activations.
// Experiment records are private and are not registered in eval viewer.
'''+combined[a:]
needle='        local_nn.summary();trace.count_by("E",0);trace.count_by("T",best.size());trace.summary();'
assert combined.count(needle)==1
extra='''        trace.count_by("cnn_calls",nn::incremental_stats.calls);
        trace.count_by("cnn_full",nn::incremental_stats.full);
        trace.count_by("cnn_home",nn::incremental_stats.home);
        trace.count_by("cnn_incremental",nn::incremental_stats.incremental);
        trace.count_by("cnn_full_cells_per_layer",nn::incremental_stats.full_cells);
        for(int b=0;b<=nn::NN_DEPTH;++b)trace.count_by("cnn_cells_"+to_string(b),nn::incremental_stats.cells[b]);
        trace.count_by("actor_source_hits",nn::actor_stats.source_hits);
        trace.count_by("actor_destination_hits",nn::actor_stats.dest_hits);
        trace.count_by("actor_source_projects",nn::actor_stats.source_projects);
        trace.count_by("actor_destination_projects",nn::actor_stats.dest_projects);
'''
combined=combined.replace(needle,extra+needle,1)
assert sha(combined.encode())==EXPECTED,'Generated source differs'
p=ROOT/'src/bin/v319_integrated_learned_racing.cpp'
if p.exists() and p.read_text()!=combined:raise RuntimeError('Refusing to replace a different existing v319')
p.write_text(combined)
print(p.relative_to(ROOT),EXPECTED)
