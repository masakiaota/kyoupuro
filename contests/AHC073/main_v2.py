# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/9afb105b7761edfd67f158672e5c47caa90510f8/contests/AHC073


"""
アイデアメモ

done 0. 実装しないけど歩行のみで訪れる。そうするとM+1点が得られる。
done 1. 新たに生成される魔法陣を無視して移動距離を最短にするっていうのが、一番シンプルなベースラインの方法だね
WIP 2. 新たに生成される魔法陣をどのように活用するかが焦点。例えば、最短経路を作った後に交差するような点にたどり着いたときに転移をすると、そこが後で移動の中心点になるので、ちょっと短くなるかな？
3. うまく探索する

"""

"""
stateで経路上に初期配置以外の未使用の魔法陣があった場合に積極的に活用する。
"""
# %%
from copy import deepcopy
from enum import Enum
from dataclasses import dataclass


def manhattan_distance(ij1: tuple[int, int], ij2: tuple[int, int]) -> int:
    return abs(ij1[0] - ij2[0]) + abs(ij1[1] - ij2[1])


def read_input() -> tuple[int, int, int, int, int, int, list[tuple[int, int]]]:
    """
    _, _, _, T, si, sj, g = read_input()
    """
    N, M, K, T = map(int, input().split())
    si, sj = map(int, input().split())
    g: list[tuple[int, int]] = []
    for _ in range(M):
        gi, gj = map(int, input().split())
        g.append((gi, gj))
    return N, M, K, T, si, sj, g


class WalkDirection:
    U = "U"
    D = "D"
    L = "L"
    R = "R"


@dataclass
class Walk:
    dir: str


@dataclass
class Jump:
    circle_ij: tuple[int, int]


class State:
    def __init__(
        self,
        si,
        sj,
        list_of_circle: list[tuple[int, int]],
        list_of_g: list[tuple[int, int]],
    ):
        """
        人の場所と訪問済みのマス、魔法陣の位置、turn数、完了か？ を把握する必要がある。
        """
        self.w = 128
        self.h = 128
        self.human_ij: tuple[int, int] = (si, sj)
        self.set_of_circle: set[tuple[int, int]] = set(deepcopy(list_of_circle))
        self.inv_list_of_g: list[tuple[int, int]] = list(
            reversed(list_of_g)
        )  # 先頭の要素を最後にすることで削除してからのアクセスを楽にする
        self.turn = 0
        self.is_completed = False
        self.now_on_goal = True
        self.actions: list[Walk | Jump] = []

    # 行もコピーするので、元の状態と盤面を共有しない。
    def copy(self):
        return State(
            self.human_ij[0],
            self.human_ij[1],
            list(self.set_of_circle),
            list(reversed(self.inv_list_of_g)),
        )

    # action = (行, 列, 新しい値)。書き換え前の情報を返す。
    def apply(self, action: Walk | Jump):
        # まずは行動
        if isinstance(action, Walk):
            if action.dir == WalkDirection.U:
                self.human_ij = (self.human_ij[0] - 1, self.human_ij[1])
            elif action.dir == WalkDirection.D:
                self.human_ij = (self.human_ij[0] + 1, self.human_ij[1])
            elif action.dir == WalkDirection.L:
                self.human_ij = (self.human_ij[0], self.human_ij[1] - 1)
            elif action.dir == WalkDirection.R:
                self.human_ij = (self.human_ij[0], self.human_ij[1] + 1)

        elif isinstance(action, Jump):
            circle_ij_to_add = self.human_ij
            self.human_ij = (
                action.circle_ij[0] * 2 - self.human_ij[0],
                action.circle_ij[1] * 2 - self.human_ij[1],
            )
            # 魔法陣の削除と作成
            if action.circle_ij in self.set_of_circle:
                self.set_of_circle.remove(action.circle_ij)
                self.set_of_circle.add(circle_ij_to_add)

        # たどりついたかの判定
        if self.inv_list_of_g and self.human_ij == self.inv_list_of_g[-1]:
            # 辿り着いたら最後の要素を出す。
            self.inv_list_of_g.pop()
            self.now_on_goal = True
        else:
            self.now_on_goal = False

        if not self.inv_list_of_g:  # これで判定できるならばいらないかもしれない。外部でチェックすればいいのだから
            self.is_completed = True

        # ターン変更
        self.turn += 1

    def score(self, T: int):
        if self.is_completed:
            # M + T - A + 1
            return 150 + T - self.turn + 1
        else:
            # V + 1
            return len(self.inv_list_of_g) + 1


