#!/usr/bin/env python3
"""Apply the preregistered capacity-only transformation to the frozen parent."""
from pathlib import Path
import difflib
import hashlib
import json

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v033_capacity"
PARENT = ROOT / "src/bin/v028_two_order_lns.cpp"
CHILD = ROOT / "src/bin/v033_capacity_lns.cpp"


def transform(parent):
    s = parent.replace("// v028_two_order_lns.cpp", "// v033_capacity_lns.cpp", 1)
    s = s.replace("constexpr int MV = 400, INF = 30000;\nusing Board = array<Word, MV>;",
                  "constexpr int INF = 30000;")
    start = s.index("int main() {")
    main = s[start:]
    s = s[:start]
    prelude, body = main.split("    geo.read();\n", 1)
    s = s.replace("struct Geometry {", '''struct Input {
    int N, K, cell_count = 0;
    vector<string> C;
    void read() {
        cin >> N >> K;
        C.resize(N);
        for (string& row : C) {
            cin >> row;
            cell_count += count_if(row.begin(), row.end(), [](char c) { return c != '#'; });
        }
    }
};

// 入力ごとに選んだ容量で、床別の盤面・距離表・作業配列をまとめて持つ。
template<int capacity>
struct Solver {
static constexpr int MV = capacity;
using Board = array<Word, MV>;
// 親の探索上限800は保存容量から分離する。2本の経路は合計4V-2手以下。
static constexpr int max_pair_steps = 800;
static constexpr int weight_count = min(max_pair_steps, 4 * capacity - 2) + 1;

struct Geometry {''', 1)
    s = s.replace("    string grid[20];\n    int id[20][20], row[MV], col[MV], parity[MV], nest[MV], home[13];",
                  "    vector<int> id;\n    int row[MV], col[MV], parity[MV], nest[MV], home[13];")
    old_read = '''    void read() {
        cin>>N>>K;
        memset(id,-1,sizeof(id)); memset(home,-1,sizeof(home));
        for(int i=0;i<N;i++) cin>>grid[i];'''
    new_read = '''    void read(const Input& input) {
        N=input.N;K=input.K;
        id.assign(N*N,-1);memset(home,-1,sizeof(home));'''
    assert old_read in s
    s = s.replace(old_read, new_read, 1)
    s = s.replace("grid[i][j]", "input.C[i][j]").replace("id[i][j]", "id[i*N+j]")
    s = s.replace("} geo;", "};\n// 大きなGeometryの実体は選択した型の1個だけをrun内で作る。\nstatic inline Geometry* geo;", 1)
    for declaration in ("void finishSingles(", "World treeBaseline(", "vector<Move> smoothRoutes(", "void validateOutput("):
        assert "\n"+declaration in s
        s = s.replace("\n"+declaration, "\nstatic "+declaration, 1)
    s = s.replace("double weight[2*MV+4];", "double weight[weight_count];")
    s = s.replace("d>=2*MV", "d>=max_pair_steps")
    s = s.replace("min(steps,2*MV)", "min(steps,max_pair_steps)")
    s = s.replace("d<2*MV+4", "d<weight_count")
    s = s.replace("geo.id[row][col]", "geo.id[row*geo.N+col]")
    body = body.replace("        trace.summary();", '''        trace.count_by("floor_cells", geo.V);
        trace.count_by("cell_capacity", capacity);
        trace.count_by("board_bytes", sizeof(Board));
        trace.count_by("identity_board_bytes", sizeof(IdentityBoard));
        trace.count_by("geometry_bytes", sizeof(Geometry) + geo.id.size()*sizeof(int));
        trace.count_by("floor_dist_bytes", sizeof(geo.dist));
        trace.count_by("group_bytes", sizeof(Group));
        trace.count_by("router_bytes", sizeof(Router));
        trace.count_by("portion_router_bytes", sizeof(PortionRouter));
        trace.count_by("constructor_bytes", sizeof(Constructor));
        trace.count_by("temporal_lns_bytes", sizeof(TemporalLNS));
        trace.count_by("weight_bytes", weight_count*sizeof(double));
        trace.summary();''')
    assert body.endswith("}\n")
    body = body[:-2] + "    return 0;\n}\n"
    s += '''static int run(const Input& input) {
    Geometry geometry{};
    geo = &geometry;
    geo->read(input);
''' + body + "};\n\n"
    s = s.replace("geo.", "geo->")
    s += '''// 探索中の容量分岐・盤面ごとの動的確保を避け、型の選択は入力後の1回だけ行う。
template<class F>
decltype(auto) with_capacity(int cell_count, F&& body) {
    switch ((cell_count - 1) / 32) {
'''
    for index, capacity in enumerate([*range(32, 385, 32), 400]):
        s += f"        case {index}: return body.template operator()<{capacity}>();\n"
    s += "    }\n    unreachable();\n}\n\n"
    s += prelude + '''    Input input;
    input.read();
    return with_capacity(input.cell_count, [&]<int capacity>() {
        return Solver<capacity>::run(input);
    });
}
'''
    return s


def main():
    OUT.mkdir(exist_ok=True)
    parent = PARENT.read_text()
    child = transform(parent)
    CHILD.write_text(child)
    (OUT / "source.diff").write_text("".join(difflib.unified_diff(
        parent.splitlines(True), child.splitlines(True), fromfile=PARENT.name, tofile=CHILD.name)))
    hashes = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in (PARENT, CHILD, ROOT/"src/bin/v000_template.cpp", ROOT/"notes/experiments/v033.md")}
    (OUT / "initial_hashes.json").write_text(json.dumps(hashes, indent=2)+"\n")
    print(json.dumps(hashes, indent=2))


if __name__ == "__main__":
    main()
