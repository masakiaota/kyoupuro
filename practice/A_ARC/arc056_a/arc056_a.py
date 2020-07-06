# https://atcoder.jp/contests/arc056/tasks/arc056_a
A, B, K, L = map(int, input().split())

# どう考えてもなるべくセットで買ってから残りをA円で買う
n_set = K // L
n_ikko = K - (n_set * L)

# print(n_set, n_ikko)
print(min(n_set * B + n_ikko * A, (n_set + 1) * B))

# オーバーしてもいいのか