# %%
def walk_between_g(g1: tuple[int, int], g2: tuple[int, int]) -> list[Walk]:
    """
    g1→g2への歩行のみのactions列を生成する。
    複数の経路が存在すると思われるが、本関数では単純に上下左右の順で移動する経路を生成する。(乱数で生成したら少し多様性が出るかもね)
    """
    actions: list[Walk] = []
    hi, hj = g1
    gi, gj = g2
    while hi < gi:  # g1の方が上にある
        actions.append(Walk(dir=WalkDirection.D))
        hi += 1
    while hi > gi:
        actions.append(Walk(dir=WalkDirection.U))
        hi += -1
    while hj < gj:
        actions.append(Walk(dir=WalkDirection.R))
        hj += 1
    while hj > gj:
        actions.append(Walk(dir=WalkDirection.L))
        hj += -1
    return actions


def listup_manhattan(si: int, sj: int, list_of_g: list[tuple[int, int]]):
    """次の地点までのmanhattan距離を計算する。"""
    goallist = [(si, sj)] + list_of_g
    list_of_manhattan = []
    for now, next in zip(goallist, goallist[1:]):
        list_of_manhattan.append(manhattan_distance(now, next))
    return list_of_manhattan


def center_point_and_actions(
    g1: tuple[int, int], g2: tuple[int, int]
) -> tuple[tuple[int, int], list[Jump | Walk]]:
    """
    2点g1, g2を選んで、その中心点を計算し、さらにその中心点のcircleを使って転移するとしたときのaction列も返す。
    g1→g2への移動のaction列である。
    """
    ci = (g1[0] + g2[0]) // 2
    cj = (g1[1] + g2[1]) // 2

    center = (ci, cj)
    # centerはどの点とどの点をつなぐのか？それによって変わるな。つまり1マスずれるかによって、walkの追加が発生する
    # 2で割り切れるときは問題なくて、割り切れないときは場合によっては盤面外に行く可能性がある。
    jump_distination: tuple[int, int] = (2 * ci - g1[0], 2 * cj - g1[1])
    if jump_distination[0] < 0:
        center = (ci + 1, cj)
    if jump_distination[1] < 0:
        center = (ci, cj + 1)
    jump_distination = (2 * center[0] - g1[0], 2 * center[1] - g1[1])
    to_walk = walk_between_g(jump_distination, g2)

    # g1からjump_distination まで転送。そのあと g2 までの歩行経路を追加
    actions: list[Jump | Walk] = [Jump(circle_ij=center)] + to_walk

    return center, actions


def greedy(
    sg: list[tuple[int, int]],
) -> tuple[list[tuple[int, int]], list[list[Jump | Walk]], list[int]]:
    """
    1. 各 g_m→g_m+1 の経路のうち、マンハッタン距離が長いものを列挙
    2. 2つのゴールの中心点に近い座標を選んで、魔法陣を置く。またg_m→g_m+1 の経路を確定する。
    3. 他のg_m→g_m+1 は歩行のみ。

    戻り値は
    - list_of_circle: 初期設置の魔法陣の座標のリスト
    - actions: ゴールごとの全ての行動のリスト (Jump と Walk の混合)
    - sg_idx_to_fix: greedyで固定された経路のindexリスト
    """
    actions_list_with_i: list[list[Jump | Walk]] = []  # indexごとのactions list
    for i in range(len(sg) - 1):  # i番目からi+1番目の地点のリスト
        actions_list_with_i.append([])

    manhattan_dists: list[int] = listup_manhattan(
        sg[0][0], sg[0][1], sg[1:]
    )  # iからi+1までのマンハッタン距離のリスト
    # assert len(manhattan_dists) == len(sg) - 1
    # indexと対応付ける
    manhattan_dists_with_index: list[tuple[int, int]] = []
    for i in range(len(manhattan_dists)):
        manhattan_dists_with_index.append((manhattan_dists[i], i))
    manhattan_dists_with_index.sort(reverse=True)  # 距離が長い順にソート

    # TOP K を抜き出すして先に経路を作る
    sg_idx_to_fix: list[int] = []
    list_of_circle: list[tuple[int, int]] = []
    for dist, i in manhattan_dists_with_index[:3]:
        center, actions = center_point_and_actions(sg[i], sg[i + 1])
        list_of_circle.append(center)
        actions_list_with_i[i] = actions
        sg_idx_to_fix.append(i)

    # ここで、TOP K 以外の経路を歩行のみで確定する処理を追加する
    for dist, i in manhattan_dists_with_index[3:]:
        actions_list_with_i[i] = walk_between_g(sg[i], sg[i + 1])

    return list_of_circle, actions_list_with_i, sg_idx_to_fix


def write_output(list_of_circle: list[tuple[int, int]], actions: list[Jump | Walk]):
    for circle in list_of_circle:
        print(*circle)
    for action in actions:
        if isinstance(action, Walk):
            print(action.dir)
        elif isinstance(action, Jump):
            print("J", *action.circle_ij)


# stateを進行していって、各goal時と次へのgoal時に初期魔法陣以外が使えないか検討する


