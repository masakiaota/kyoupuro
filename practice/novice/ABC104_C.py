def readln():
    return list(map(int, input().split()))


D, G = readln()
PC = [readln() for _ in range(D)]

ans = 10000

for i in range(1 << D):
    score = 0
    difficulty = -1  # バグにならないようにあとで注意
    n_problem = 0
    for j in range(D):
        if (i >> j) % 2:
            score += PC[j][1] + PC[j][0] * 100 * (j + 1)
            n_problem += PC[j][0]
        else:
            difficulty = max(difficulty, j)

    if difficulty == -1:  # すべてが−1となるとき全問取り終わっているということ
        ans = min(ans, n_problem)
        break

    for j in range(PC[difficulty][0]):
        # print(j)
        if score >= G:
            ans = min(ans, n_problem)
            # print(ans)
            break
        else:
            score += (difficulty+1) * 100
            n_problem += 1

print(ans)
