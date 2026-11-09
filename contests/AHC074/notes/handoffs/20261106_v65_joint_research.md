# AHC074　紙・取得・終状態の共同設計

提出候補は [v6417 の単一C++23](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/src/bin/v6417_grown_runtime_feedback.cpp)。保存されていたLinux・GCC13.3の非LOCAL公式100件は平均 **10,927.88**、E=0、不正0、最大CPU **1.826288秒**、最大対話実時間 **1.883549秒**。今回、両時間とも2秒以内でこの平均を更新した版はなかった。目標10,500まで427.88点。別のMac Studio・標準LOCALでは同じソースに10,917.39の記録がある。[出発版の当時の100件](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/cloud_20261106_final_panel.csv)

## 公式100件で同じCPUを割り当てた比較

最終提出用の .cpp をGCC 13.3のGNU C++23・非LOCALでコンパイルした。各solverは単一コア・単一スレッド。審判は商品表だけを先に示し、各注文の操作出力を読み終えてから次の商品を渡した。2秒の判定はsolverのプロセスCPU（wait4のuser＋system）と、起動から終了までの対話実時間の両方。入力100件のGit blobはすべてリポジトリの `tools/in` と一致した。

| パネル・版 | 平均点 | 同じケース・同じCPUのv6417との差 | 対応SE | 最大CPU秒 | 最大対話秒 | 対話2秒超過 |
|---|---:|---:|---:|---:|---:|---:|
| 8版・v6417 | 10,995.63 | 0 | — | 1.829054 | 1.951234 | 0 |
| 8版・v6514　探索段階の短縮と費用を学習 | 10,984.30 | −11.33 | 19.99 | 1.831563 | 1.900139 | 0 |
| 8版・v6588　相補群の候補を整列済みのまま評価 | 10,951.17 | −44.46 | 19.63 | 1.843802 | 3.134118 | 8 |
| 追加5版・v6417 | 10,951.87 | 0 | — | 1.827973 | 1.889458 | 0 |
| 追加5版・v6592　残りCPUに合わせて展開数を制御 | 10,935.14 | −16.73 | 13.37 | 1.777216 | 3.004332 | 3 |
| 追加5版・v6596　担当区間の退避価格を移植 | 10,936.69 | −15.18 | 13.20 | 1.807668 | 3.362045 | 2 |
| 追加5版・v6597　相補群ごとに側面6窓の符号を選択 | 11,007.63 | +55.76 | 20.26 | 1.838056 | 1.904990 | 0 |
| 追加5版・v6598　残りCPUと対話実時間で展開数を制御 | 10,940.22 | −11.65 | 12.90 | 1.794904 | 2.098537 | 1 |

すべての行が102,400注文完走、E=0、不正操作0、全100出力を独立再生。二つのパネルを合わせて1,300出力を再生し、保管した全出力のSHA-256も個票と照合した。各パネルで同一入力の各版には同じCPUを割り当て、順序を回転・反転した。開始後の超過を含め、すべての初回を採用した。CPU供給が変動すると時間依存の探索経路とスコアも変わるため、表の差は同じパネルの対照から計算した。[8版の全個票](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_primary_cases.csv)・[8版の全結果](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_primary_summary.csv)・[追加5版の全個票](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_late_cases.csv)・[追加5版の全結果](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_late_summary.csv)・[評価手順](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/notes/handoffs/20261106_v65_final_protocol.md)

## 作業中に公開された既存系列の照合

別系列から公開された [v6576 二設計・三経路](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/src/bin/v6576_two_heads_three_routes.cpp) の Git blob が `7f00067e6d8bf50b1e3b1fbca2fb0ce7ae7afc1b` であることを確認し、同じファイルを非LOCALでビルドした。独立監査に続き、専用の二版パネルで公式100件を同じCPU・順序均衡で比較した。v6576は **10,953.09**、同時測定のv6417は **10,969.77**、差−16.68手（対応SE15.86）。両版とも全E0・不正0、全出力再生、CPU・対話実時間とも全件2秒以内。最大CPUと実時間はv6576が1.473155／1.553115秒、v6417が1.847472／1.902079秒。現在環境での時間余裕と、過去のv6417の最小絶対記録を両方示す。[公式個票](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_external_cases.csv)・[集計](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_external_summary.csv)

## 選抜に使わなかった64表

最終選抜前に乱数種とマニフェストを記録した別の商品表・注文列を使った。全ての版が65,536注文を完走し、E0・不正0・全64出力再生一致。