def try_use_unused_circles(
    state: State,
    list_of_circle_to_fix: list[tuple[int, int]],
    prev_sub_action: list[Jump | Walk],
) -> list[Jump | Walk]:
    """
    あるゴール※に到着した瞬間に呼び出される。次のゴールに向かうときに積極的に魔法陣を使うaction列を返す
    ※ただし、fixedなcircleは使用する区間では呼び出されない。
    """
    # 次の目的地の取得
    # 現在位置から次の目的地の間に、使用可能な魔法陣があるか？
    # もしあれば、アクション列を作成、なければ、今までどおりの歩行のみのアクション列を返す
    next_goal = state.inv_list_of_g[-1]
    now = state.human_ij

    # バウンディングボックスの中に使用可能な魔法陣があれば使用できる。
    # bbox = (min_i, min_j, max_i, max_j) の形式でバウンディングボックスを定義
    bbox = (
        min(now[0], next_goal[0]),
        min(now[1], next_goal[1]),
        max(now[0], next_goal[0]),
        max(now[1], next_goal[1]),
    )
    is_usable = False
    for circle in state.set_of_circle:
        if circle in list_of_circle_to_fix:
            continue
        # print("Checking circle:", circle, "bbox:", bbox)
        if bbox[0] <= circle[0] <= bbox[2] and bbox[1] <= circle[1] <= bbox[3]:
            is_usable = True
            circle_to_use = circle
            break

    if not is_usable:
        return prev_sub_action

    # print("Using circle:", circle_to_use, "from", now, "to", next_goal)

    # bboxにぶつかる最大の正方形で転移可能それまでは歩く。
    # 転移bboxの取得bboxの4点のうち一番近いものへ歩けばよい。

    # 転移正方形の定義
    half_wh = min(
        abs(circle_to_use[0] - bbox[0]),
        abs(circle_to_use[0] - bbox[2]),
        abs(circle_to_use[1] - bbox[1]),
        abs(circle_to_use[1] - bbox[3]),
    )
    four_points = [
        (circle_to_use[0] - half_wh, circle_to_use[1] - half_wh),
        (circle_to_use[0] - half_wh, circle_to_use[1] + half_wh),
        (circle_to_use[0] + half_wh, circle_to_use[1] - half_wh),
        (circle_to_use[0] + half_wh, circle_to_use[1] + half_wh),
    ]  # 歩きではこの4点から一番近いものを選ぼう

    # nowから一番近い四点を選ぶ
    now_nearest_dist = 10000000
    for point in four_points:
        dist = manhattan_distance(now, point)
        if dist < now_nearest_dist:
            now_nearest_dist = dist
            now_nearest_point = point

    # next_goalから一番近い四点を選ぶ
    next_goal_nearest_dist = 10000000
    for point in four_points:
        dist = manhattan_distance(next_goal, point)
        if dist < next_goal_nearest_dist:
            next_goal_nearest_dist = dist
            next_goal_nearest_point = point

    # walk → jump → walk のアクション列を作成
    sub_actions: list[Jump | Walk] = []
    # nowからnow_nearest_pointまで歩く
    sub_actions.extend(walk_between_g(now, now_nearest_point))
    # now_nearest_pointからcircle_to_useへジャンプ
    sub_actions.append(Jump(circle_to_use))
    # circle_to_useからnext_goal_nearest_pointまで歩く
    sub_actions.extend(walk_between_g(next_goal_nearest_point, next_goal))

    return sub_actions


def ins_jump_actions(
    state: State,
    actions_with_i: list[list[Jump | Walk]],
    sg_idx_to_fix: list[int],
    list_of_circle_to_fix: list[
        tuple[int, int]
    ],  # これを使ってはいけない(あとで消化されるものなので。その判定用)
) -> list[Jump | Walk]:
    ret_actions = []
    for i, sub_actions in enumerate(actions_with_i):
        if i in sg_idx_to_fix:
            for action in sub_actions:  # 単純に進める
                state.apply(action)
            ret_actions.extend(sub_actions)
        else:  # circleを積極的に使う
            # ここで関数を呼び出す
            new_sub_actions = try_use_unused_circles(
                state, list_of_circle_to_fix, sub_actions
            )
            for action in new_sub_actions:  # 新しいので進める
                state.apply(action)
            ret_actions.extend(new_sub_actions)
    return ret_actions


def main():
    N, M, K = 128, 150, 3
    _, _, _, T, si, sj, g = read_input()
    sg = [(si, sj)] + g
    list_of_circle, actions_with_i, sg_idx_to_fix = greedy(sg)  # greedy(si, sj, g)
    state = State(si, sj, list_of_circle, g)
    actions = ins_jump_actions(state, actions_with_i, sg_idx_to_fix, list_of_circle)
    write_output(list_of_circle, actions)


if __name__ == "__main__":
    main()

# %%
