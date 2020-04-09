# https://atcoder.jp/contests/agc005/tasks/agc005_a

# 'ST'を抜け。
# 文字列を連長圧縮、先頭は必ずSを、最後には必ずTになるようにする(0個を許容)
# そしたらSTSTSTSTの文字数なので二個ずつabs(n_s-n_t)を取っていけばよい。ただしTが先になくなるときは、Sの余りを次のSに移し替えなければ行けない
# とかいろいろ考えたけどおとなしくstackで実装
# ここでは何文字STができたかを数える


X = input()

# おとなしくstackで実装し直す
stack = []
n_poped = 0
for x in X:
    if x == 'S':
        stack.append(1)
    elif x == 'T':
        if stack:
            stack.pop()
            n_poped += 2

print(len(X) - n_poped)
