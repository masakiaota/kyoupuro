# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/3/ALDS1_3_D
# これは少し難しい
# stackをうまく利用して個々の湖の面積を出せるかが重要
# 条件を満たしたら面積mergeをするにはどうしたら良いのか考えよう。

chikei = input()
v_lakes = []  # 個々の湖の面積管理用のstack # (面積の始まりのidx, 面積)
idx_stack = []  # 地形のindex管理用のstack

for i, c in enumerate(chikei):
    if c is '\\':
        idx_stack.append(i)
    elif c is '/' and idx_stack:
        j = idx_stack.pop()  # 今の/に対応する\の位置
        v = i-j
        # 可能であるならば面積merge
        # 今のjがv_lakesのj_preよりも小さかったら面積mergeができるということ
        while v_lakes and j < v_lakes[-1][0]:
            # すべての面積をmerge
            v += v_lakes.pop()[1]
        v_lakes.append((j, v))

ans = [len(v_lakes)]
v = [v[1] for v in v_lakes]
ans.extend(v)

print(sum(v))
print(*ans)
