#!/usr/bin/env python3
"""Port only working Board capacities, retaining v028's direct global Geometry."""
from pathlib import Path
import difflib
import hashlib
import re

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v035_tower_capacity"
PARENT = ROOT / "src/bin/v028_two_order_lns.cpp"
CHILD = ROOT / "src/bin/v035_tower_capacity_lns.cpp"


def once(s, old, new):
    if s.count(old) != 1:
        raise RuntimeError(f"Expected exactly one: {old!r}")
    return s.replace(old, new, 1)


def transform(s):
    s = once(s, "// v028_two_order_lns.cpp", "// v035_tower_capacity_lns.cpp")
    s = once(s, "using Board = array<Word, MV>;", """// 試行で複製する塔だけを床数以上の32刻みにする。容量の選択は入力後の1回だけ。
template<size_t capacity>
using Board = array<Word, capacity>;""")
    at = s.index("struct Geometry {")
    prefix, body = s[:at], s[at:]
    body = once(body, "    Board initial{};", """    // 初期配置は共有地形内の1個だけ。元の配置と直接参照を保つ。
    array<Word, MV> initial{};""")
    body = re.sub(r"\bBoard\b", "Board<capacity>", body)
    body = re.sub(r"\bWorld\b", "World<capacity>", body)
    body = once(body, "struct World<capacity> {", "template<size_t capacity>\nstruct World {")
    body = once(body, "explicit World<capacity>(", "explicit World(")
    for declaration in ("class Constructor {",):
        body = once(body, declaration, "template<size_t capacity>\n" + declaration)
    for declaration in (
        "    int apply(Board<capacity> &b,Move m) const {",
        "    int count(const Board<capacity> &b) const {",
        "    void run(const Board<capacity> &b,int source,int target) {",
        "    DeliveryDP(const Board<capacity> &b,int selected) {",
        "    void emit(World<capacity> &w,Word word,int p) {",
        "    void finish(World<capacity> &w,int p) {",
        "    void firstEvent(World<capacity> &w,int p) {",
        "    void run(const Board<capacity>& b,int s,int k,int goal,int maxLength=INF) {",
        "    Board<capacity> words() const {",
        "    bool extract(const vector<Move>& original,const vector<int>& removed,",
        "    static int shortcutSupportNeed(const Board<capacity>& b,const vector<Move>& base,int t,int q,int hq) {",
        "    bool insertAt(const Board<capacity>& initial,const vector<Move>& base,int source,int color,",
        "    bool insert(const Board<capacity>& initial,const vector<Move>& base,int token,int cap,",
        "    bool insertTwoOrders(const Board<capacity>& initial,const vector<Move>& base,",
        "    bool insertPacket(const Board<capacity>& initial,const vector<Move>& base,int source,",
    ):
        body = once(body, declaration, "    template<size_t capacity>\n" + declaration)
    body = once(body, "void finishSingles(World<capacity> &w) {",
                "template<size_t capacity>\nvoid finishSingles(World<capacity> &w) {")
    body = body.replace("treeBaseline(", "treeBaseline<capacity>(")
    body = once(body, "World<capacity> treeBaseline<capacity>(int variant) {",
                "template<size_t capacity>\nWorld<capacity> treeBaseline(int variant) {")
    body = body.replace("smoothRoutes(", "smoothRoutes<capacity>(")
    body = once(body, "vector<Move> smoothRoutes<capacity>(const vector<Move>& answer) {",
                "template<size_t capacity>\nvector<Move> smoothRoutes(const vector<Move>& answer) {")
    body = body.replace(".words()", ".words<capacity>()")
    body = re.sub(r"geo\.initial(?!\[)", "geo.initial_board<capacity>()", body)
    body = once(body, "    Word normalize(Word a,int p) const {", """    // 初期配置から作業盤面を作る。未使用枠も元の0をコピーし、通常の配列比較を保つ。
    template<size_t capacity>
    Board<capacity> initial_board() const {
        Board<capacity> board;
        copy_n(initial.begin(), capacity, board.begin());
        return board;
    }
    Word normalize(Word a,int p) const {""")
    body = once(body, "            Constructor solver(", "            Constructor<capacity> solver(")
    # TemporalLNS holds no Board. Keep its helper types and capacity-independent methods shared.
    for name, declaration in (
        ("extractFrom", "    bool extractFrom(const vector<Move>& original,IdentityBoard& cur,int start,"),
        ("packetNeighbor", "    bool packetNeighbor(const vector<Move>& current,const PacketCut& cut,int slack,"),
        ("flexibleNeighbor", "    bool flexibleNeighbor(const vector<Move>& current,const PacketCut& cut,int slack,"),
        ("pairedNeighbor", "    bool pairedNeighbor(const vector<Move>& current,int slack,vector<Move>& result) {"),
        ("causalReorder", "    vector<Move> causalReorder(const vector<Move>& original,int style) {"),
        ("optimize", "    void optimize(vector<Move>& best) {"),
    ):
        body = re.sub(r"\b" + name + r"\(", name + "<capacity>(", body)
        altered = declaration.replace(name + "(", name + "<capacity>(")
        body = once(body, altered, "    template<size_t capacity>\n" + declaration)
    # The original main prologue, including LOCAL timer initialization, remains in main.
    at = body.index("int main() {")
    definitions, main = body[:at], body[at:]
    prologue, solve = main.split("    geo.read();\n", 1)
    solve = once(solve, "        trace.summary();", """        trace.count_by("floor_cells", geo.V);
        trace.count_by("cell_capacity", capacity);
        trace.count_by("board_bytes", sizeof(Board<capacity>));
        trace.count_by("identity_board_bytes", sizeof(IdentityBoard));
        trace.count_by("geometry_bytes", sizeof(Geometry));
        trace.summary();""")
    assert solve.rstrip().endswith("}")
    solve = "template<size_t capacity>\nint solve() {\n" + solve.rstrip()[:-1] + "    return 0;\n}\n"
    dispatch = """
// 容量に依存しない地形と処理は共通のまま、必要な固定長配列の型を選ぶ。
template<class F>
decltype(auto) with_board_capacity(int floors,F&& body) {
    switch((floors-1)/32) {
"""
    for index, capacity in enumerate([*range(32, 385, 32), 400]):
        dispatch += f"        case {index}:return body.template operator()<{capacity}>();\n"
    dispatch += "    }\n    unreachable();\n}\n\n"
    main = prologue + """    geo.read();
    return with_board_capacity(geo.V,[&]<size_t capacity>() { return solve<capacity>(); });
}
"""
    result = prefix + definitions + solve + dispatch + main
    if "geo->" in result or "struct Solver" in result:
        raise RuntimeError("Unwanted solver wrapper or indirect Geometry access")
    return result


def main():
    if (OUT / "frozen.json").exists():
        raise RuntimeError("Experiment source is frozen")
    OUT.mkdir(exist_ok=True)
    parent = PARENT.read_text()
    if hashlib.sha256(parent.encode()).hexdigest() != "29aa1b31304a06d63db75ad152be6a9e52c2aa752c7e8d7b516b506e9334e717":
        raise RuntimeError("Unexpected parent revision")
    child = transform(parent)
    CHILD.write_text(child)
    (OUT / "source.diff").write_text("".join(difflib.unified_diff(parent.splitlines(True), child.splitlines(True),
                                                                fromfile=PARENT.name, tofile=CHILD.name)))
    print(CHILD)


if __name__ == "__main__":
    main()
