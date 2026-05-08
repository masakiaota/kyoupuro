# AHC064 - berry8192_permute 解法メモ

## 参照元

- 記事: 引き継ぎ: ラインの最終入れ替え戦略
- URL: https://raw.githubusercontent.com/berry8192/Atc/main/ahc064/HANDOFF_PERMUTE.md
- 著者: berry8192
- サイト: GitHub
- 種別: 改善方針・公開コード引き継ぎ
- 成績・順位: 文書内の現役ベストはI.ccでavg 4913、平均約88ターン。F.ccはavg 4911、平均89ターン
- コード有無: あり。`I.cc`, `J.cc`, `NOTES.md`, `F.cc`, `D.cc`, `G.cc` など
- コードを読めたか: 読めた。`HANDOFF_PERMUTE.md`, `NOTES.md`, `I.cc`, `J.cc` を読み、`F.cc`, `D.cc`, `G.cc` は関数構造と重み差分を確認した
- 読めなかったもの: 旧版 `A.cc` から `H.cc` までの全コード全文は読んでいない。引き継ぎ文書の主対象はI.ccとpermutation拡張なので、I.cc/J.ccとNOTESを優先した

## 解法の全体像

この参照元は、既存ベストI.ccの貪欲構築をさらに改善するため、「最初から出発線rにIDグループrを作る必要はない」という探索空間拡張を提案している。各出発線に作る10連番グループを置換 `perm[r]` で選び、作り終わった後に線ごと待避線へ退避して正しい位置へ入れ替える。

既存I.cc自体は、全車両を待避線へdumpし、各出発線が次に欲しい車両を待避線から探して順に構築する。邪魔な車両は一度別の出発線に上げて別の待避線へ退避する。この退避先選択 `plan_disp` に、容量、非交差でpackしやすいか、同じ半分に置くか、連番形成するか、といった重みを入れ、乱択再試行とsmart packでターン数を詰めている。

## 主要アイデア

- I.ccは、次に必要な車両を待避線内で探し、上にある邪魔な車両を退避してから出発線へ積む。
- 邪魔な車両の退避先は、後のpackで同ターン化しやすい非交差ペアや、連番形成を高く評価する。
- 生成した逐次操作列は、依存を壊さない範囲で最も早いターンへ詰めるsmart packで並列化する。
- 乱択再試行では、評価重みにノイズを足して多数の構築順を試し、最短ターンを採用する。
- permutation拡張では、line `r` に作るIDグループを `perm[r]` として選べるようにする。
- 置換がidentityでなければ、最後に完成済み10両編成を待避線経由で線ごと入れ替える。
- `J.cc` では初期行に含まれるグループ数 `W[r][g]` を使い、bitmask DPで割当を選ぶ簡易版が実装されている。

## 最終コードの構造

### 状態表現

- `D[10]`: 出発線。構築済みの目標列を末尾側へ伸ばす。
- `Sd[10]`: 待避線。先頭から取り出す。
- `D_init[10]`: 入力の初期出発線。
- `ops`: 逐次操作列。
- `Move { type, i, j, k }`: 出力操作。
- `I.cc` では目標グループは固定 `r`、`J.cc` では `perm[r]` を追加し、line `r` に group `perm[r]` を作る。

### 観測・制約・入力の扱い

- 初手で全 `D[r]` を `Sd[r]` へ10両dumpする。`(r,r)` の10操作なので非交差で同時にpackできる。
- `op0`, `op1` は容量assertを持ち、状態を実際に更新しながら `ops` へ追加する。
- `pack_ops` は出発線・待避線の最終使用ターンと交差判定を見て、各操作を可能な最も早いターンへ入れる。
- `J.cc` のpermute phaseでは、完成後に非identityのlineを同番号待避線へdumpし、`perm[r]` の出発線へ戻す。

### 評価関数

`plan_disp(j, max_chunk, target_r)` が退避先を評価する。

- 大きいchunkを優先する。
- 一時的に上げる出発線 `m` が `target_r` でないことを加点し、構築対象を邪魔しないようにする。
- `op1(m,j)` と `op0(m,k)` が非交差ペアになりやすい配置を加点する。
- 空の待避線を加点し、既存サイズには小さなペナルティを入れる。
- 同じ半分の待避線・出発線を優先するhalf-biasを入れる。
- 退避先の先頭車両と、移すchunkの末尾側が同一グループで連番になる場合に大きく加点する。
- ノイズ `NOISE` を足して乱択再試行の多様性を出す。