| 版 | 平均点 | 対v6417の対応差 | 対応SE | 最大CPU秒 | 最大対話秒 |
|---|---:|---:|---:|---:|---:|
| v6417 | 11,029.31 | 0 | — | 1.833580 | 1.895751 |
| v6514 | 11,004.38 | −24.94 | 19.00 | 1.831869 | 1.931876 |
| v6576　作業中に別系列から公開 | 10,978.44 | −50.88 | 25.81 | 1.451712 | 1.485898 |
| v6592 | 10,953.66 | −75.66 | 25.11 | 1.817711 | 3.343390 |

v6592の実時間超過は1件。上の比較から公式100件の順位を決め直していない。v6576は別セッションの公式100件で平均10,981.87、最大CPU1.434707秒・実時間1.870698秒、同セッションの独立人工24表では対照比+33.38手だった。[今回の独立表の個票](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_blind_cases.csv)・[集計](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_blind_summary.csv)・[別系列のv6576ノート](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/notes/experiments/v6576.md)

## どの変更が効いたか

- 1窓を独立二色紙に分けたv6460は、残り63窓で平均75.33手を節約したが、切替に493.83手かかり、人工6表で+418.50手。v6529は主紙にもその窓を印字し、二色紙の位置をDPで決めた。同じ人工8表の追加手数は492.625→164.125、出発版との差は+58.50手だった。6窓の制御紙を廃して実64窓を通常の192枚で表示するv6593は、公式100件で11,447.41手、対照11,272.95手、全E0・不正0。一部の窓を外しても、主紙と安く取得できる第二紙の組で残窓が減らなければ全体は伸びない。[v6593の100件](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_native100.csv)・[側面系列の調査](20261106_v65_side_panel.md)
- 第二の合法な先頭紙を与える編集は、独立32表の18,025候補から全商品可達・対象の把持改善を満たした23組まで絞った。対象商品と変更紙の元担当までなら8組で純改善。全256商品の本番相当探索を同じ4棚から行うと23組とも14〜489手悪化し、同じ前商品を表示する棚だけに絞っても改善0組だった。[紙の検証](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/notes/handoffs/20261106_v6469_cells.md)・[全商品比較](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/v6583_study/arm_scores.csv)
- 残窓集合が要求する正色紙とその把持遮蔽を使った探索は、96の同一開始棚で観測済み1,051→1,017手。全把持・表示を再生した。同じ棚から時間打切りを外して既存v6417を完走させると1,014手で、新方式にしかなかった短縮は1注文1手。短縮15状態から8列×4注文を実行したところ、退避紙を底へ送ると後続4注文が平均71.75手、同じ移動集合で元の棚順位に近く戻すと43.96手だった。[物理証明と再現手順](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/notes/handoffs/20261106_v6531_proof.md)・[同じ棚の3経路](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/v6531_study/matched_routes.csv)・[4注文の実ロールアウト](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/v6531_study/matched_rollouts.csv)
- v6514は20,480人工注文の段階別短縮とCPU時間から探索の採否を学習し、別12,288注文で検査した。標準LOCALの公式100件は対照比−75.08、非LOCALの別2版比較は−38.89、最終8版比較は−11.33。探索時間が不足する環境ほど利得が大きかった。最終8版では58勝40敗2同点、対応SE19.99であり、再計測での順位は固定されない。[標準LOCAL](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_v6514_standard.csv)・[非LOCAL](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_v6514_nonlocal.csv)
- 物理下界で完成設計を選ぶv6546は未使用32表で+1.91手、商品別に補正したv6577は+0.28手。実行済みの両候補から毎回よい方を選ぶ事後的な余地自体が、未使用32表の平均で34.31手だった。8版の実際の公式出力について、各ケースで最少手数の版を事後選択しても10,877.75点。いずれの事後値も提出時の選択には使っていない。[設計選択の両候補](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_bound_design_arms.csv)・[商品別の独立評価](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_product_gate_audit.csv)

## 次の調査

1. 紙を変える前に、同じ開始棚・同じ候補経路・十分な既存探索で取得費用と残窓修復の下限を測る。今回の96棚では複数窓と把持の緩和下界が平均10.1875手、既存の全探索が10.5625手、目標相当が10.2539手。次は、主紙と第二紙の遮蔽を一緒に減らして下界そのものが下がる紙の配置を探す。
2. 紙を変更して受益する商品だけでなく、全256商品を同じ棚から解く。印字変更で前注文の表示まで変わる場合には、前表示が等しい開始状態でも差を調べる。次注文以降については独立した同一注文列を使い、短縮した紙の再挿入位置まで固定して効果を分ける。

全51件の採否・評価範囲・対照・スコアと、提出用ソースのSHA-256を保存した。固定探索量と実時間の判定は各表の評価種別で識別できる。[実験台帳](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_experiments.csv)・[ソース一覧](https://github.com/masakiaota/AHC074/blob/f97d5afb40be100ecbd991198a94d548dff0d5ce/results/joint65_sources.csv)
