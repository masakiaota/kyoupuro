#!/usr/bin/env python3
"""Create the preregistered exact-length Board variant from v037."""
from pathlib import Path
import difflib

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v039_exact_board"
PARENT = ROOT / "src/bin/v037_pair_transfer_compression.cpp"
CHILD = ROOT / "src/bin/v039_exact_board_lns.cpp"

BOARD = r'''// 各盤面は床数ぴったりの領域を借りる。探索中に確保・解放しない。
// 最大同時使用はpairedNeighborのboundary/initial/restoredとinsertAtのbの4枚。
struct BoardPool {
    static constexpr int slots = 4;
    unique_ptr<Word[]> storage;
    Word* free_slot[slots];
    size_t bytes = 0;
    int free_count = 0;
    void prepare(int floors) {
        storage.reset(new Word[size_t(floors) * slots]);
        bytes = size_t(floors) * sizeof(Word);
        free_count = slots;
        for(int i=0;i<slots;i++)free_slot[i]=storage.get()+size_t(i)*floors;
    }
    Word* take() { return free_slot[--free_count]; }
    void give(Word* cell) { free_slot[free_count++]=cell; }
} board_pool;

class Board {
    Word* cell;
public:
    // 引数なしで作る盤面はIdentityBoard::wordsで全床を書き込んでから使う。
    Board():cell(board_pool.take()) {}
    Board(const array<Word,MV>& other):Board() {
        memcpy(cell,other.data(),board_pool.bytes);
    }
    Board(const Board& other):Board() {
        memcpy(cell,other.cell,board_pool.bytes);
    }
    Board(Board&& other) noexcept:cell(exchange(other.cell,nullptr)) {}
    ~Board() { if(cell)board_pool.give(cell); }
    Board& operator=(const Board& other) {
        if(this!=&other) {
            if(!cell)cell=board_pool.take();
            memcpy(cell,other.cell,board_pool.bytes);
        }
        return *this;
    }
    Board& operator=(const array<Word,MV>& other) {
        if(!cell)cell=board_pool.take();
        memcpy(cell,other.data(),board_pool.bytes);
        return *this;
    }
    Board& operator=(Board&& other) noexcept {
        swap(cell,other.cell);
        return *this;
    }
    Word& operator[](int p) { return cell[p]; }
    const Word& operator[](int p) const { return cell[p]; }
    bool operator==(const Board& other) const {
        return memcmp(cell,other.cell,board_pool.bytes)==0;
    }
};'''


def create():
    OUT.mkdir(exist_ok=True)
    assert not CHILD.exists()
    s = PARENT.read_text()
    replacements = [
        ('// v037_pair_transfer_compression.cpp', '// v039_exact_board_lns.cpp'),
        ('using Board = array<Word, MV>;', BOARD),
        ('    Board initial{};', '    array<Word,MV> initial{};'),
        ('    explicit World(const Board &a):b(a),remaining(geo.count(a)) { out.reserve(3000); }',
         '    explicit World(const Board &a):b(a),remaining(geo.count(a)) { out.reserve(3000); }\n'
         '    explicit World(const array<Word,MV>& a):b(a),remaining(geo.count(b)) { out.reserve(3000); }'),
        ('    geo.read();', '    geo.read();\n    board_pool.prepare(geo.V);'),
        ('        trace.count_by("baseline_ops", baseline);',
         '        trace.count_by("floor_cells", geo.V);\n'
         '        trace.count_by("board_payload_bytes", board_pool.bytes);\n'
         '        trace.count_by("board_handle_bytes", sizeof(Board));\n'
         '        trace.count_by("board_pool_slots", BoardPool::slots);\n'
         '        trace.count_by("board_pool_bytes", board_pool.bytes * BoardPool::slots);\n'
         '        trace.count_by("board_pool_free_at_end", board_pool.free_count);\n'
         '        trace.count_by("baseline_ops", baseline);')]
    child = s
    for a, b in replacements:
        assert child.count(a) == 1, a
        child = child.replace(a, b, 1)
    CHILD.write_text(child)
    restored = child
    for a, b in reversed(replacements):
        assert restored.count(b) == 1, b
        restored = restored.replace(b, a, 1)
    assert restored == s
    (OUT / "source.diff").write_text(''.join(difflib.unified_diff(s.splitlines(True), child.splitlines(True), fromfile=PARENT.name, tofile=CHILD.name)))
    print(CHILD)


if __name__ == "__main__":
    create()