I.ccではF.ccから重みが調整され、空待避線、half-bias、chain bonusが強くなっている。構築順のcostは `depth * 1000 + noise` で、F.ccにあったchain長ボーナスはI.ccでは削られている。

### 探索・構築・更新

- `find_next(r)` は、出発線 `r` が次に欲しい車両 `10*r + |D[r]|` を待避線から探し、深さ `d` と連続長 `L` を返す。
- `J.cc` では `10*perm[r] + |D[r]|` を探す。
- 未完成の出発線のうち、次車両の深さが最も浅いものを選ぶ。I.ccではノイズ付きで多数再試行する。
- 深さ `d>0` なら、上にある邪魔な `d` 両を `displace` で他待避線へ逃がす。
- 目的車両から始まる連番長 `L` をまとめて `op1` で出発線へ積む。
- これを全出発線が10両になるまで繰り返す。
- 生成操作列をsmart packし、制限時間まで乱択再試行する。

### 操作・クエリ・出力選択

- 各試行で `run_solver()` し、`pack_ops()` 後のターン数が最短なら保存する。
- 出力は最短のpackedターン列。
- `J.cc` のpermutation試行では、identity baseline、Hungarian風bitmask DPで選んだperm baseline、さらにidentity中心・一部permの乱択再試行を行う。

### 時間配分・パラメータ

- I.ccは `TIME_LIMIT_MS=1950`、J.ccは `1850`。
- F.cc/NOTESでは `NOISE=1000` と `5000` が有効で、20000や50000は寄与が薄いと分析している。
- I.ccでは `NOISE` を1000から5000の一様乱数にしている。
- J.ccではidentityを9割、Hungarian permを1割程度試す。
- HANDOFFではswap 1回を約3ターン、一般permutationを数ターンから十数ターン程度の追加コストと見積もっている。

## 実装上重要な断片

```text
run_solver():
    dump all D[r] to Sd[r]
    while some D[r] is incomplete:
        for each r:
            find next needed car in sidings
            estimate cost by depth and noise
        choose best r
        displace cars above target if needed
        move consecutive chain into D[r]
```

```text
plan_disp(j, total, target_r):
    for temporary departure m:
        for destination siding k != j:
            chunk = min(total, capacity_m, capacity_k)
            score = chunk weight
                  + non-crossing-pack bonus
                  + empty/half/chain bonuses
                  + noise
    return best (m, k, chunk)
```

```text
permute_phase():
    for r with perm[r] != r:
        op0(r, r, 10)
    for (dst=perm[r], src=r) sorted by dst:
        op1(dst, src, 10)
    rely on pack_ops to split non-crossing turns
```

## この解法の本質

I.ccの本質は、正しい連番を前から作る単純な構築を、退避先選択と後段packingで強くしている点である。逐次操作としては貪欲だが、各退避で「あとで同ターンに詰めやすいか」「連番を作るか」を評価しているため、最終的な並列スケジュールに効く操作列が生成される。

HANDOFFのpermutation案の本質は、目標線を固定する制約を緩めることにある。線ごとの完成グループを後で入れ替えるコストが小さいなら、構築中は作りやすいグループを作った方が得になる可能性がある。これは「最後に安く正規化できるなら、探索中のラベルを自由化する」という一般的な発想である。

## 真似するならまず実装する部分

まずはI.cc相当のidentity版を実装するのがよい。全dump、`find_next`、`displace`、`plan_disp`、`pack_ops` の順に作る。とくにsmart packはターン数に直結するので早めに入れる。

permutation拡張は、identity版が安定してから入れるべきである。最初は `perm=identity` を必ず候補に残し、bitmask DPで選んだ1種類のpermだけを追加試行するのが安全である。

## 注意点・未理解点

- HANDOFFは改善提案であり、permutation案のネット改善は文書上では未確定である。
- J.ccには実装があるが、NOTESにはJ.ccの平均スコア実測が載っていない。
- 旧版A-Hの全コード全文は読んでいないため、失敗試行の詳細はNOTESと関数構造からの理解に留まる。
- `pack_ops` は逐次操作順の依存をある程度守るが、依存グラフを完全に最適化するものではない。
- permutation後処理は容量・非交差・cycle処理を誤ると完成列を壊しやすい。
