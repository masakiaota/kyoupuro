# Journal

実験の正史である。
各実験は一つのエントリとして時系列で記録し、書式と更新規則は `AGENTS.md` の「実験知見の記録」に従う。

見出しの状態は、実験結果を現在どのように利用できるかを表す。
本文の「当時の判定」は、評価時点の採否を表し、現在状態が変わっても書き換えない。

- **現行採用**：現行solverまたは現在使う補助実装である。
- **後続への統合**：中心機構が明示的な系譜を通じて現行solverへ残っている。
- **知見のみ有効**：実装は使わないが、得られた結論を今後の判断に使える。
- **条件付き再検討**：見出しまたは学びに示した再開条件を満たす場合だけ再検討する。
- **未決着**：実験中、機構未確認、または証拠不足により結論を出せない。

`系譜` の `base` は実装上の主な出発点、`imports` は別実験から明示的に取り込んだ中心機構を表す。
着想が似ているだけの実験や、同等機構の独立検証は `imports` に含めない。

## エントリ

## v001_theory_admission — 知見のみ有効: 理論負荷連動admissionの独立検証として有効

系譜: series=foundation; base=-; imports=[]
当時の判定: 採用。
仮説: 生成分布から逐次推定したケース負荷に応じてセル時間価値の採択閾値を変えると、低価値な長期滞在による容量浪費を抑えられる。
変更: v000_template を基準に、移動なし・最小境界長形状の決定的配置を共通経路として実装し、理論負荷連動の admission だけを中心アイデアとして追加する。
機構確認: `TraceStats` の `price_reject` と `placed` がともに正、`theta_update` が全到着で発動し、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが 10,000,000 以上で、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 47,256,006。`price_reject=24452`、`placed=51247`、`theta_update=100000`、`fallback_count=0` で採否基準を満たした。
学び: 理論選別後も `geometry_reject=24301` と価格棄却に匹敵し、次の改善対象は価値閾値の緩和ではなく自由領域を壊しにくい配置である。

## v002_temporal_packing — 知見のみ有効: 退去時刻の近接配置による正効果の独立検証として有効

系譜: series=foundation; base=v001_theory_admission; imports=[]
当時の判定: 採用。
仮説: 退去時刻が近いグループを隣接配置すると、退去後の空き領域がまとまり、価値閾値通過後の幾何棄却が減ってスコアが上がる。
変更: v001_theory_admission を基準に、admission・最小周長形状・移動なしを固定し、配置候補を退去時刻差で評価する。候補列挙は v000 と同じ local 時間比率を使い、turn ごとの累積締切まで行う。
機構確認: `TraceStats` の `temporal_choice_changed`、`temporal_contact_edges`、`search_time_cutoff` がすべて正で、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが 47,256,006 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 51,987,123（v001 比 +10.012%）。`temporal_choice_changed=26036`、`temporal_contact_edges=107527396`、`search_time_cutoff=1168`、`fallback_count=0` で採否基準を満たした。
学び: admission 数を変えず配置成功が 1,734 件増え、退去時刻の空間的凝集が後続の配置可能性を高めると確認した。候補形状数の途中検証は [deep/v002_temporal_packing.md](deep/v002_temporal_packing.md) に残す。

## v501_pro_shadow_packing — 後続への統合: 現行solverの基盤として存続

系譜: series=foundation; base=-; imports=[]
当時の判定: 採用。
仮説: 生成分布に基づく動的シャドー価格と、空き領域の断片化を抑えるコンパクト配置、採算の合う局所再配置を統合すると、v001 より高い価値を限られた芝生へ詰められる。
変更: `picnic_solver.cpp` を基準に、アルゴリズムを変えず Rust へ移植し、時間閾値だけを `PROGRAM_TIME_LIMIT_SEC` 比へ変換して `TraceStats` を追加する。
機構確認: `TraceStats` の `normal_placed` と `fragment_evaluated` がともに正で、`relocation_attempt` と `relocation_success` の発動数、`growth_placement_success` の利用数を確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが v001_theory_admission の 47,256,006 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 62,136,847（v001 比 +31.490%）、最大 1,222 ms。`normal_placed=62944`、`fragment_evaluated=960714`、`relocation_attempt=17618`、`relocation_success=996`、`growth_placement_success=33003`、`fallback_count=0` で基準を満たした。
学び: 統合手法は v002 も +19.524% 上回ったが、各部品の寄与はこの実験だけでは分離できないため、個別に再利用・調整する前にはアブレーションが必要である。部品分析は [deep/v501_component_analysis.md](deep/v501_component_analysis.md) に残す。

## v003_perimeter_slack — 知見のみ有効: 採算制約付き周長余裕の正効果を示す知見として有効

系譜: series=foundation; base=v002_temporal_packing; imports=[]
当時の判定: 棄却。
仮説: 最小周長形状が入らない場合でも、機会費用を上回る範囲で周長を増やせば、利用料の低下より幾何棄却の削減が勝ってスコアが上がる。
変更: v002_temporal_packing を基準に、admission・退去時刻評価・移動なしを固定する。`L_min` の配置失敗後だけ `L_min+2`、`L_min+4`、`L_min+6` を順に試し、実際の compactness による価値密度が閾値以上の形だけを採用する。
機構確認: `TraceStats` の `slack_attempt` と `slack_placed` がともに正で、`actual_fee_loss` が記録され、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが v002_temporal_packing の 51,987,123 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、v002 比 +4.552%（91 勝 9 敗）で事前登録基準を満たした。`slack_attempt=68529`、`slack_placed=3331`、`actual_fee_loss=48771787`、`fallback_count=0`。評価中に追加された全体最高更新条件では v501 比 -12.526%（8 勝 92 敗）のため棄却した。
学び: 採算制約付き周長余裕には単独の正効果があるが、v002 系を主力として前進させるには不足する。以後は v501 を基準に全体最高更新を狙い、この部品は統合候補として扱う。詳細は [deep/v003_perimeter_slack.md](deep/v003_perimeter_slack.md) に残す。

## v004_component_aware_choice — 条件付き再検討: 残余成分価値を価格へ直接入れる場合に再検討

系譜: series=foundation; base=v501_pro_shadow_packing; imports=[]
当時の判定: 棄却。
仮説: 空間評価で最良の配置が価格判定に落ちても、同じ到着内の別候補が空き成分別の閾値を満たす場合があるため、採算を満たす候補を探索中に優先すれば低機会費用の空間を活用できる。
変更: v501_pro_shadow_packing を基準に、admission 式・候補生成・空間評価・局所再配置を固定する。通常配置で最初の空間最良候補が採算を満たさない場合だけ、同じ周長および既存の周長範囲から成分別閾値を満たす候補を選ぶ。該当候補がなければ v501 と同じ候補を返す。
機構確認: `TraceStats` の `component_price_rescue` が正で、`component_price_same_level` または `component_price_later_level` が正、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v501_pro_shadow_packing の 62,136,847 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功。最良の2%余裕版は `component_price_rescue=676`、同一周長662件、後続周長14件、`fallback_count=0` で発動したが、v501 比 -0.045%（43勝13分44敗）で基準未達だった。余裕なしは -0.126%、5%は -0.228%だった。
学び: 成分別価格を候補選択へ入れる効果はほぼ中立で、薄い採算余裕を除く方向は改善したが全体最高を更新しない。再開条件: 配置後の残余成分価値を価格へ直接入れる場合。詳細は [deep/v004_component_aware_choice.md](deep/v004_component_aware_choice.md) に残す。

## v005_shape_diversity — 後続への統合: 拡張した規則形状集合が現行solverへ存続

系譜: series=foundation; base=v501_pro_shadow_packing; imports=[]
当時の判定: 採用。
仮説: v501 が各周長で捨てている規則形状を既存候補の上位集合として追加すれば、同じ周長と利用料のまま空き領域に適合する候補が増え、幾何棄却と断片化が減ってスコアが上がる。
変更: v501_pro_shadow_packing を基準に、admission・候補評価・成長配置・局所再配置を固定する。各周長の既存14形状をすべて残し、形状上限だけを20へ増やす。
機構確認: `TraceStats` の `shape_variants_kept` が v501 相当の9,303を上回り、`extra_shape_chosen` と `extra_shape_placed` がともに正、`fallback_count` が0であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v501_pro_shadow_packing の 62,136,847 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、v501 比 +0.427%（62勝38敗）、最大1,207 ms。各ケースの `shape_variants_kept=11989`、合計 `extra_shape_chosen=7674`、`extra_shape_placed=6869`、`fallback_count=0` で採否基準を満たした。
学び: 追加形状は幾何棄却を減らさず、規則形状を成長配置より先に見つけて配置後価格棄却を減らすことで改善した。以後の主力基準は v005 とする。詳細は [deep/v005_shape_diversity.md](deep/v005_shape_diversity.md) に残す。

## v005_move_rescue — 後続への統合: 悪形配置を比較するrelocation主経路が現行solverへ存続

系譜: series=foundation; base=v501_pro_shadow_packing; imports=[]
当時の判定: 採用。
仮説: v501 の主損失は悪形受け入れによる C 低下（価値加重 C/C_max=0.805、slack>=8 由来が約 1,430M）と断片化棄却であり、これらのターンに既存グループの移動で最小周長近傍の配置を作り、悪形配置との純利益比較で採用すれば、移動費より利用料の増加が上回る。
変更: v501_pro_shadow_packing を基準に、admission・形状集合・断片化評価・通常配置探索を固定する。relocation の発動を「通常配置が不在または slack>=4」へ拡大して悪形受け入れ成立時にも比較し、純利益差（移動費・blocker の C 低下損失込み）で移動プランを採用する。blocker の逃げ先に worst_perimeter+2 を損失込み採算で許可し、空板スキャンを前計算し、relocation 系の時間上限を後ろへ動かす。
機構確認: `TraceStats` の `reloc_beats_normal`（悪形配置を移動が上回った回数）が正、`relocation_success` が v501 の 996 回/100 ケースから大幅増、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v501_pro_shadow_packing の 62,136,847 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 63,713,999（v501 比 +2.538%、並行採用の v005_shape_diversity 比 +2.102%）で全体最高を更新、最大 1,233ms。機構確認は 0002/0077 実測で `reloc_beats_normal=14/24`、`relocation_success=21/46`、`fallback_count=0`、出力リプレイで移動 15→56 回/ケースを確認した。
学び: 改善源は悪形受け入れの良形置換で、価値加重 C/C_max は 0.805→0.831、realize は 0.794→0.816 に上がった。一方、断片化棄却の移動救済はネット中立（util 0.616→0.613）で、救済で埋めた空間は後続の配置機会と相殺する。残る伸びしろは C 損失約 1,100M と repack 成功率 5〜6% の改善、相殺されない形の断片化予防にある。

## v006_shape_move_combo — 後続への統合: 形状多様化と移動主経路の統合が現行solverへ存続

系譜: series=foundation; base=v005_move_rescue; imports=[v005_shape_diversity]
当時の判定: 採用。
仮説: v005_shape_diversity の形状上位集合（周長ごと上限20）と v005_move_rescue の移動主経路化は、規則形状の適合性と悪形置換という独立の損失源に効くため、統合すれば両方の改善がほぼ加算的に得られる。
変更: v005_move_rescue を基準に、v005_shape_diversity の形状生成（既存14形状を保つ上限20）と形状トレースだけを取り込む。admission・移動・断片化評価・時間予算は v005_move_rescue のまま。
機構確認: `TraceStats` の `extra_shape_chosen` と `extra_shape_placed` が正、`reloc_beats_normal` が正、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v005_move_rescue の 63,713,999 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 63,875,066（v005_move_rescue 比 +0.253%）で全体最高を更新、最大 1,355ms。0002/0077 実測で `extra_shape_chosen=46/–`、`extra_shape_placed=38/54`、`reloc_beats_normal=13/–`、`fallback_count=0` を確認した。
学び: 加算を期待した +0.427% 分は +0.253% に目減りした。形状多様化と移動置換はどちらも悪形受け入れの削減に効くため部分的に競合する。max_elapsed が 1,233→1,355ms に増えており、以後の実験は時間予算の残りに注意する。

## v007_plan_quality — 後続への統合: attempt深化と複数プラン収集が現行solverへ存続

系譜: series=foundation; base=v006_shape_move_combo; imports=[]
当時の判定: 採用。
仮説: 移動プランを最初の採算成功で採用すると rank 下位 target の採算ぎりぎりの移動が盤面を悪化させる（attempt 12 で 0077 +1.6% / 0002 -4.0% の実測）。採算を制約とし、repack 後盤面の断片化メトリック最小でプランを選べば、深い探索の利益だけを取れる。
変更: v006_shape_move_combo を基準に、attempt_relocation を「attempt 上限 12・採算を満たす成功プランを最大 3 つ収集し、repack 後の断片化メトリックが最小のプランを採用」に変える。採算式・repack・時間予算・形状・admission は不変。
機構確認: `TraceStats` の `reloc_plan_collected` が正で、`reloc_plan_switched`（最初の成功と異なるプランを選んだ回数）が正、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v006_shape_move_combo の 63,875,066 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 63,904,848（v006 比 +0.047%）で基準を満たした。実装中の調整でメトリック単独選択は 0077 -1.0% となり、選択基準は net − 800×metric の複合、attempt 6 では両ケース劣化のため attempt 12 で確定。`reloc_plan_collected=43/72`、`reloc_plan_switched=8/14`（0002/0077）、`fallback_count=0`。
学び: プラン収集・複合選択の寄与は attempt 深化とセットで辛うじて正になる程度で、0002 +3.7% と 0077 -0.5% のようにケース間で相殺する。局所的な断片化メトリックは移動プランの将来価値の良い代理ではなく、移動置換の量を増やす方（repack 成功率、Q4）が本命と判断する。

## v008_flexible_escape — 後続への統合: 損失を価格化した柔軟な逃げ先探索が現行solverへ存続

系譜: series=foundation; base=v007_plan_quality; imports=[]
当時の判定: 採用。
仮説: repack 失敗の 79〜99% は「最大空き成分 >= P なのに worst+2 以下の周長で置ける形がない」形状不足であり（Q4 probe 実測）、逃げ先の周長許容を広げて C 低下損失込みの採算に任せれば、repack 成功と移動置換の適用数が増えて C 回収が進む。
変更: v007_plan_quality を基準に、repack の規則形状探索を worst_perimeter+6 までの段階探索へ広げ、growth 逃げ先の周長上限を撤廃する。fee_loss 計上と採算式・プラン選択は不変。時間超過対策で relocation 系締め切りを 4/190 前倒し、予算比を 0.60 とする。
機構確認: `TraceStats` の `repack_success` が v007 系 probe 実測（0002: 45/985）から増加し、`reloc_fee_loss` が増加、`relocation_success` が増加、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v007_plan_quality の 63,904,848 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 66,036,608（v007 比 +3.336%、v501 比 +6.276%）で全体最高を大幅更新、最大 1,347ms。0002/0077 実測で `repack_success=806/887`（成功率 100%/55%、v007 系は 5%）、`relocation_success=41/48`、`reloc_fee_loss=0.9M/2.3M`、`fallback_count=0`。
学び: 移動置換のボトルネックは探索の深さでも採算でもなく、blocker 逃げ先の品質制約だった。「損失は禁止でなく価格に載せて市場に任せる」設計が repack を 10〜20 倍通し、+3.3% を生んだ。0002 は -1.0% と悪化しており、fee_loss の大きい逃げが後続の盤面を悪化させる副作用は残る。

## v009_wider_blockers — 条件付き再検討: target rankを純利益期待値で再設計する場合に再検討

系譜: series=foundation; base=v008_flexible_escape; imports=[]
当時の判定: 棄却。
仮説: 逃げ先制約の撤廃（v008）で repack がほぼ通るようになった今、target 列挙時の blocker_limit=4 が移動置換の適用機会（特に大きな空間を要する高価値グループ）を制限しており、上限を 6 に上げれば適用が増えてスコアが上がる。
変更: v008_flexible_escape を基準に、collect_relocation_targets の blocker_limit を 4→6 にするだけ。逃げ先探索・採算・プラン選択・時間予算は不変。
機構確認: `TraceStats` の `moved_groups`/`relocation_success` 比（1 成功あたり移動数）が増え、`relocation_success` が減らず、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v008_flexible_escape の 66,036,608 を上回り、上記の機構確認を満たしたら採用する。
結果: dry-run 100 ケースで 65,933,163（v008 比 -0.157%）の基準未達。機構確認も 1 成功あたり移動数 1.32→1.35 とほぼ不変のまま 0077 で `relocation_success` 48→37 に減少し、条件を満たさなかった。
学び: rank 上位に来る target はほぼ blocker<=4 であり、上限拡大は候補列挙と repack の時間コストだけを増やして成功数を減らす。適用機会の次の制限は blocker 数ではない。再開条件: target rank を純利益期待値ベースに作り直し、多 blocker target が上位に来る場合。

## v010_budget_retune — 後続への統合: relocationの時間予算として現行solverへ存続

系譜: series=foundation; base=v008_flexible_escape; imports=[]
当時の判定: 採用。
仮説: v008 の TLE 対策（予算比 0.60・締切 4/190 前倒し）は保守的すぎ、0077 実測の `reloc_budget_hit=61` が示すとおり高負荷ケースで移動置換の適用数を制限している。実測 max 1,347ms の余裕内で予算を戻せば適用が増えてスコアが上がる。
変更: v008_flexible_escape を基準に、時間定数のみ変更する。RELOC_TIME_BUDGET_RATIO 0.60→0.68、TARGET_SCAN/GROWTH/REPACK の締切を 172/174/176→176/178/180 (/190) に戻す。探索・採算・プラン選択は不変。
機構確認: `TraceStats` の `reloc_budget_hit` が 0077 で減少し、`relocation_attempt`/`relocation_success` が増加、`fallback_count` が 0、eval の max_elapsed が 1,450ms 未満であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v008_flexible_escape の 66,036,608 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 66,154,932（v008 比 +0.179%）で全体最高を更新、最大 1,366ms < 1,450ms。0077 実測で `reloc_budget_hit` 61→47、`relocation_success` 48→52、0002 は不変、`fallback_count=0`。
学び: 時間予算はまだ高負荷ケース (0093 で budget_hit=144) の制約であり、残る適用拡大は per-repack コストの削減（探索の効率化）が必要。予算比 0.68・締切 180/190 が現構成の実測安全上限に近い。

## v011_deep_levels — 知見のみ有効: growthに落ちる空きは深い規則形状でも受からない

系譜: series=foundation; base=v010_budget_retune; imports=[]
当時の判定: 棄却。
仮説: v008 系でも slack>=8 の C 損失が約 1,090M 残り、その多くは growth（自由形、slack 20〜40 になりがち）由来である。growth に落ちる前に L_min+8/+10 の規則形状レベルを試せば、同じ受け入れをより良い C で実現できる。
変更: v010_budget_retune を基準に、形状生成の周長上限を L_min+8→L_min+12 へ、通常探索の周長範囲を L_min+6→L_min+10 へ広げる。fast_mode・repack・採算・時間予算は不変。
機構確認: `TraceStats` の `deep_level_placed`（slack>=8 の規則形状での通常配置）が正、`growth_placement_success` が v010 より減少、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v010_budget_retune の 66,154,932 を上回り、上記の機構確認を満たしたら採用する。
結果: dry-run 100 ケースで 66,063,412（v010 比 -0.138%）の基準未達。`deep_level_placed` は 0002/0077 で 2/5 回とほぼ発動せず、growth 成功はほぼ不変、program_elapsed は +46〜56ms 増えた。
学び: 規則形状 L_min..+6 で受からず growth に落ちる空きは不規則すぎて、L_min+8/+10 の規則形状でも受からない。深いレベルの形状追加はスキャンと clone のコストだけを増やして relocation 予算を圧迫する。slack>=8 損失の削減には growth 自体の形状品質改善か、移動側の適用拡大が必要である。

## v012_rollout_choice — 後続への統合: 移動プランの短期rollout比較として現行solverへ存続

系譜: series=foundation; base=v010_budget_retune; imports=[]
当時の判定: 採用。
仮説: 移動プランの選択と採否を断片化メトリックの代理で評価するのは限界がある（v007 の学び）。生成規則と θ 推定から将来到着を共通乱数でサンプルし、プラン適用後盤面の将来受け入れ価値をロールアウトで直接評価すれば、rescue の機会費用（後続との相殺）も自動的に織り込まれ、選択と採否の質が上がる。
変更: v010_budget_retune を基準に、attempt_relocation の「プラン間選択」と「最終採否（最良プラン vs baseline）」を、即時項（V×C − 移動費 − fee_loss）+ 共通乱数 K 本ロールアウトの将来項の比較へ置き換える。プラン収集の採算フィルタ・repack・通常配置・admission は不変。
機構確認: `TraceStats` の `rollout_session` が正、`rollout_flip`（メトリック複合評価と異なる判断をした回数）が正、`rollout_reject`（採算通過プランをロールアウトが棄却した回数）を記録し、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v010_budget_retune の 66,154,932 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 66,632,207（v010 比 +0.721%）で全体最高を更新、最大 1,433ms。0002/0077 実測で `rollout_session=46/54`、`rollout_flip=32/26`、`rollout_reject=16/9`、rollout 時間 26〜34ms、`fallback_count=0`。0002 は 31,327,967 とケース過去最高を更新した。
学び: K=3 本・到着 22 件の短い水平線が最良で、K=5・28 件に増やすと両確認ケースで悪化した。将来項は水平線を伸ばすほど分散が即時項の確実な差を薄めるため、ロールアウトは「近い将来の差分検出器」として使うのが正しい。採算通過プランの 2〜3 割をロールアウトが棄却しており、シャドー価格採算は移動を過剰採用していた。

## v013_departure_field — 条件付き再検討: 壁距離等高線と移動による場維持を組み合わせる場合に再検討

系譜: series=foundation; base=v012_rollout_choice; imports=[]
当時の判定: 棄却。
仮説: v002 系の退去時刻近接評価は隣接セルだけの局所評価であり、盤面大域の構造（奥=長期滞在、手前=短期滞在）を作らない。幾何勾配場 field[cell]=(x+y)/(2(N−1)) とグループの滞在分布位置 f(dur)=1−exp(−dur/θ) の一致ボーナスを配置評価へ加えれば、退去の空間的まとまりが大域的に生まれ、空きの断片化がさらに減る。
変更: v012_rollout_choice を基準に、build_weight_data へ場一致ボーナス（w=6.0、free セルへ −w×|field−f|）を追加し、通常配置の探索でのみ有効化する（repack・permanent は場なし）。移動・admission・ロールアウトは不変。
機構確認: `TraceStats` の `field_bonus_turns` が正で、`fallback_count` が 0 であることを確認し、比較用に w=0 相当の v012 と配置分布が変わることをスコア差で見る。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v012_rollout_choice の 66,632,207 を上回り、上記の機構確認を満たしたら採用する。
結果: 機構は発動（`field_bonus_turns=839/737`、`fallback_count=0`）したが、w=6.0 で 0002 -1.9%・0077 -1.0%、w=2.5 でさらに悪化し、w=2.5 の dry-run 100 ケースは 66,433,890（v012 比 -0.298%）で基準未達。
学び: 全周の壁際が優良地であるこの問題で単一方向の勾配場を強制すると、右下側の壁際が長期グループから遠ざかり充填が乱れる。退去時刻の空間的まとまりは局所近接評価（v002 系）で既に取れており、大域場の追加は干渉が勝つ。再開条件: 場を壁距離ベースの等高線にし、移動も場の維持に使う場合。

## v014_greedy_growth — 後続への統合: 共有辺優先growthとして現行solverへ存続

系譜: series=foundation; base=v012_rollout_choice; imports=[]
当時の判定: 採用。
仮説: growth 配置の悪形（slack 20〜40、価値加重損失 ~1,000M）は、成長優先度が中心距離だけで共有辺数を見ないことによる。性質2（L += 4−2d）に従い共有辺数 d 最大のセルを優先して成長すれば、同じ空きでもより小さい周長の形が得られ、受け入れ形状の品質が上がる。
変更: v012_rollout_choice を基準に、growth_placement のフロンティア選択を「共有辺数 d 最大（pop 時に d を再評価する lazy heap）、タイブレークに従来の ring/manhattan/attraction」へ変える。seed 選定・断片化評価・repack・admission・ロールアウトは不変。
機構確認: `TraceStats` の `growth_slack_sum`（生成された growth 候補の L−L_min 合計）/`growth_placement_success` の平均 slack が v012 実測より下がり、`fallback_count` が 0 であることを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v012_rollout_choice の 66,632,207 を上回り、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 66,783,368（v012 比 +0.227%）で全体最高を更新、最大 1,359ms、`fallback_count=0`。平均 slack は 0077 で 35.5→33.5 と低下したが 0002 は 40.2→40.9 と微増で、低下は部分的。実装初版の pop 時 lazy 再評価は機構不発（結果が v012 と完全一致）で、選択時の最新 d 再挿入方式に修正して発動した。
学び: キーが改善する方向の decrease-key は pop 時 lazy 評価では機能せず、値が変わった瞬間の再挿入が必要（ヒープに埋もれた古いキーは浮上しない）。d 優先グリーディは growth が呼ばれる不規則な空きでは周長を大きくは削れず、形の傾向変化として +0.2% 程度に留まる。growth 悪形の残損失は成長規則でなく「呼ばれる状況の空きの悪さ」が支配的で、削減には空きを整える側（予防・移動）の機構が要る。

## v015_quick_repack — 条件付き再検討: quickをbeam候補化し全体の律速を先に削減できた場合に再検討

系譜: series=foundation; base=v014_greedy_growth; imports=[]
当時の判定: 棄却。
仮説: 容易な repack を逐次 quick-first で確定し、失敗・不採算時だけ既存ビームへ戻せば、再配置品質を大きく落とさず per-repack 時間を削減でき、時間予算枯渇ケースの移動適用数とスコアが増える。
変更: v014_greedy_growth を基準に、blocker を現行順のまま `L_min` / `L_min+2` の先頭形状へ逐次配置する quick 経路を repack 冒頭へ追加する。全員を置けて既存採算を満たす場合だけ使用し、失敗・不採算時は初期盤面から既存ビームを変更せず実行する。
機構確認: `repack_quick_attempt`, `repack_quick_used`, `repack_beam_fallback` が正で、0077・0093の repack 平均時間と `reloc_budget_hit` が低下し、`fallback_count=0` であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高 v014_greedy_growth の 66,783,368 を上回り、最大経過時間が 1,450ms 未満で、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 66,738,256（v014 比 -0.068%）で最高更新に失敗、最大 1,361ms、`fallback_count=0`。quick は 6,412/150,315 repack（4.27%）で使用され、`repack_beam_fallback=143,903`。`reloc_budget_hit` は 2,184→2,008 に減ったが、総 relocation 時間は 65.08s→65.22s、移動成功は 6,998→6,915 だった。
学び: repack の大半は quick で解けず、失敗コストを上乗せした後に既存ビームを走らせる。成功時も first-fit でビームの配置品質を捨てるため、時間短縮はほぼなく判断品質だけが下がった。再開条件: quick を置換経路でなくビーム候補の一つとして使い、target scan を含む relocation 全体のボトルネックを先に削減できた場合。

## v016_low_r_slack2 — 条件付き再検討: 増分利益と計算時間で移動候補を順位付けできる場合に再検討

系譜: series=foundation; base=v014_greedy_growth; imports=[]
当時の判定: 棄却。
仮説: `R<=0.02` では移動費が安いため、通常受理済みの `L=L_min+2` でも、既存 relocation が作る最小周長近傍の配置による利用料増加が移動費と将来機会費用を上回る場合がある。現行は `slack>=4` だけを呼ぶため、この利益を取りこぼしている。
変更: v014_greedy_growth を基準に、`R_milli<=20` かつ通常配置を受理済みで `L=L_min+2` の場合を relocation 発動候補へ加えるだけとする。target・repack・採算式・ロールアウト・admission・時間予算は変えない。
機構確認: `low_r_slack2_eligible`, `low_r_slack2_attempt`, `low_r_slack2_success` が正で、0006・0093における発動と時間予算への影響を確認し、`fallback_count=0` であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高 v014_greedy_growth の 66,783,368 を上回り、`R<=0.02` の20ケース合計が基準 1,365,900,910 を上回り、最大経過時間が 1,450ms 未満で、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、全体平均は 66,826,651（v014 比 +0.065%）、最大 1,363ms、`fallback_count=0`。機構は `eligible/attempt/success=473/473/169` と発動したが、対象の低 R 20ケース合計は 1,364,572,654（基準比 -0.097%）で採用条件を満たさなかった。対象外80ケースの見かけ上の増分 +5,656,525 は時間打ち切り差で、65ケースは完全同点だった。
学び: `R` が下げるのは移動費であり探索時間ではない。slack 2 の追加呼び出しで低 R 20ケースの relocation attempt は 6,564→7,042、budget hit は 685→827、総時間は +382msとなり、後続のより価値ある移動機会まで圧迫した。再開条件: 発動を `R` だけで広げず、増分利益/計算時間で優先順位を付けられる場合。

## v017_normal_rollout — 後続への統合: 通常配置の短期rollout比較として現行solverへ存続

系譜: series=foundation; base=v014_greedy_growth; imports=[]
当時の判定: 採用。
仮説: 通常配置の現行評価 `cheap_score - 1.4*fragment_delta` は将来盤面価値の代理にすぎない。同一周長・同一 admission 閾値の座標候補を短期ロールアウトで直接比較すれば、即時利用料と受け入れ可否を変えず、後続を置きやすい座標を選べる。
変更: v014_greedy_growth を基準に、最初に見つかった周長レベルと上位20候補を固定する。現行評価最大を候補0とし、同じ `component_size` 内の局所重み最大・断片化増分最小を重複除去して最大3候補とする。受理済み・非 fast mode・2候補以上の場合だけ既存と同じ K=3・22到着の共通乱数ロールアウトで座標を選ぶ。growth・周長・admission・relocation は変えない。
機構確認: `normal_rollout_session`, `normal_rollout_candidate_sum`, `normal_rollout_flip` が正で、0002・0077における発動回数と `normal_rollout` 時間を確認し、`fallback_count=0` であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高 v014_greedy_growth の 66,783,368 を上回り、最大経過時間が 1,450ms 未満で、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 67,106,996（v014 比 +0.485%）で全体最高を更新、69勝31敗、最大 1,370ms、`fallback_count=0`。`normal_rollout_session=19,656`、候補平均2.12、`normal_rollout_flip=3,438`（17.5%）、総時間7.27sだった。
学び: 周長・即時利用料・admission 閾値が同じでも座標が作る将来盤面価値には差があり、現行の局所重みと断片化メトリックだけでは捉え切れない。受け入れ数は 63,263→63,178 と増えずにスコアが上がったため、改善源は件数でなく後続の価値・形状品質である。一方、平均約73ms/ケースの追加計算で fast mode ターンが148→1,834に増えており、次は判断精度を保った選択的発動が時間面の伸びしろになる。

## v018_no_move_boundary — 条件付き再検討: growth形状を移動なしで直接改善できる場合に再検討

系譜: series=no_move; base=v017_normal_rollout; imports=[]
当時の判定: 棄却。
仮説: 再移動を禁止した固定配置では、候補領域が増やす空き領域境界の時間積分を最小化すると、早く空く隣接領域の内側へ長期グループを残す配置を避け、後続が使えるまとまった空きを維持して全体最高を更新できる。
変更: v017_normal_rollout を基準に、全ターンの移動出力を0へ固定し、relocation関連の状態・探索・出力を削除する。通常配置の局所重みだけを、領域周長×滞在時間から池・外周および利用中領域との共有辺が有効な時間の2倍を引く増分量に置き換える。admission・形状集合・growth・断片化評価・通常配置ロールアウトは維持する。
機構確認: `move_zero_output=100000`、`integrated_contact_edges`、`integrated_contact_time`、`normal_rollout_session`、`normal_rollout_flip` が正で、移動出力が全ケースで常に0、`fallback_count=0` であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高 v017_normal_rollout の67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均63,377,535（v017比 -5.557%、既存の再移動なし最高v003比 +16.603%）、最大747ms。`move_zero_output=100000`、接触辺20,509,508、通常ロールアウト22,207回中3,843反転、`fallback_count=0`で機構は発動したが最高更新に失敗した。
学び: 受入数はv017より1,089件多い一方、理想利用料比は0.818（v017移動後0.863）、初期slack>=8は137.9件/ケース（同95.1件）であり、差は件数でなくgrowth由来の形状品質に集中する。再開条件: growth形状を移動なしで直接改善できる場合。

## v019_no_move_growth_sa — 条件付き再検討: 多セル近傍か将来盤面比較を時間内に実装できる場合に再検討

系譜: series=no_move; base=v018_no_move_boundary; imports=[]
当時の判定: 棄却。
仮説: 再移動なし系列で規則形状が入らないとき、greedy growth完成形へ連結性を保つ1セル交換の焼きなましを行えば、同じ空き成分から周長の短い領域を取り出せて利用料実現率が上がり、全体最高を更新できる。
変更: v018_no_move_boundaryを基準に、移動なし・admission・規則形状・時間積分境界重み・断片化評価・通常配置ロールアウトを固定する。`growth_placement`が作った上位完成形だけを、1セル削除後も連結か確認し、残り領域へ隣接する空き1セルを追加する固定サイズ近傍で時間制限付き焼きなましする。
機構確認: `growth_sa_session`、`growth_sa_iteration`、`growth_sa_improved`、`growth_perimeter_reduction` が正で、`growth_slack_sum/growth_placement_success` がv018実測32.739より下がり、`move_zero_output=100000`、`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 候補2×512反復へ調整後、100/100ケース成功、平均64,365,010（v018比 +1.558%、83勝17敗、v017比 -4.086%）、最大1,344ms。平均growth slackは29.890、`session/iteration/improved/reduction=58,626/30,016,512/181,726/151,298`、移動0、`fallback_count=0`で機構は発動した。
学び: 1セル交換による周長改善は正だが、初期slack>=8は137.9→134.8件/ケースにしか減らず、理想利用料比も0.818→0.823に留まる。再開条件: 連結・frontier管理を増分化して多セル近傍を試すか、複数growth候補を将来盤面で選べる場合。

## v020_no_move_growth_rollout — 条件付き再検討: 長期幾何余力を評価する比較尺度を用意できる場合に再検討

系譜: series=no_move; base=v019_no_move_growth_sa; imports=[]
当時の判定: 棄却。
仮説: 再移動なしではgrowth完成形の座標選択を後から修正できないため、同じ周長・同じadmission条件の複数growth候補を短期ロールアウトで比較すると、周長改善を保ったまま後続を置きやすい形と位置を選べて全体最高を更新できる。
変更: v019_no_move_growth_saを基準に、移動なし・SA・admission・regular配置・時間積分境界重み・通常配置ロールアウトを固定する。growthの最終候補を1件に潰さず、現行評価最大と同じ周長・同じ空き成分サイズに限定して局所重み最大・断片化増分最小を重複除去した最大3候補を、既存K=3・22到着ロールアウトへ渡す。
機構確認: `growth_rollout_session`、`growth_rollout_candidate_sum`、`growth_rollout_flip` が正で、`growth_slack_sum/growth_placement_success` がv019実測29.890以下、`move_zero_output=100000`、`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均64,268,283（v019比 -0.150%、27勝32敗41同点、v017比 -4.229%）、最大1,373ms。growth rolloutは1,046回中109回反転し、平均growth slack=29.757、移動0、`fallback_count=0`で機構条件は満たしたが最高更新に失敗した。
学び: K=3・22到着の短期rolloutはgrowthの不可逆な形状価値を正しく順位付けできず、初期slack>=8を134.79→135.68件/ケースへ悪化させた。再開条件: growth候補の比較尺度を長期の幾何余力へ変えるか、将来生成を長期化しても時間内に評価できる場合。

## v021_no_move_departure_affinity — 知見のみ有効: 退去時刻クラスタは固定配置を悪化させる

系譜: series=no_move; base=v019_no_move_growth_sa; imports=[]
当時の判定: 棄却。
仮説: 再移動なしでは隣接区画が同時期に解放されることの価値が高く、`T[g]>=T[i]`を同価値とする時間積分境界重みより、`T[g]-T[i]`が小さいほど強く引き合わせる退去時刻親和性の方が、固定配置後にまとまった空きを再生して全体最高を更新できる。
変更: v019_no_move_growth_saを基準に、移動なし・growth SA・admission・規則形状・断片化評価・通常配置ロールアウトと全時間パラメータを固定する。通常配置とgrowthのセル重みだけをv017系の退去時刻差重みへ戻し、壁・池10、遅い隣接`8+8exp(-(T[g]-T[i])/theta)`、早い隣接`2+8exp((T[g]-T[i])/theta)`とする。
機構確認: `departure_affinity_earlier_edges`と`departure_affinity_later_edges`が正で、`move_zero_output=100000`、`fallback_count=0`であることを確認する。v019との差が配置場だけであることもdiffで確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均64,148,000（v019比 -0.337%、38勝62敗、v017比 -4.409%）、最大1,335ms。earlier/later隣接辺は10,364,629/10,001,834、移動0、`fallback_count=0`で機構は発動したが最高更新に失敗した。
学び: 退去時刻親和性は最小周長配置を452.21→450.37件/ケースへ減らし、slack>=8を134.79→136.81件へ増やした。再移動なしでは同時退去クラスタより、占有・静的境界の時間積分を直接小さくするv019の配置場を維持する。

## v022_no_move_growth_lns — 条件付き再検討: 良い初期空間を残した上で大型LNSを組み合わせる場合に再検討

系譜: series=no_move; base=v019_no_move_growth_sa; imports=[]
当時の判定: 棄却。
仮説: v019の形状損失は大型のgrowth悪形へ集中しているため、現行1セルSA後の`P>=64`かつslack>=8候補へ回収可能利用料に応じた複数セルerode-and-regrowを追加すれば、同じ時間内で1セル局所解を越えて周長を縮め、全体最高を更新できる。
変更: v019_no_move_growth_saを基準に、移動なし・1セルSA・admission・規則形状・時間積分境界重み・断片化評価・通常配置ロールアウトを固定する。SA後のgrowth上位候補だけに、連結なcoreが残るよう4/8/16セルを除去し、共有辺優先で再成長する割当なしLNSを追加する。元候補を常にbestとして残し、`V*(C_min-C_current)`が大きい候補へ探索量を優先する。
機構確認: `growth_lns_eligible`、`growth_lns_attempt`、`growth_lns_improved`、`growth_lns_batch_cells`、`growth_lns_perimeter_reduction`が正で、推定`growth_fee_loss_after<growth_fee_loss_before`、平均growth slackがv019の29.890未満、`move_zero_output=100000`、`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 大型40・中型8試行へ配分後、100/100ケース成功、平均64,776,951（v019比 +0.640%、56勝44敗、v017比 -3.472%）、最大1,433ms。LNSは694,468試行で28,164辺を追加削減し、平均growth slack=29.120、推定fee損失before/after=39,913,715,517/38,636,591,183、移動0、`fallback_count=0`だった。
学び: 高価値大型growthへの多セル局所修復は正だが、最終的なslack>=8形状損失はなお12.449M/ケース残る。動的空き成分が小さくなってから直すだけでは全体最高に届かず、過去の小案件が将来の大型compact配置を分断しない予防機構が必要である。

## v023_no_move_slot_calendar — 条件付き再検討: target寸法と到着時点別の将来評価へslot保護を統合する場合に再検討

系譜: series=no_move; base=v022_no_move_growth_lns; imports=[]
当時の判定: 棄却。
仮説: `P>=96,D>=6000,Q>=1`の大型高価値groupが悪形になる主因は過去の固定配置による一時的分断なので、代表大型slotの利用可能時刻をshadow resourceとして守れば、非target groupの即時feeを変えずに将来の最小周長配置を増やし、全体最高を更新できる。
変更: v022_no_move_growth_lnsを基準に、移動なし・LNS・admission・周長段階・growth・SA・時間積分境界・断片化評価を固定する。10x10〜12x13の静的合法な代表大型矩形について各セルownerの最大退去時刻を`ready`とし、早い非重複slotを将来target数だけ選ぶ。非targetの同一周長・同一成分通常候補にslot遅延最小候補を別枠で残し、`max(T-ready,0)`と推定target到着率・悪形損失から算出した期待損失を既存rollout比較へ加える。target自身には課さない。
機構確認: `slot_calendar_turn`、`slot_inventory_sum`、`slot_candidate_added`、`slot_preservation_flip`が正で、比較対象の`slot_delay_after<slot_delay_before`、`move_zero_output=100000`、`fallback_count=0`を確認する。大型targetのslack>=8件数と損失もv022から減ることを出力解析で確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 標準係数の最終確認で100/100ケース成功、平均65,080,345（v022比 +0.468%、v017比 -3.020%）、最大1,436ms。slot比較18,446回中2,977回反転し、delay合計27,755,727→15,345,652、slack>=8は136.14→133.61件/ケース、同損失は12.449M→12.185M/ケース、移動0、`fallback_count=0`だった。係数は半分64,983,490・2倍65,030,936より標準414kが最良だった。
学び: 既知退去時刻から大型空間をshadow resourceとして予防的に守る方向は正である。ただしtargetの寸法・到着時点を区別しない静的矩形inventoryでは回収が約0.47%に留まるため、次は実際の将来時点で解放済みセルを除いた盤面のcompact配置可能性を候補ごとに比較する必要がある。

## v024_no_move_capacity_reserve — 知見のみ有効: 小幅reserveは正だが容量価格だけでは不足する

系譜: series=no_move; base=v023_no_move_slot_calendar; imports=[]
当時の判定: 棄却。
仮説: 再移動なしのv023はv017より13.33件/ケース多く受け入れる一方で受理時の理想利用料が0.153M/ケース低く、固定空間を低価値groupへ過剰配分している。admissionが見る有効収容力だけを再移動不能分だけ縮小すれば、受理件数を抑えて後続の高価値groupへcompactな空間を残し、全体最高を更新できる。
変更: v023_no_move_slot_calendarを基準に、移動なし・配置候補・slot calendar・rollout・growth SA/LNS・価値分布モデルを固定する。`initialize_static_capacity`で推定した`effective_capacity`に再移動なし用の単一reserve係数を掛け、base thresholdと占有誤差補正へ同じ容量を使う。初期値0.95とし、同じ仮説内で係数だけを調整する。
機構確認: `capacity_reserve_milli<1000`、v023比で`accepted`が減り、`price_prefilter_reject`が増え、出力解析で理想利用料を大きく落とさずslack>=8件数・損失が減ることを確認する。`move_zero_output=100000`、`fallback_count=0`も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 係数0.975で100/100ケース成功、平均65,307,135（v023比 +0.349%、v017比 -2.682%）、最大1,442ms。受理は645.11→636.25件/ケース、理想利用料は78.259M→78.091M、slack>=8は133.61→128.02件、同損失は12.185M→11.781M、移動0、`fallback_count=0`だった。係数0.95は65,114,239で過剰reserveだった。
学び: 再移動なしでは見かけの収容力を2.5%縮めると、理想利用料0.168Mを手放して形状損失0.404Mを減らせる。ただし改善は0.35%に留まり、残差の主因はadmission総量より、受け入れるgroupをどこへ固定するかという幾何品質である。

## v025_no_move_growth_slot — 条件付き再検討: slot境界起点のgrowth候補を生成できる場合に再検討

系譜: series=no_move; base=v024_no_move_capacity_reserve; imports=[]
当時の判定: 棄却。
仮説: v023のslot保護は通常配置だけで正の効果を得た一方、slot機構が届かないgrowth探索が289.49回/ケースあり、大型targetもregular 4.58件に対してgrowth 22.17件/ケースへ偏る。非target growthでも同一周長・同一成分の候補から大型slotを壊しにくいものを既存rolloutで選べば、現在groupの利用料とadmissionを変えずに後続の悪形を減らし、全体最高を更新できる。
変更: v024_no_move_capacity_reserveを基準に、移動なし・capacity係数・regular slot・admission・seed・SA/LNS・周長制約を固定する。growthの現行winnerを候補0とし、同じperimeter・component_sizeの既存growth候補からslot delay最小を別枠候補に加える。非targetかつ非fast modeだけ現行と同じcalendar・414k換算を使い、delayが厳密に改善するときだけ既存K=3・22到着rolloutで比較する。
機構確認: `growth_slot_calendar_turn`、`growth_slot_candidate_added`、`growth_slot_comparison`、`growth_slot_preservation_flip`が正で、比較時のperimeter・component_sizeが不変、`growth_slot_delay_after<growth_slot_delay_before`を確認する。targetのslack>=8損失と全体の同損失がv024より減り、`move_zero_output=100000`、`fallback_count=0`であることも確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: calendarを代替候補があるturnだけに限定後、100/100ケース成功、平均65,283,687（v024比 -0.036%、v017比 -2.717%）、最大1,444ms。4,374回calendarを作って候補追加は27回、25回反転し、growth slot delayは174,436→13,373へ減ったが、slack>=8損失は11.782M→11.822M/ケースへ悪化、移動0、`fallback_count=0`だった。
学び: growthの既存seed候補は同一周長・同一成分に限定すると位置多様性がほぼなく、slot delayを改善する候補は0.62%しかない。その希少な候補も現行fragment評価を捨てる副作用が上回るため、regularで正だったcalendarをgrowthへ単純移植しない。再開条件: growth候補生成自体をslot境界から始め、現行winner近傍のPareto候補を作れる場合。

## v026_no_move_medium_slots — 知見のみ有効: 中型slotは悪形件数を減らすが価値配分を悪化させる

系譜: series=no_move; base=v024_no_move_capacity_reserve; imports=[]
当時の判定: 棄却。
仮説: v024のslack>=8損失11.782M/ケースのうち、現行大型calendarの対象外である`P=64..95,D>=6000,Q>=1`だけで1.622Mを占める。既存大型slotを変えず、その外側に8x8〜10x10の中型compact-slotを別階層で予約すれば、大型保護を損なわず中型高価値groupの悪形を減らして全体最高を更新できる。
変更: v024_no_move_capacity_reserveを基準に、移動なし・capacity係数・大型slot・admission・regular候補・rollout・growth SA/LNSを固定する。既存大型quotaとslotを先に選び、全class合計6枠の残りへ、それらと非重複な8x8・8x10・10x8・9x9・10x10の早いslotを`P=64..95,D>=6000,Q>=1`の推定同時到着数だけ追加する。中型slotの期待損失は実測222k/badとし、class別到着率・Kで大型とは別に価格化する。
機構確認: `medium_slot_calendar_turn`、`medium_slot_inventory_sum`、`medium_slot_candidate_added`、`medium_slot_preservation_flip`が正で、中型slotのdelay合計が比較前より後で減ることを確認する。大型slotの選抜結果がv024と同じ規則で先に固定され、`move_zero_output=100000`、`fallback_count=0`、中型targetと全体のslack>=8損失がv024より減ることも確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 222k係数で100/100ケース成功、平均65,265,062（v024比 -0.064%、v017比 -2.745%）、最大1,442ms。中型比較20,424回中1,685回でdelayを減らし、15,658,193→10,652,877、slack>=8件数は128.02→126.76件/ケースへ減ったが、同損失は11.782M→11.825Mへ悪化した。半係数も65,257,628で改善せず、移動0、`fallback_count=0`だった。
学び: 中型slot数を増やすと悪形の件数は減るが、P帯だけの平均損失価格では将来groupの価値順位を表せず、高価値な残存悪形へ損失が集中する。静的slotのscale追加はここで止め、全候補が重要slotを塞ぐときの受否を個別の実額で比較する。

## v027_no_move_slot_veto — 知見のみ有効: robust vetoでも受理価値の損失が上回る

系譜: series=no_move; base=v024_no_move_capacity_reserve; imports=[]
当時の判定: 棄却。
仮説: slot calendarは回避可能な損傷を候補間で減らすだけで、同一周長の全合法配置が大型slotを塞ぐ場合でもbase admission通過groupを必ず受理する。現在利用料・短期rollout価値より大型slotの長期外部性が明確に大きい場合だけ拒否を許せば、一般admissionを二重化せず高価値targetのcompact空間を残して全体最高を更新できる。
変更: v024_no_move_capacity_reserveを基準に、移動なし・capacity係数・大型calendar・配置候補・growth・SA/LNS・base admissionを固定する。非target regularの既存slot比較で全候補が正penalty、かつadmissionを通る同一周長の全成分でも無傷候補がない場合だけ、現盤面・即時0のreject候補を共通乱数rolloutへ加える。penaltyなしなら受理勝ち、半penaltyでもreject勝ちとなる場合だけvetoし、配置候補間の選択は従来の全penaltyで行う。
機構確認: `slot_reject_rollout_session`、`slot_reject_global_same_level_damaged`、`slot_reject_raw_accept_supported`、`slot_reject_half_robust`、`slot_reject_executed`が正で、全実行vetoが`raw_accept_score>reject_score>half_penalty_accept_score`を満たすことを確認する。手放した利用料・回避penalty、targetと全体のslack>=8損失、`move_zero_output=100000`、`fallback_count=0`も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: robust係数を0.25へ絞った最終版で100/100ケース成功、平均65,036,583（v024比 -0.414%、v017比 -3.085%）、最大1,436ms。3,221比較のうち369件をvetoし、手放しfee0.723M/ケース、回避penalty1.371M/ケース。slack>=8は128.02→126.88件、同損失は11.782M→11.639Mへ減ったが、理想利用料は78.091M→77.637Mへ低下した。0.5版も65,024,249で、移動0、`fallback_count=0`だった。
学び: slot penaltyは同一利用料の配置候補を相対順位付けするには有効だが、代表矩形の期待損失を受否の絶対価格へ移すと、厳しい因果gateでも将来の高価値受理を選べない。再移動なしの改善は受理集合をさらに削るより、受理するgroupの固定位置候補を増やす側で行う。

## v028_no_move_compact_inventory — 知見のみ有効: fit冗長性候補は受理価値を悪化させる

系譜: series=no_move; base=v024_no_move_capacity_reserve; imports=[]
当時の判定: 棄却。
仮説: 現行`fragment_metric`は成分サイズとdead-endだけを見て、大成分内部の太さを区別しない。代表P=48,80,120の最小周長shapeを置ける位置数の保持率が最大のregular候補を追加すれば、slot一箇所に偏らず多尺度のcompact配置余力を残し、既存rolloutが将来価値の高い候補を選んで全体最高を更新できる。
変更: v024_no_move_capacity_reserveを基準に、移動なし・capacity係数・大型slot・admission・既存3候補・slot候補・rollout・growth SA/LNSを固定する。regular上位20候補について、P=48,80,120の最小周長shape各6種の合法位置数をbitboardで数え、現在盤面からの保持率を残存bad損失比1:3:7で加重する。候補0と同じ成分かつslot penaltyを悪化させないfit-bestを一件だけ追加し、fit値は通貨・admission・最終scoreへ入れない。
機構確認: `compact_fit_turn`、`compact_fit_evaluated`、`compact_fit_candidate_added`、`compact_fit_rollout_comparison`、`compact_fit_rollout_chosen`が正で、選択時の`fit48/80/120_after`加重保持率が候補0以上、slot penaltyが候補0以下であることを確認する。P帯別slack>=8損失、処理時間、`move_zero_output=100000`、`fallback_count=0`も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,216,250（v024比 -0.139%、v017比 -2.818%）、最大1,448ms。fit候補を7,381回追加し6,767比較中1,519回選択、compact-fit処理は11.59ms/ケースだった。slack>=8は128.02→127.87件、同損失は11.782M→11.727Mへ減ったが、理想利用料は78.091M→77.891Mへ低下した。移動0、`fallback_count=0`。
学び: 現在盤面のcompact配置数を保つ候補は形状損失を僅かに減らすが、将来到着の価値・退去時刻を含まないため受理集合の入れ替わり損失が上回る。大成分内部の太さは代理指標で選ぶより、現在group自身の低周長候補を直接構成して既存の通貨評価へ渡す。

## v029_no_move_box_growth — 知見のみ有効: box侵食は形状損失を回収するが現行最高には届かない

系譜: series=no_move; base=v024_no_move_capacity_reserve; imports=[]
当時の判定: 棄却。
仮説: v024のslack>=8受理128.02件/ケースのうち70.99件では、配置直前に面積`P..P+24`の近正方形box内だけでPセル以上の連結空間が存在し、簡易侵食でも51.97件の周長と0.972M/ケースの即時利用料を改善できる。障害物を許すbox候補をgrowthへ追加すれば、固定形状scanとseed成長が見逃す低周長領域を回収して全体最高を更新できる。
変更: v024_no_move_capacity_reserveを基準に、移動なし・capacity係数・regular・slot・admission・既存growth seed・SA/LNS予算・最終評価を固定する。`P>=36`のgrowthで、`P<=h*w<=P+24`、`2(h+w)<=L_min+2`、aspect比1.5以下のboxをprefix sumで走査する。box内のP以上の局所free成分を、外部に接する非関節点から低次数優先でPセルまで侵食し、実周長最良の一候補だけを既存growth候補へSA前に追加する。
機構確認: `box_growth_turn`、`box_position_scanned`、`box_local_component_pass`、`box_erosion_attempt`、`box_erosion_success`、`box_candidate_added`、`box_candidate_entered_sa`、`box_candidate_selected`が正で、候補セル数P・連結・非重複を確認する。box候補の実周長が同turnのbest seedより短い回数、growth slackとP帯別損失、既存SA/LNS総予算、`move_zero_output=100000`、`fallback_count=0`も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,700,592（v024比 +0.603%、v017比 -2.096%）、最大1,443ms。box候補7,898件中7,299件がSAへ入り5,011件が選択され、seed比周長差合計-39,716、slack>=8件数128.02→115.80、同損失11.782M→11.070M/ケースとなった。受理理想利用料は78.091M→77.851Mへ減ったが、形状損失回収が上回り、移動0・`fallback_count=0`だった。
学び: 障害物を許す局所boxから直接Pセルを構成する方法は、固定shape scanとseed成長の実在する探索漏れを大きく回収する。残る11.070M/ケースの悪形損失は到着時点の候補生成だけでは足りず、同じ低周長候補を維持しつつ受理集合と将来盤面価値を改善する必要がある。

## v030_no_move_future_fit — 知見のみ有効: 将来fit保持は僅差改善に留まる

系譜: series=no_move; base=v029_no_move_box_growth; imports=[]
当時の判定: 棄却。
仮説: v024の後続growth形状損失12.415M/ケースのうち、先行groupとの接触辺按分で7.387Mは先行regular配置に由来し、長期滞在group由来は7.628Mある。同一周長regular候補を、現在盤面でなく既知group退去後の複数時点でcompact shapeを壊しにくい座標へ選べば、v029の低周長構成を保ったまま残差1.406Mを回収できる。
変更: v029_no_move_box_growthを基準に、移動0・admission・最初の周長level・slot・growth box/SA/LNS・既存rolloutを固定する。`D>=5000`の非fast regular turnだけ、`S+0.2D, S+0.5D, S+0.8D`で既知activeの退去を反映した盤面を作り、P=48/80/120の最小周長代表shape各6種の合法位置保持率を1:3:7で評価する。同一componentかつslot penalty非悪化のfuture-fit最良候補を一件だけ既存候補へ加え、最終選択は既存共通乱数rolloutへ委ねる。
機構確認: `future_fit_turn`、`future_fit_snapshot`、`future_fit_evaluated`、`future_fit_candidate_added`、`future_fit_rollout_comparison`、`future_fit_rollout_chosen`が正で、候補0より加重保持率が悪化せず、比較時のperimeter・component_size・slot penalty制約が不変であることを確認する。P帯別slack>=8損失、既存growth boxの発動、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,726,747（v029比 +0.040%、v017比 -2.057%）、最大1,443ms。future-fit候補4,483件を追加し3,821回比較、590件を選択した。加重保持率は集計3,635.6→3,849.6へ改善したが、slack>=8損失は11.070M→11.087M/ケースへ僅かに悪化し、理想利用料増0.045Mとの差し引きだけが得点増となった。移動0、`fallback_count=0`だった。
学び: 既知退去後のcompact shape位置数は受理集合を僅かに改善するが、単独blockerが空き成分の橋を塞ぐ損失を十分に順位付けできない。将来盤面はshape位置の重複数ではなく、大型groupを収容できる連結成分容量で直接比較する必要がある。

## v031_no_move_future_components — 知見のみ有効: 成分容量保持は受理価値を悪化させる

系譜: series=no_move; base=v029_no_move_box_growth; imports=[]
当時の判定: 棄却。
仮説: v024の最大空き成分不足による拒否は61.10件・理想利用料16.359M/ケースあり、61.08件は利用中group一件を除くだけで必要成分が復活する。future-fit位置数でなく、既知退去後に大型groupを収容できる連結成分容量を最大化する同一周長regular座標なら、橋を塞ぐ単独blockerを避けてv029の残差を回収できる。
変更: v029_no_move_box_growthを基準に、移動0・admission・周長level・slot・growth box/SA/LNS・既存rolloutを固定する。`D>=5000`の非fast regular turnで`S+0.2D, S+0.5D, S+0.8D`の既知退去盤面を作り、候補配置後の各自由成分サイズ`s`からP=64/96/120ごとに`sum(max(s-P+1,0))`を計算し、基準盤面比を1:3:7で集約する。同一componentかつslot penalty非悪化の最良候補を一件だけ既存候補へ加え、最終選択は既存rolloutへ委ねる。
機構確認: `future_component_turn`、`future_component_snapshot`、`future_component_evaluated`、`future_component_candidate_added`、`future_component_rollout_comparison`、`future_component_rollout_chosen`が正で、候補0より加重成分容量が悪化せず、perimeter・component_size・slot penalty制約が不変であることを確認する。geometry reject、P帯別slack>=8損失、box機構、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,556,917（v029比 -0.219%、v017比 -2.310%）、最大1,452ms。future-component候補を1,948件追加し、加重成分容量は621,718,514→623,335,401へ増えたが、slack>=8損失は11.070M/ケースで不変、受理理想利用料は77.851M→77.707Mへ低下した。移動0、`fallback_count=0`だった。
学び: 既知退去後の連結成分容量を保つ候補は橋blockerの局所指標を改善しても、将来到着の価値・時刻との対応がなく受理集合を悪化させる。現在/将来盤面の代理指標追加を続けず、候補自身の利用料差を既存rolloutの通貨尺度で直接比較する。

## v032_no_move_cross_perimeter — 知見のみ有効: L+2の将来gainは即時損失と時間を回収しない

系譜: series=no_move; base=v029_no_move_box_growth; imports=[]
当時の判定: 棄却。
仮説: 現行regular配置は最初に置ける周長levelで打ち切るため、同じgroupの`L+2`候補が作る将来盤面価値を、その利用料低下`V*(C_L-C_{L+2})`と比較していない。低価値groupでは僅かなcompactnessを譲って後続の高価値groupへ良形空間を残す方が得な場合があり、通貨尺度の既存rolloutで比較すれば代理指標を増やさずv029の残差を回収できる。
変更: v029_no_move_box_growthを基準に、移動0・admission式・最初の周長候補群・slot・growth box/SA/LNS・K=3/22 rolloutを固定する。非fastかつ時間比0.90未満で最初の候補0が価格を通るturnだけ、次の`L+2` levelを既存regular scanで上位20件調べる。各候補自身のcomponent thresholdを`q*C(P,L+2)`が通るものに限定し、現行`final_score`最良を一件だけ追加する。rollout即時額を候補ごとの`V*C-slot_penalty`として比較し、`L+2`が価格を通らない場合は従来のL候補だけを使う。
機構確認: `cross_perimeter_turn`、`cross_perimeter_scan`、`cross_perimeter_price_pass`、`cross_perimeter_candidate_added`、`cross_perimeter_rollout_comparison`、`cross_perimeter_rollout_chosen`が正で、選択した`L+2`候補自身もadmissionを通り、即時利用料損失とrollout将来gainの差で選ばれることを確認する。既存L候補・box機構、P帯別slack>=8損失、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,526,435（v029比 -0.265%、v017比 -2.355%）、最大1,452ms。L+2候補42,601件を比較して3,628件選択し、slack>=8損失は11.070M→10.720M/ケースへ減ったが、slack2損失増と受理理想利用料77.851M→77.576Mの低下が上回った。fast modeは1,266turn、box選択は5,011→4,830件となり、移動0・`fallback_count=0`だった。
学び: 異周長を通貨尺度で比較しても、短期rolloutはL+2が生む長期形状改善を即時compactness損失・受理集合変化・追加計算込みで正しく回収できない。L+4へ広げず、同一周長内で受理feeを失わない改善かgrowth形状構成そのものへ戻る。

## v033_no_move_box_beam — 知見のみ有効: 事前probeで追加回収上限が小さいと確認済み

系譜: series=no_move; base=v029_no_move_box_growth; imports=[]
当時の判定: 中断。
仮説: v029のbox侵食は各局所成分を一つの決定的な削除順だけでPセルにし、greedy選択の手詰まり・局所周長を探索していない。v029に残るslack>=8損失11.070M/ケースのうちP>=64が10.061Mを占めるため、高価値大型boxだけ複数の侵食順を保持すれば、受理集合を変えず形状損失を追加回収して全体最高を更新できる。
変更: v029_no_move_box_growthを基準に、移動0・admission・regular/slot・既存box寸法/窓走査・seed growth・SA/LNS予算・最終評価を固定する。現行greedyで全box中最良になった元成分について、`P>=64`、余剰2..24、greedy slack>=4、最小周長までの回収可能利用料20万以上、時間比0.86未満の場合だけ侵食beamを行う。各depthで外部接触かつ除去後連結なセルを各親上位6分岐、removed集合をdedupして周長優先の幅12を残す。最終beamと現行greedyを実周長、同値なら既存`cheap-1.4*fragment`で比較し、box枠一件だけを置換する。
機構確認: `box_beam_eligible`、`box_beam_session`、`box_beam_layer`、`box_beam_state`、`box_beam_child`、`box_beam_success`、`box_beam_candidate_replaced`、`box_beam_perimeter_reduction`が正で、beam候補がPセル・連結・非重複、greedyより周長非悪化であることを確認する。boxのSA進入/選択、P帯別slack>=8損失、既存SA/LNS反復数、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: solver実装・eval前の固定軌跡20ケースprobeで、P>=64のstrict box成分は8.75件/ケースだけだった。幅4 beamの即時fee回収は0.021M/ケース、幅8でも0.022Mで、幅拡大の上積みは0.001M未満だったため中断し、不完全なbinは削除した。
学び: v029のgreedy侵食後に残る1.406M/ケース差は、同じstrict box内の削除順探索では埋まらない。box beamへ時間を足さず、同一周長候補の現在盤面に対する実分断をfee不変で減らす方向を試す。

## v034_no_move_growth_topology — 条件付き再検討: growth生成時にtopology多様性を作れる場合に再検討

系譜: series=no_move; base=v029_no_move_box_growth; imports=[]
当時の判定: 棄却。
仮説: v029の悪形growthが元自由成分の最大残余成分から切り離すセル数は中央値12であり、現行`fragment_metric`はこの切断量を直接最小化しない。改善後の同一周長・同一component候補から、fragmentを悪化させず切断セルを減らすPareto候補へ置換すれば、現在groupのfeeとadmissionを一切変えず後続の連結空間を守り、全体最高を更新できる。
変更: v029_no_move_box_growthを基準に、移動0・admission・regular/slot・seed/box候補生成・top2 SA/LNSの対象/順序/予算・乱数列を固定する。全growth候補の既存最終評価後に現行winnerをbaselineとし、同じperimeter・component_sizeの候補だけ、`severed=component_size-P-(配置後に元成分へ残る最大連結成分)`をexact BFSで求める。`severed<baseline`かつ`fragment_delta<=baseline`の候補があれば、severed最小、同値なら既存final_score最大へ直接置換し、なければbaselineを維持する。rolloutや新しい係数は使わない。
機構確認: `growth_topology_same_fee_candidate`、`growth_topology_comparison`、`growth_topology_pareto_size`、`growth_topology_flip`、`growth_topology_severed_before/after/reduction`、`growth_topology_fragment_before/after`、`growth_topology_cheap_loss`が正で、flip時のperimeter・component_size不変、fragment非悪化を確認する。P/slack帯別flip、box↔seed遷移、既存SA/LNS反復、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,710,384（v029比 +0.015%、v017比 -2.081%）、最大1,440ms。同fee候補14,948件の比較でPareto候補101件、flip77件だけが発動し、対象のseveredを3,651→2,084、fragment×1000を13,556,121→13,098,539へ減らした。slack>=8損失は11.070M→11.142M/ケース、移動0、`fallback_count=0`だった。
学び: 現行候補集合には同feeかつfragment非悪化でseveredを減らせる代替が0.77件/ケースしかなく、最終選抜だけでは規模不足である。切断を減らすなら、候補完成後のoverrideではなくgrowth生成時の同周長タイブレークへ橋セル損失を入れて候補多様性を作る必要がある。

## v035_no_move_growth_cutloss — 後続への統合: 関節点cut-loss tie-breakが現行solverへ移植済み

系譜: series=no_move; base=v029_no_move_box_growth; imports=[]
当時の判定: 棄却。
仮説: v029固定軌跡20ケースのslack>=8 growth 2,383件中87.2%が配置直前の関節点cut-loss正セルを含み、平均5.16セル・cut-loss合計85.7だった。共有辺数dが同じ成長候補で橋セルを後回しにすれば、周長増分を変えず元自由成分の切断を減らす候補多様性を生成し、v034で不足した同fee代替を増やして全体最高を更新できる。
変更: v029_no_move_box_growthを基準に、移動0・admission・regular/slot・box・SA/LNS・最終評価・探索予算を固定する。growth呼出時に現在free graphをTarjan法で一度走査し、各セルを除いたとき最大残余成分外へ切れるセル数`cut_loss`を計算する。各成分のweight上位seed2件だけを`cut_loss昇順→weight降順`へ変更し、既存6空間sampleは維持する。frontierは共有辺数dを厳密最優先のまま、同d内だけ`cut_loss→ring→manhattan→attraction→id`で選ぶ。cut-lossはbox、SA/LNS、最終scoreへ入れない。
機構確認: `growth_cutloss_turn`、`growth_cutloss_positive_cell`、`growth_cutloss_sum/max`、`growth_cutloss_seed_changed`、`growth_cutloss_positive_selected`が正で、Tarjan値を小盤面の素朴なセル除去BFSと照合する。生成直後/SA後/最終候補のexact severed、P/slack帯、既存周長・SA/LNS反復・box発動、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,828,027（v029比 +0.194%、v017比 -1.906%）で再移動なし最高を更新、最大1,449ms。growth平均slackは28.329→26.983、slack>=8損失は11.070M→10.740M/ケースへ減った一方、受理理想利用料は77.851M→77.726Mへ低下した。seed変更285回、cut-loss処理は単発約7.5ms、移動0・`fallback_count=0`だった。
学び: 共有辺数dを保った関節点回避は候補形状を改善し、静的cut-lossと後続損失の単純相関が弱くても生成tie-breakとして正に働く。ただし幾何改善0.330Mの一部を受理集合変化で失うため、次はこの生成を保ち、既知退去列による価格誤差を独立に直す。

## v036_no_move_reservation_price — 条件付き再検討: 条件付き将来到着負荷を同じ積分点へ加える場合に再検討

系譜: series=no_move; base=v035_no_move_growth_cutloss; imports=[]
当時の判定: 棄却。
仮説: 現行admissionは滞在区間の需要価格を3点積分した後、到着時の総占有数だけで一括補正するため、現在は混雑していても滞在中に既存groupが退去する長期案件を過大価格化する。v029固定軌跡では既知予約量版の閾値比中央値0.960、現行Noの救済候補1,862件・理想fee上限3.817M/ケースであり、確定退去列を各評価点へ入れればv035の残差を受理価値で回収できる。
変更: v035_no_move_growth_cutlossを基準に、移動0・capacity ratio・配置/slot/box/cut-loss/SA/LNS/rolloutを固定する。既存3評価時刻ごとに`reserved(t)=sum(P[j] for active j with T[j]>t)`と同時刻のtarget occupancyを求め、各`local_bid(t)`へ`exp(0.70*(reserved(t)+0.5P-target(t))/capacity)`を既存範囲でclipしてからGauss重みで積分する。到着時occupancyによる一括multiplierは外す。探索量を固定するため0.74 prefilterだけは旧thresholdを使い、配置後admissionとrolloutは新thresholdを使う。
機構確認: `reservation_price_turn`、3点の`reservation_reserved_area`、`reservation_target_area`、`reservation_threshold_old/new_milli`、`reservation_price_rescue_candidate`、`reservation_price_drop_candidate`、救済/脱落の件数・理想feeが正で、prefilter通過集合がv035と同じであることを確認する。accepted/geometry/slack損失、cut-loss/box機構、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,255,959（v035比 -0.869%、v017比 -2.758%）、最大1,449ms。新価格は2,183件・理想fee 3.417M/ケースを追加救済したが、受理理想利用料は77.726M→77.688M、形状損失は11.898M→12.432M/ケースへ悪化した。移動0、`fallback_count=0`だった。
学び: 現activeの確定退去を差し引くだけでは、その後に到着する未知groupの競合を価格から落とし、長期案件を過剰に救済する。退去列をadmissionへ使うなら、同時刻までの条件付き将来到着負荷も必ず足す必要がある。

## v037_no_move_prefix_theta — 知見のみ有効: 推定改善だけでは暗黙reserveが失われる

系譜: series=no_move; base=v035_no_move_growth_cutloss; imports=[]
当時の判定: 棄却。
仮説: 現行theta推定は到着順prefixに未観測groupの`S>current_S`を条件付けず、中盤で平均約245過大である。公式の丸め・`l<100000`・到着順条件を尤度に入れればbiasがほぼ0になり、admission価格、slot需要量、rollout滞在を同じ生成過程に対して整合させ、v035の受理と固定配置判断を改善できる。
変更: v035_no_move_growth_cutlossを基準に、移動0・capacity ratio・配置/slot/box/cut-loss/SA/LNS/rollout/admission式を固定する。`theta=2000,2100,...,8000`ごとに丸め付き切断指数分布の`log p(D-1)`を累積し、未観測数分の`log P(S>current_S)`を加えたsoftmax後平均thetaだけを全theta利用箇所へ渡す。現行推定もshadow計算し判断差をtraceする。
機構確認: 決定的模擬生成でprefix推定のbias/MAEが現行より小さく、`theta_prefix_turn=100000`、`theta_legacy/new_milli`、旧/新prefilter・配置後passと反転数が正であることを確認する。accepted/geometry/slack損失、cut-loss/box機構、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,733,682（v035比 -0.143%、v017比 -2.046%）、最大1,449ms。prefix thetaは現行より平均230.9小さく、prefilter 962件・配置後1,410件を片方向に通した。受理理想利用料は77.726M→78.105Mへ増えたが、形状損失が11.898M→12.371M/ケースへ悪化し、移動0・`fallback_count=0`だった。
学び: 公式生成に忠実なprefix theta推定は統計的に正しいが、現行の過大thetaは再移動なし盤面に対する暗黙の容量reserveとして機能していた。生成推定と運用reserveを分離し、prefix thetaに合わせてcapacity ratioを明示的に再較正する価値がある。

## v038_no_move_prefix_reserve — 知見のみ有効: reserveを分離してもtheta精度の純利益はほぼない

系譜: series=no_move; base=v037_no_move_prefix_theta; imports=[]
当時の判定: 棄却。
仮説: v037の正しいprefix thetaは受理理想利用料を0.379M/ケース増やしたが、現行の過大thetaが担っていた暗黙reserveを消し、形状損失を0.474M増やした。生成推定と無移動用reserveを分離し、後者をcapacityで明示化すれば、受理価値を保ったまま形状悪化を抑えられる。
変更: v037_no_move_prefix_thetaを基準に、移動0・prefix推定・配置/slot/box/cut-loss/SA/LNS/rollout/admission式・時間比率を固定し、`NO_MOVE_CAPACITY_RATIO`だけを0.975から0.950へ変更する。v035固定軌跡へtheta比0.9575を入れたshadowで、0.950は旧価格とのlog閾値差平均+0.16%・中央値+0.57%となるため、この一点に事前固定する。
機構確認: `capacity_reserve_milli=950`、`theta_prefix_turn=100000`、v037比でacceptedが減少しprice rejectが増加すること、受理理想利用料とslack>=8損失を確認する。cut-loss/box/rollout機構、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,816,980（v037比 +0.127%、v035比 -0.017%、v017比 -1.922%）、最大1,448ms。受理はv037の645.09→639.05件/ケースへ減り、理想利用料はv035比-0.160M、形状損失は-0.149M/ケースでほぼ相殺した。prefix・cut-loss/box/rolloutは発動し、移動0・`fallback_count=0`だった。
学び: capacity ratio 0.950は過大thetaの暗黙reserveを再現し、v037の損失を0.011M/ケースまで戻したが、正しいthetaがslotと短期rolloutへもたらす純利益はこの精度ではほぼ0である。theta/reserve調整を続けず、v035を基準に直接の幾何改善へ戻る。

## v039_no_move_temporal_cutloss — 条件付き再検討: 将来groupの実配置差分rolloutを使える場合に再検討

系譜: series=no_move; base=v035_no_move_growth_cutloss; imports=[]
当時の判定: 棄却。
仮説: v035のbad完成領域の84.1%は現在の自由成分を分断し、v017が移動で良形化したbad差のうち、既存group退去後に分断が悪化する案件だけで2.198M/ケースある。現在の1セルcut-lossに加え、incoming滞在中の既知退去後にもseparatorとして残るセルをgrowth生成時から避ければ、移動で後から解いていたspace-time inversionを予防できる。
変更: v035_no_move_growth_cutlossを基準に、移動0・admission・regular/slot/box・static growth・SA/LNS/rollout・時間比率を固定する。まずv035と同じbaseline winnerを確定し、`P>=64`・理想周長へのfee損失`>=100000`・時間比`<0.90`の時だけ、`S,S+0.2D,S+0.5D,S+0.8D`で既知activeを解放したcut-lossを`2:3:3:2`で積分し、baselineと同じ自由成分からtemporal候補を追加生成する。
機構確認: `temporal_cutloss_eligible/snapshot/candidate`が正で、元候補群とbaseline winnerがv035に一致することを確認する。temporal候補は同一成分のみとし、`L<baseline.L`は優先、`L==baseline.L`では4時点のregion-level severed加重和最小を優先、`L>baseline.L`は不採用とし、flip数とsevered減少を確認する。bad形状損失、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,516,050（v035比 -0.474%、v017比 -2.371%）、最大1,456ms。eligible 5,927件、candidate 9,190件、flip 980件で加重severedを4,174,348→4,146,951へ減らしたが、受理は635.09件/ケース、理想利用料77.199M、形状損失11.683Mとなった。baseline一致5,927件、移動0、`fallback_count=0`だった。
学び: 既知退去後の最大残余成分だけを同fee候補の目的にすると分断値自体は下がるが、将来到着の形状需要と退去セルへの再接続方向を区別できず、v035の局所重み・static topologyより悪い位置を選ぶ。再開条件: 将来groupを実際に配置する差分rolloutで候補を順位付けできる場合。

## v040_no_move_biased_swap — 後続への統合: biased boundary swapの原型が現行solverへ移植済み

系譜: series=no_move; base=v035_no_move_growth_cutloss; imports=[]
当時の判定: 棄却。
仮説: v035の既存SAは全100ケースで27,786,528反復を一様なremove/addへ使う一方、固定軌跡probeでは非関節境界の低次数removeと高次数frontier addへ偏らせた512反復が、回収可能fee 100k以上の2,954件中2,221件を改善し0.513M/ケースを回収した。最終growth winnerだけへこの提案分布を使えば、同じ局所近傍でも残る周長損失を時間内に直接減らせる。
変更: v035_no_move_growth_cutlossを基準に、移動0・admission・regular/slot/box・growth生成・既存SA/LNS/rolloutを固定する。baseline winnerが受理済みで`P>=64`・`L-L_min>=8`・回収可能fee`>=100000`・時間比`<0.86`の時だけ最大512反復を後置し、remove次数1/2/3を24:6:1、add次数1/2/3/4を1:6:36:216で抽選する。`ΔL<=0`を受理し、悪化は温度1.0→0.03の幾何冷却で受理する。
機構確認: `biased_swap_eligible/session/iteration/accepted/improved`が正で、baselineより厳密に短いbestだけを返し、同周長なら座標もbaselineのままにする。周長削減・即時fee回収と形状損失を測り、16反復ごとの時間確認で0.90を越えないこと、`move_zero_output=100000`、`fallback_count=0`も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,878,542（v035比 +0.077%、v017比 -1.830%）で再移動なし最高を更新したが、最大1,456ms。session 2,519件・1,289,728反復で周長8,232、即時fee44.644M（0.446M/ケース）を回収した一方、受理理想利用料はv035比0.449M/ケース低下した。移動0、`fallback_count=0`だった。
学び: final shapeの局所周長を厳密に改善しても、置換セルが後続の受理可能性を失わせて即時益の89%を相殺する。周長改善候補は強制採用せず、baselineとの実将来配置差分で選ぶ必要がある。再開条件: 同じbiased候補をbaselineと共通乱数rolloutで比較する場合。

## v041_no_move_swap_rollout — 知見のみ有効: 64到着quick rolloutも局所改善の副作用を識別できない

系譜: series=no_move; base=v040_no_move_biased_swap; imports=[]
当時の判定: 棄却。
仮説: v040はbiased候補の即時feeを0.446M/ケース改善した一方、後続の受理理想利用料を0.449M失った。baselineと改善候補へ同一の将来到着列を実配置すれば、周長益を残しつつ後続容量を壊す置換だけをvetoできる。
変更: v040_no_move_biased_swapを基準に、移動0・admission・通常配置・growth/box・既存SA/LNS・通常配置用22到着rolloutを固定する。受理済みgrowthのbiased bestがbaselineより短い場合だけ、両盤面を専用の64到着×3本の共通乱数rolloutで比較し、`即時fee + 将来受理fee`最大を選ぶ。高負荷時の余白確保としてbiased開始/打切り比だけ0.84/0.88へ前倒しする。
機構確認: `swap_rollout_comparison/baseline/improved`が全て正で、提案・選択別の周長削減と即時fee gainを測る。共通乱数・既存active退去・incoming退去・未来group退去を両候補で一致させ、biased不発・時間打切り時はv040以前のbaselineを保つ。`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが全体最高v017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,829,968（v035比 +0.003%、v040比 -0.074%、v017比 -1.903%）、最大1,458ms。1,885比較でbaseline 124・改善1,761を選び、提案fee45.721M中42.628Mを残した。形状損失はv035比0.589M/ケース減ったが、受理理想利用料も0.587M減って再び相殺した。移動0、`fallback_count=0`だった。
学び: 64到着へ延ばしてもfirst-fit quick placementはbiased swapの長期的な受理集合変化をほぼ全て有利と誤判定する。候補選択のhorizon調整を続けず、簡易rolloutとは異なる大域配置規則か、actual solver相当の差分評価が必要である。

## v042_no_move_admission_rollout — 条件付き再検討: actual solver相当の受否差分評価を時間内に行える場合に再検討

系譜: series=no_move; base=v035_no_move_growth_cutloss; imports=[]
当時の判定: 棄却。
仮説: v035のnear-threshold受理は解析的な平均容量価格だけで不可逆な幾何損失を見ないため、actual-future64 forkで得た楽観上限3.927M/ケースの一部を、accept/reject盤面の共通乱数rollout差で回収できる。
変更: v035_no_move_growth_cutlossを基準に、移動0・配置/slot/box/cut-loss/SA/LNS・通常配置rollout・admission式を固定する。時間比0.84未満の`base_threshold>0`かつ実配置margin 1.00..1.10の受理だけ、quick placementの22到着×3共通乱数で両盤面を比較する。各sampleで`Z=G-Λ_full+F_A-F_R+Λ_H`とし、解析shadowの区間分`Λ_H`を戻して3本全て`Z<0`のときだけvetoする。
機構確認: `near_veto_margin_eligible/session/sample/negative_sample/executed/kept`が正で、同一sample・同一固定thresholdを両枝に使い、`h=D`で`Λ_H=Λ_full`となる恒等性を数値照合する。veto時はnegative sample=3、reject→acceptなし、`move_zero_output=100000`、`fallback_count=0`、追加時間も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均65,765,420（v035比 -0.095%、v017比 -1.999%）、最大1,453ms。3,818 sessionの11,454 sample中2,194本が負で166件をvetoし、恒等性誤差は最大66e-15、移動0・`fallback_count=0`だった。受理理想料を0.175M/ケース失い、形状損失改善0.112Mを上回った。
学び: shadowの二重計上を除いても、22到着first-fit rolloutはactual-future64の受理正負を十分に識別できない。quick rolloutのadmission調整は打ち切り、actual oracleでsigned正と確認した滞在比・形状帯の因果を別実験で検証する。

## v043_no_move_causal_veto — 知見のみ有効: 因果帯reserveは正だが現行最高には届かない

系譜: series=no_move; base=v035_no_move_growth_cutloss; imports=[]
当時の判定: 棄却。
仮説: actual-future64 oracleで`D/theta<2`・実配置slack 14以下のnear-threshold受理は、現在feeを含むaccept枝よりreject枝のsigned差が正であり、この因果帯だけに容量reserveをかければv035の後続受理価値を回収できる。
変更: v035_no_move_growth_cutlossを基準に、移動0・配置/slot/box/cut-loss/SA/LNS/rollout・元のadmission式を固定する。元の受理が通った`base_threshold>0`かつ実配置margin 1.00..1.04のうち、`D/theta<2`かつ`L-L_min<=14`だけをvetoする。固定軌跡では10.91件/ケース・拒否対象fee 0.698M・actual-future64 signed +0.413Mの事前固定帯である。
機構確認: `causal_veto_near_threshold/duration_pass/slack_pass/executed`が正で、全vetoがmargin・`D/theta`・slackの3条件を満たし、reject→acceptなしを確認する。veto件数・fee・P/滞在比/slack帯、受理理想料と形状損失、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 単独負荷の最良margin上限1.13で100/100ケース成功、平均66,270,794（v035比 +0.673%、v017比 -1.246%）、最大1,455ms。1.10では27.21件/ケースをvetoして形状損失0.332Mを受理理想料0.059Mで回収し、1.12/1.15は66,205,201/66,142,553、1.14は66,119,308だった。移動0・`fallback_count=0`を確認し、並行probeで汚染された評価行は採否から除外した。
学び: actual-futureで正と見えた滞在比・slack帯は逐次実行でも有効で、正のreserve帯はmargin 1.13までに局在する。admission単独で0.443M/ケースを回収したが最高まで0.836M残るため、この帯を固定部品とし残差は配置機構から独立に回収する必要がある。

## v044_no_move_veto_biased — 知見のみ有効: 因果帯reserveとの統合は正だが現行系には残らない

系譜: series=no_move; base=v040_no_move_biased_swap; imports=[v043_no_move_causal_veto]
当時の判定: 棄却。
仮説: v040のbiased swapは即時周長益0.446M/ケースを得たが後続受理理想料を0.449M失い、v043の因果帯reserveは小さい受理価値損失0.059Mで形状損失を0.332M改善した。reserveで空間余力を先に確保すればbiased候補の後続副作用が弱まり、両部品の単純加算を超えて即時周長益を残せる。
変更: v040_no_move_biased_swapを基準に、移動0・admission・通常配置・growth/box/cut-loss・SA/LNS/rollout・biased swap・時間比率を固定する。元の受理が通った`base_threshold>0`かつ実配置margin 1.00..1.10のうち、`D/theta<2`かつ`L-L_min<=14`だけをv043と同じ位置でvetoする。
機構確認: v040の`biased_swap_*`とv043の`causal_veto_*`がともに正で、全vetoが3条件を満たしreject→acceptがないことを確認する。即時biased fee回収、受理理想料、形状損失、両部品の単体差との相互作用、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 単独負荷・margin上限1.13で100/100ケース成功、平均66,356,164（v043比 +0.129%、v017比 -1.119%）、最大1,455ms。biased swapは2,495 sessionで即時fee0.472M/ケースを回収し、因果vetoは35.39件/ケース発動した。v035比で形状損失を0.826M改善し受理理想料を0.298M失った結果、純増0.528M、移動0・`fallback_count=0`だった。並行probeで汚染された旧評価は採否から除外した。
学び: 因果帯reserveはbiased swapの後続容量損失を緩和し、両部品の統合は小幅ながらv043比+0.085M/ケースで正となる。ただし即時形状改善と後続受理価値の交換は残るため、残差には候補ごとのroom option損失を直接値付けする必要がある。

## v045_no_move_geometry_veto — 知見のみ有効: 静的case相関は逐次利得へ移らない

系譜: series=no_move; base=v043_no_move_causal_veto; imports=[]
当時の判定: 棄却。
仮説: pond境界が複雑なparkでは空きをreserveしてもcompact領域へ再結合しにくく、v043のnear-threshold vetoが失う受理価値を後続形状で回収できない。静的な芝生対pond・盤外の境界長が小さいparkだけへ因果帯reserveを適用すれば、複雑parkの下振れを避けてv043の正のケースを残せる。
変更: v043_no_move_causal_vetoを基準に、移動0・margin 1.10・`D/theta<2`・slack 14以下・admission・全配置機構・時間比率を固定する。初期盤面の各芝生セルからpondまたは盤外へ接する4近傍辺を数え、境界長1200以下のcaseだけvetoを有効化し、それ以外はv035と同じ判断にする。
機構確認: `park_boundary`と`causal_veto_case_enabled`を記録し、enabled caseだけ`causal_veto_executed>0`、disabled caseは0かつv035出力・score一致を確認する。受理理想料と形状損失、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 単独負荷・margin上限1.13で100/100ケース成功、平均66,247,020（v043比 -0.036%、v017比 -1.281%）、最大1,453ms。境界長1200以下78ケースでvetoを有効化して28.63件/ケース発動し、22 disabledケースは発動0、全出力で移動0・`fallback_count=0`だった。
学び: 保存済み出力ではpark境界長と因果veto利得に相関があっても、時間打切りを含む逐次軌跡の分岐後scoreは静的case gateで改善しない。case-level selectorは打ち切り、decision時点のroom option損失を直接測る必要がある。

## v046_no_move_deep_biased — 知見のみ有効: biased swapの深化は後続受理価値を過剰に損なう

系譜: series=no_move; base=v044_no_move_veto_biased; imports=[]
当時の判定: 棄却。
仮説: v044は平均1140msでlocal予算を約380ms残し、biased swapは24.95 session×最大512反復で即時fee0.472M/ケースを回収している。reserveで後続容量損失を抑えたまま軽いcaseの反復を深くすれば、未使用時間を追加の周長改善へ変換できる。
変更: v044_no_move_veto_biasedを基準に、移動0・margin 1.13の因果veto・admission・全配置/探索機構・biased提案分布・開始/打切り比0.86/0.90を固定し、`BIASED_SWAP_ITERATIONS`だけ512から2048へ増やす。
機構確認: `biased_swap_iteration`がv044の12,774/ケースを上回り、追加の周長削減と即時fee回収が正であること、time-limit-hit・受理理想料・形状損失を確認する。因果veto、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 単独負荷で100/100ケース成功、平均65,986,149（v044比 -0.558%、v017比 -1.670%）、最大1,459ms。biased反復は12,774→49,029/ケース、即時fee回収は0.472M→0.515Mとなったが、v044比の形状損失改善0.340Mに対し受理理想料を0.710M失った。因果veto35.46件/ケース、移動0・`fallback_count=0`だった。
学び: 同じ1セル交換を4倍近く深めても即時周長益は0.043Mしか増えず、セル置換の後続容量損失だけが拡大する。未使用時間は局所形状の深化でなく、room option評価か新しい候補生成へ使う必要がある。

## v047_no_move_strong_biased — 後続への統合: 高回収余地biased swapがv050へ移植済み

系譜: series=no_move; base=v044_no_move_veto_biased; imports=[v040_no_move_biased_swap]
当時の判定: 棄却。
仮説: v044のbiased swapは即時fee0.472M/ケースに対して純増0.085Mしか残らず、v046で弱い追加改善を増やすと受理理想料損失が急増した。回収可能feeの大きいgrowthだけへ限定すれば、後続セル配置を変える回数を半減しながら即時益の大きい候補を残せる。
変更: v044_no_move_veto_biasedを基準に、移動0・margin 1.13の因果veto・admission・全配置/探索機構・biased 512反復と時間比を固定し、`BIASED_SWAP_MIN_RECOVERABLE_FEE`だけ100,000から200,000へ上げる。
機構確認: biased sessionがv044の24.95件/ケースから減り、即時fee回収・受理理想料・形状損失の比率が改善することを確認する。因果veto、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 単独負荷の最良300k gateで100/100ケース成功、平均66,408,521（v044比 +0.079%、v017比 -1.041%）、最大1,454ms。biased sessionは24.95→6.64件/ケース、即時fee回収0.217Mとなり、v035比で形状損失を0.804M改善し受理理想料を0.223M失った。200kは66,407,491、因果veto35.97件、移動0・`fallback_count=0`だった。
学び: biased swapは回収余地300k以上の強いgrowthへ絞ると、弱い候補を含むv044より後続受理価値を保って純増する。ただし200kとの差は0.001Mで選別余地は飽和し、残る0.698M/ケースはroom option評価または新形状生成が必要である。

## v048_no_move_release_atlas — 条件付き再検討: room候補をrollout外の保守条件で選べる場合に再検討

系譜: series=no_move; base=v047_no_move_strong_biased; imports=[]
当時の判定: 棄却。
仮説: q>=1.5のfragmented rejectには同形・同周長の先行blocker平行移動だけで救える理想利用料0.792M/ケースがあり、既知退去を反映した連結成分投影は0.423M分を事前識別する。小・中型の長期blockerへ連結room候補を追加し、実配置rolloutで選べば、既存の長方形slotが見落とす後続配置価値を回収できる。
変更: v047_no_move_strong_biasedを基準に、移動0・admission/因果veto・全shape/growth/SA/LNS/biased・既存slotとrollout・時間比率を固定する。非fastのregularかつP<=95・D>=3000だけ、同周長・baselineと同じ自由成分の全位置から盤面5×5 bucketごとの局所重み最大を残し、S+D/4, S+D/2, S+3D/4で既知active退去後の最大自由成分合計が最大の1候補を既存22到着×3 rolloutへ追加する。
機構確認: `release_atlas_eligible/snapshot/bucket/candidate_added/comparison/flip`が全て正で、追加候補はbaselineと同周長・同一成分であること、投影最大成分の選択前後差とrollout勝敗を測る。因果veto・biased swap、`move_zero_output=100000`、`fallback_count=0`、最大1,450ms未満も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均66,265,113（v047比 -0.216%、v017比 -1.254%）、最大1,454ms。atlasは230.95件/ケース発動し候補22.48件、比較100.00件、投影改善flip2.88件だったが、rollout winnerの投影scoreは合計1,910/ケース低下した。移動0、`fallback_count=0`だった。
学び: 連結room候補を増やすだけでは短期rolloutが価値を識別せず、形状損失をほぼ変えないまま受理理想料を0.144M/ケース失う。次は候補生成と選択を分離し、現在連結性非悪化かつ将来room差が十分大きい場合だけ保守的に反転する。

## v049_no_move_room_pareto — 知見のみ有効: 最大成分を守ってもcompact roomは保証されない

系譜: series=no_move; base=v048_no_move_release_atlas; imports=[]
当時の判定: 棄却。
仮説: v048の失点はroom候補を短期rolloutへ混ぜたことで生じ、連結性を直接改善する候補だけを独立選択すれば、固定軌跡で識別可能だった理想利用料0.423M/ケースの一部を受理集合の損失なしに回収できる。
変更: v048_no_move_release_atlasを基準に、移動0・atlas対象/3時点/5×5候補生成・全既存機構を固定する。追加room候補を22到着rolloutから除外して従来候補だけのwinnerを先に確定し、全choices中のroom score最大候補がwinnerより期待P以上改善し、現在盤面の最大自由成分も非悪化のときだけroom候補へ反転する。
機構確認: v048の`release_atlas_*`に加え、`room_pareto_compare/gain_pass/current_pass/flip`が全て正であること、rollout候補数がv047と同じであることを確認する。反転前後の将来・現在最大成分差、受理理想料・形状損失、因果veto・biased、移動0、`fallback_count=0`、最大1,450ms未満も測る。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均66,070,832（v047比 -0.508%、v017比 -1.544%）、最大1,456ms。198.11比較/ケースでfuture gain条件13.30件、Pareto反転12.29件（追加候補3.30件）、将来最大成分合計+1,618/ケース、移動0、`fallback_count=0`だった。
学び: room反転は受理集合の理想利用料を0.013M増やしたが形状損失を0.351M悪化させた。最大自由成分は人数収容だけを表しcompact配置可能性を保証しないため、この指標の閾値調整を止め、最小周長shapeの有無を直接評価する。

## v050_move_strong_biased — 後続への統合: 現行solverの直接基盤として存続

系譜: series=current; base=v017_normal_rollout; imports=[v047_no_move_strong_biased]
当時の判定: 採用。
仮説: v047で正だった高回収余地growth限定のbiased boundary swapを移動あり最高のv017へ移植すれば、受理済み悪形growthを既存groupの移動費なしで改善できる。改善後形状をrelocationのbaselineにすれば、局所改善で十分な場合は移動探索を省き、不十分なら既存relocationを残せるため両機構は補完的に働く。
変更: v017_normal_rolloutを基準に、通常配置・admission・通常/移動ロールアウト・repack・時間予算を固定する。受理済みgrowth winnerのうち`P>=64`・`L-L_min>=8`・回収可能利用料30万以上だけ、v047と同じ次数重み・512反復・開始/打切り比0.86/0.90の1セルboundary swapを行う。厳密に周長が短くなった場合だけ採用し、改善後の`V*C`をnormal baselineとrelocation判定へ反映する。
機構確認: `biased_swap_eligible/session/applied/perimeter_reduction/fee_gain`が正で、0009・0082における発動、`biased_swap_avoided_relocation`、既存`relocation_success`と`biased_swap`時間を確認する。返す領域がPセル・連結・空き・同一自由成分であり、`fallback_count=0`も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv017_normal_rolloutの67,106,996を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100ケース成功、平均67,416,980（v017比 +0.462%、54勝29敗17同点）で全体最高を更新し、最大1,372ms。838 eligible中835 session・783改善、周長8,230・即時fee60.799Mを回収し、局所改善だけでrelocationを4回省いた一方、既存relocationも6,752回成功した。biased時間は計957.7ms、全検証assert通過、`fallback_count=0`だった。
学び: no-move系で得た「回収余地30万以上へ絞るbiased swap」は、移動系のgrowth後処理にも9.6ms/ケースで移植でき、即時fee0.608M/ケースの約半分を最終スコア0.310M/ケースとして残した。局所改善はrelocationの代替ではなく、改善後形状をbaselineにする補完部品として有効である。

## v051_departure_compaction — 条件付き再検討: attemptを軽量化するか対象の効果を拡大できる場合に再検討

系譜: series=current; base=v050_move_strong_biased; imports=[]
当時の判定: 棄却。
仮説: 幾何棄却・断片化・growth 悪形の共通根は「空きの形の悪さ」であり (util 0.626 が最大の残損失)、退去で小空き成分が孤立したとき、境界グループを移動して空きを統合すれば、以後の受け入れ全体の前提が改善する。rescue と違い個別の受け入れでなく将来全体の前提を変えるため、相殺されにくい。
変更: v050_move_strong_biased を基準に、退去発生ターンの incoming 処理前に圧縮フェーズを追加する。小成分と最大成分の両方に隣接するグループを worst 以下の形 (fee_loss=0) で移し、跡地による成分接続を before/after の成分照合で確認し、incoming を先頭に加えたロールアウト差分 − 移動費 > 0 のときだけ実行する (1 ターン 1 件、時間は relocation 予算内でサブ上限 1/3)。同一ターンの二重移動は turn_moved で禁止する。admission・通常配置・relocation・biased swap は不変。
機構確認: `TraceStats` の `compaction_attempt`/`compaction_success` が正、`compaction_rollout_reject` を記録、圧縮採用時の最大空き成分の増分 `compaction_gain_sum` が正、`fallback_count=0`。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v050_move_strong_biased の 67,416,980 を上回り、最大経過時間が 1,450ms 未満で、上記の機構確認を満たしたら採用する。
結果: 機構は発動 (`compaction_attempt≈230〜360/ケース`、`compaction_success=4〜13`、`fallback_count=0`、max 1,372ms) したが、dry-run 100 ケースで worst 以下版 67,339,667 (v050 比 -0.115%)、worst+6 損失込み版 67,301,483 (-0.171%) の両構成とも基準未達。個別ケースでは 0006 +1.4%・0077 +0.7% と正の効果を確認した。
学び: 「動かせば接続でき行き先もある」状況は attempt の数% しかなく、成功 4〜13 回/ケースの利得を attempt 約 300 回分の free_info・ロールアウト時間 (relocation 予算と共有) が食い潰す。行き先の品質緩和は個別ケースを伸ばすが、短水平線ロールアウトで見えない長期劣化が他ケースで上回る。再開条件: attempt の軽量化 (差分 free_info・事前絞り込み) か、対象を小成分統合から大成分の形状改善へ広げ発動あたり効果を上げる場合。

## v052_adaptive_capacity — 条件付き再検討: 容量限界と断片化の交絡を切る観測を設計できる場合に再検討

系譜: series=current; base=v050_move_strong_biased; imports=[]
当時の判定: 棄却。
仮説: 実効詰め容量の静的式 (0.80〜0.89、池と成分数のみ) は、ケースの盤面形状と実際の詰め能力のずれを持つ。幾何棄却が起きた瞬間の占有率は「この盤面で実際に詰められる限界」の直接観測であり、その EMA で effective_capacity をケース内適応させれば、詰めすぎのケースだけ選別が適正化してスコアが上がる (v024 の一律 -2.5% が +0.349% だった容量感度のケース適応版)。
変更: v050_move_strong_biased を基準に、幾何棄却時の占有率 EMA (λ=0.15) を観測とし、観測数 n に応じた重み w=min(0.7, n/20) で packing 比を static と混合して effective_capacity を毎回更新する (margin +0.015、比は 0.72〜0.93 に clamp)。admission 式・配置・移動・ロールアウトは不変。
機構確認: `TraceStats` の `capacity_obs` が正、`adaptive_ratio_final_permille` を記録し、適応の入ったケースで static との乖離を確認、`fallback_count=0`。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v050_move_strong_biased の 67,416,980 を上回り、最大経過時間が 1,450ms 未満で、上記の機構確認を満たしたら採用する。
結果: 機構は発動 (obs 6〜107 回/ケース、比の適応、`fallback_count=0`) したが、無条件版は 0002 -2.7% (低占有断片化を限界と誤認し 828→796‰)、高占有フィルタ版 (static−0.06 未満の観測を棄却) でも 0006 -2.8% が残り、dry-run 100 ケースで 67,222,985 (v050 比 -0.288%) の基準未達。0077 では +0.95〜2.6% と正効果もあった。
学び: 幾何棄却の発生は容量限界・断片化・移動失敗の複合であり、占有率フィルタでは分離できない (Q2 の「幾何棄却の実体はほぼ断片化」の再確認)。特に低 R ケースでは高占有の幾何棄却すら移動で救えたはずの断片化で、容量を下げる適応が誤作動する。θ (v037/v038) と合わせ、ケース内オンライン推定は「入力の未知量は搾り尽くされ、自己能力の未知量は観測が交絡する」で決着。再開条件: 交絡を切れる観測 (移動探索を尽くした後の棄却のみ、等) を設計できた場合。

## v053_posterior_rollout — 後続への統合: posterior rolloutが現行solverへ存続

系譜: series=current; base=v050_move_strong_biased; imports=[]
当時の判定: 採用。
仮説: ロールアウトの将来到着は θ 点推定 (事後平均) で生成しており、序盤の θ 不確実性が評価に入っていない。K 本のロールアウトそれぞれで θ を事後分布からサンプルすれば (predictive rollout)、候補の優劣が θ に依存する場面で不確実性込みの正しい比較になる。
変更: v050_move_strong_biased を基準に、θ 事後グリッド (121 点) の累積分布を到着ごとに保存し、ロールアウト各本の生成 θ を事後サンプルへ置き換える。閾値計算・admission・配置・移動は不変 (点推定のまま)。
機構確認: `TraceStats` の `theta_sample_spread_permille`（サンプル θ の平均絶対偏差 / 点推定、×1000 の合計）が正で序盤ほど大きいこと、`fallback_count=0` を確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v050_move_strong_biased の 67,416,980 を上回り、最大経過時間が 1,450ms 未満で、上記の機構確認を満たしたら採用する。
結果: 100/100 ケース成功、平均絶対スコア 67,545,422（v050 比 +0.191%）で全体最高を更新、最大 1,374ms。0006 +2.9%・0077 +1.7%・0002 不変、`theta_sample_spread_permille=31k〜41k`、`fallback_count=0`。SD>300 で点推定へ戻すゲート版は 67,308,983 へ悪化したため、常時サンプルで確定した。
学び: θ の点推定精度には価値が残っていなかった (v037/v038) が、事後分布の幅には未回収の情報があった。K 本に別々の事後サンプル θ を使うと θ 依存の候補優劣が正しく混合され、収束後もサンプル多様性が判定を頑健にする (ゲートで消すと悪化)。「推定を正確にする」のでなく「不確実性を判断へ流し込む」のが有効なベイズの使い方である。閾値側はまだ点推定のままで、サンプル世界と閾値の整合 (θ_k 依存閾値) が次の一手。

## v054_sampled_threshold — 条件付き再検討: 明示的な負荷シナリオ分散で多様性を補える場合に再検討

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 棄却。
仮説: v053 はロールアウト到着列を θ_k サンプルで生成する一方、rollout 内の admission 閾値は点推定 θ の値を全本で共有しており、サンプル世界と閾値が不整合である (θ_k 大の世界では選別も強いはず)。各本の閾値を θ_k で再計算すれば、サンプル世界内の将来受け入れが自己整合し、候補比較の質が上がる。
変更: v053_posterior_rollout を基準に、evaluate_candidates_rollout の K 本ループで threshold_k = base_dynamic_threshold(S, duration, P, θ_k) を各本計算し、rollout_one へ渡す。到着列生成・候補構成・admission 本体・配置・移動は不変。
機構確認: `TraceStats` の `threshold_k_spread_permille`（threshold_k と共有閾値の相対乖離合計）が正、`fallback_count=0`、max_elapsed への影響が小さいことを確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v053_posterior_rollout の 67,545,422 を上回り、最大経過時間が 1,450ms 未満で、上記の機構確認を満たしたら採用する。
結果: 機構は発動 (`threshold_k_spread_permille=18k〜25k`、`fallback_count=0`) したが、v053 で +1.7〜2.9% だった 0077/0006 が -2.4%/-2.0% へ反転し、dry-run 100 ケースで 67,193,613 (v053 比 -0.521%) の基準未達。
学び: v053 の利得の源泉は θ 不確実性の「正しい伝搬」ではなく、θ_k が作る将来負荷シナリオの多様性である。閾値を θ_k に整合させると、θ_k 大の重い将来像は高閾値の選別で、θ_k 小の軽い将来像は低閾値の受け入れで相殺され、シナリオ間の占有差が消えて多様性が失われる。理論的整合が暗黙の多様性・保守性を壊して悪化する現象は v037/v038・SD ゲートに続き 3 例目で、この解法の構造的性質と考える。再開条件: 失われる多様性を明示的な負荷シナリオ分散で補う場合。

## v055_stratified_scenarios — 知見のみ有効: 決定的分位点はランダム事後サンプルに劣る

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 棄却。
仮説: v053 の利得の源泉は θ_k が作る将来負荷シナリオの多様性である (v054 で整合により消すと悪化)。ランダム事後サンプルを事後分位点 u=0.1/0.5/0.9 の決定的 3 シナリオへ構造化すれば、同じ多様性をサンプリング分散なしで安定に得られ、判定の質が上がる。
変更: v053_posterior_rollout を基準に、K 本ループの θ_k を「seed 乱数による事後サンプル」から「sample_theta(0.1/0.5/0.9)」の固定分位点へ置き換える。到着列の乱数 (P・滞在・q)・閾値 (共有点推定)・他は不変。
機構確認: `TraceStats` の `theta_scenario_width_permille`（90% 点と 10% 点の差 / 点推定）が正で序盤ほど大きいこと、`fallback_count=0` を確認する。
採否基準: `tools/in` 100 ケースが全成功し、平均絶対スコアが全体最高 v053_posterior_rollout の 67,545,422 を上回り、最大経過時間が 1,450ms 未満で、上記の機構確認を満たしたら採用する。
結果: 機構は発動 (`fallback_count=0`) したが、0077 -2.6%・0006 -2.5% (v053 比) と v054 同等の悪化で、dry-run 100 ケース 67,062,915 (v053 比 -0.714%) の基準未達。
学び: 有効なのは「セッション (到着) ごとに変わるランダムな θ 揺らぎ」だけで、固定分位点はセッション間多様性を失って悪化する。θ シリーズ総括 — 精度向上 (v037/38)・SD ゲート・閾値整合 (v054)・分位点構造化 (v055) はすべて負け、ランダム事後サンプル (v053) だけが勝った。θ の情報価値というより、決定的ロールアウト評価の系統バイアスを確率的揺らぎが壊す dithering 効果と解釈する。正確化・構造化系のロールアウト改良 (B-024 含む) は期待を下げ、θ 系実験はここで打ち切る。

## v056_persistent_owner — 条件付き再検討: owner点参照が少なく多数versionを保持する探索を採る場合に再検討

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 棄却。
仮説: 長方形に近い領域を `O(Q log²N)` で永続更新できるowner表現は、実際のv053で発生する配置・退去・blocker参照の混合でも、単点参照の悪化を上回ってsolver全体を高速化する。
変更: v053_posterior_rolloutを基準に、占有`Rows`・探索・パラメータを固定し、確定盤面の`owner_cell`だけをタイムスタンプ付き永続2次元遅延セグメントツリーへ置換する。各配置領域は同一水平runを縦結合した正確な長方形列に変換する。
機構確認: `owner_query`・`owner_assign`・`owner_rectangle`が正で、ランダム分岐照合と保存済み盤面リプレイを通過し、arena最大使用量を測る。`fallback_count=0`も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均CPU時間がv053の926msから5%以上短縮、最大1,450ms未満、平均絶対スコアが全体最高67,545,422を上回り、上記機構確認を満たしたら採用する。
結果: 100/100成功だが平均CPU 1,136ms（+22.6%）、平均スコア63,150,878（-6.51%）で基準未達。owner APIは3,245万回/ケース、relocation成功は67.3→27.4回、arenaは平均0.71MiB・最大2.14MiBだった。
学び: 確定盤面更新の節約は約1ms/ケースに対しowner点参照が数千万回あり、`O(log²N)`参照が探索量を削る。永続2次元木はowner参照をほぼ行わず多数versionを保持する専用探索以外では使わない。

## bench_cell_set — 現行採用: CellSetを共通基盤で維持する根拠として使用中

系譜: series=auxiliary; base=-; imports=[]
当時の判定: 採用。
仮説: `[u64; 40]`の`CellSet`は`[bool; 2500]`より単点参照では不利でも、V000で頻出するclone・集合演算・行抽出・領域列挙を含む混合処理では、圧縮と64bit並列処理により高速である。
変更: solverは変更・実行せず、v053保存済み出力の実領域と盤面を`CellSet`とbool配列へ同一変換し、単点操作、clone、集合演算、count、row_bits、compactness、領域配置、V000風clone+候補適用をreleaseで交互測定する。
機構確認: ランダム更新と全サンプルの全2500セル・各API・checksumが両表現で一致し、実領域の面積分布と各型のサイズを報告する。
採否基準: V000風混合処理で`CellSet`が高速で、集合演算・row_bits・領域列挙の主要項目でも優位なら維持する。bool配列が混合処理で勝つ場合は置換候補とする。
結果: v053実領域742件・盤面763件で全照合通過。boolはhot contains 0.314ns対0.459nsで速いが、CellSetはdisjoint 69.6倍、count 25.0倍、列挙15.1倍、clone 12.0倍、V000風混合84.2倍高速で基準達成。固定State payload cloneも1.74倍高速だった。
学び: 2,500セルでは単点だけならbool配列が僅かに勝つが、`CellSet`の役割は64セル並列の集合処理・疎列挙・小型cloneにある。V000の共通マス集合は`[u64;40]`を維持する。

## v057_deferred_relocation — 条件付き再検討: 赤字案の推定将来益を保存軌跡で較正できる場合に再検討

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 棄却。
仮説: 到着時の旧採算ゲートで落ちる再配置案の中に、通常配置または拒否より即時実額は低くても、後続の配置可能性を改善して既存ロールアウトの総価値では勝つ案があり、これを復活させるとスコアが上がる。
変更: v053_posterior_rolloutを基準に、従来の黒字target・repack・最大3案を固定する。旧target採算ゲートで落ちたがgross surplusは正の案を別枠でrank順に最大6回repackし、これと従来targetの最終採算落ちから、通常配置または拒否より即時実額が低い案を合計最大2案だけ既存の22到着×3ロールアウトへ追加する。他のadmission・配置・移動・時間予算・ロールアウトは不変。
機構確認: `deferred_target_option`・`deferred_plan_collected`・`deferred_rollout_win`が正で、`deferred_shortfall_sum`と採用案の`deferred_winner_shortfall_sum`を記録し、`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100成功、平均65,550,364（v053比-2.954%、15勝85敗）、平均CPU 1,041ms（+12.4%）・最大1,312ms。赤字案27,287件を収集して2,179件をrolloutが採用し、採用時短期赤字110.809M、`deferred_target_option=92,482,658`、`fallback_count=0`で機構は発動したがスコア基準未達。
学び: 現行22到着×3 quick rolloutは黒字案を棄却する差分検出器としては有効でも、短期赤字案を21.79件/ケース復活させる絶対評価には楽観的で、確実な即時損失を将来益で回収できなかった。再開条件: 保存軌跡分析で推定将来益と実現差分を較正できる場合。

## v058_pocket_packing — 条件付き再検討: 袋領域を線形時間で列挙できる場合に再検討

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 棄却。
仮説: 池・外周で囲まれた低汎用の小領域を同人数の小型groupで完全に埋めれば、以後動かさずに広く形のよい自由空間を大型group用に残せてスコアが上がる。
変更: v053_posterior_rolloutを基準に、静的草地の独立成分または1マス除去で大成分から切り離される4..63マスの半島を事前抽出し、`P`と容量が完全一致して全マスが空き、周長がv053の現在の走査レベルと一致する場合だけ同一候補集合へ追加する。周長走査・評価式・admission・relocation・rollout・時間予算は不変。
機構確認: `static_pocket_count`・`static_pocket_candidate`・`static_pocket_placed`・`static_lobe_placed`が正で、追加候補の面積一致・連結性・全マス空きをassertし、`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100成功、平均66,984,193（v053比-0.831%、5勝59分36敗）、平均CPU 954ms（+3.0%）・最大1,324ms。袋候補1,212件、選択102件、配置94件（半島63件）、`fallback_count=0`で機構は発動したがスコア基準未達。
学び: 袋候補自体が一度も出ない51ケースでも平均-0.910M（1勝25分25敗）なのに対し、配置した25ケースは平均-0.158M（4勝15分6敗）で、低下の大半は`O(|G|²)`静的列挙が時間制御を攪乱した交絡であり幾何仮説の否定にはならない。再開条件: Tarjan法など線形時間の列挙で前計算負荷を除く場合。

## v059_theta_map — 条件付き再検討: 多様性を保つ軽量サンプラの費用削減を先に実証できる場合に再検討

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 棄却。
仮説: `theta`の121点事後平均・累積分布更新を文書どおりの連続MAPへ置換すれば、推定・サンプリング計算を除いて時間制御下の探索量を増やし、最頻値だけでも負荷推定に十分なためスコアとCPU時間を同時に改善できる。
変更: v053_posterior_rolloutを基準に、`theta_cum`・`theta_sd`・121点評価を削除し、`Y_n=sum(D[i]-1)`と`n`をO(1)更新して`clip(Y_n/n,2000,8000)`を全theta利用箇所および全rolloutへ渡す。rolloutは各本・各仮想到着で同じMAPを使い、3本×22到着、乱数列、admission・配置・移動・全時間予算は不変。
機構確認: `theta_map_turn=100000`、`theta_map_rollout`が正で、`theta_map_value_milli`を記録し、旧121点事後分布の状態・サンプリング経路がコードから消えており、`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、平均CPU時間が926ms未満、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100成功、平均66,841,809（v053比-1.042%、34勝3分63敗）、平均CPU 944ms（+1.9%）・最大1,313ms。`theta_map_turn=100000`、`theta_map_rollout=82017`、`fallback_count=0`で機構は発動したがスコア・平均CPU基準とも未達。
学び: 推定器自体をO(1)化してもMAPによる判断変化で`fast_mode_turn`が1,563→5,215、relocation成功が6,728→6,487、受理が63,107→62,777となり、solver全体のCPUは減らなかった。v053の事後サンプル多様性を捨てる最軽量MAPにはrollout予算を再配分する原資がない。再開条件: 多様性を保つ軽量サンプラの単体費用削減を事前に実証できる場合。

## v060_groupwise_theta — 知見のみ有効: group別theta多様化はMAP固定化の損失を回収しない

系譜: series=current; base=v059_theta_map; imports=[]
当時の判定: 棄却。
仮説: v059で失った事後多様性を仮想到着groupごとの正確な連続事後サンプルで戻せば、MAP固定ロールアウトの偏りが減り、約3ms/ケースの見積追加費用内で現行最高v053を上回る。
変更: v059_theta_mapを基準に、通常判断のMAP、3本×22到着、候補・admission・配置・移動・時間予算を固定し、各rollout内の仮想到着groupごとに`p(theta|D)∝theta^{-n}exp(-Y_n/theta)`からlog-theta一様提案の棄却法で独立に正確サンプルして滞在時間を生成する。
機構確認: `theta_map_turn=100000`、`theta_exact_group_sample`・`theta_exact_proposal`・`theta_exact_adjacent_change`が正で、サンプルが常に`[2000,8000]`内、`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100成功、平均66,906,974（v053比-0.945%、30勝2分68敗、v059比+0.097%）、平均CPU 939ms・最大1,312ms。正確サンプル1,792,076件、15.118提案/件、MAPからの平均絶対偏差240.859、隣接変化1,709,764件、`fallback_count=0`で機構と時間基準は達成したがスコア基準は未達。
学び: 正確なgroup別サンプルは速度上の障害ではなかったが、v059比+0.097%に留まりv053との差の9.3%しか回収しなかった。ケース内で共有される真のthetaをgroupごとに独立化する多様化は採用しない。

## check_prefix_map — 知見のみ有効: prefix補正推定器の精度と計算量の単体根拠として有効

系譜: series=auxiliary; base=-; imports=[]
当時の判定: 採用。
仮説: 7次prefix尤度と逐次更新によるMAP推定が、同じ目的関数の大域MAPへ十分近く、未補正推定の中盤biasを軽量に減らせる。
変更: solverは変更・実行せず、100ケースの全10万prefixについて逐次MAPと大域MAPを照合し、真のthetaに対する中盤推定誤差とCPU時間を測定する。
機構確認: 全prefixで目的関数差と数値異常を検査し、区間探索とNewton更新が有効範囲内の推定値を返すことを確認する。
採否基準: 事前登録はjournalに残っておらず、既存記録からは確認できない。
結果: 逐次MAPと大域MAPの差は平均絶対0.420、最大73.85、目的関数損失は平均0.000113、最大0.184で、数値異常は0件、100ケース合計CPUは47.8msだった。
学び: prefix補正はgroup 101–250のbiasを+261.6から-13.2へ、RMSEを476.5から347.7へ縮小し、推定器単体では十分軽量かつ正確である。ただし、v061により、この統計的改善が現行solverの得点改善を保証しないと確認された。

## v061_prefix_map_rollout — 知見のみ有効: 推定biasの改善は現行solverの得点改善を保証しない

系譜: series=current; base=v059_theta_map; imports=[check_prefix_map]
当時の判定: 棄却。
仮説: 未到着groupのprefix尤度を含む7次近似MAPで中盤以降のtheta上方バイアスを減らし、そのthetaからrolloutの各仮想到着groupの滞在時間をサンプルすれば、v059の軽量性を保ったまま将来占有の系統誤差が減る。
変更: v059_theta_mapを基準に、候補・admission・配置・移動・3本×22到着・乱数列・時間予算を固定し、実到着時のtheta推定だけを`input_distribution.md`記載の7次prefix尤度、最初32groupの12回区間探索、以降の前回値始動2回保護付きNewtonへ置換する。Beta事前分布やtheta事後サンプルは導入せず、得た単一MAPを共有して各rollout内の`l[i]`を従来どおり指数分布から個別サンプルする。
機構確認: 単体100ケースで逐次MAPと同じ7次目的関数の大域MAPとの差を照合した上で、`theta_prefix_map_turn=100000`、`theta_prefix_interval_turn=3200`、`theta_prefix_newton_turn=96800`、`rollout_l_sample`が正、数値異常と`fallback_count`が0であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: 100/100成功、平均66,590,713（v053比-1.413%、40勝0分60敗、v059比-0.376%、54勝1分45敗）、平均CPU 963ms・最大1,320ms。`theta_prefix_map_turn=100000`、区間3,200・Newton 96,800、保護step 168、`rollout_l_sample=1777565`、平均theta補正-255.412、`fallback_count=0`で機構と最大時間基準は達成したがスコア基準は未達。
学び: prefix補正は推定biasを減らした一方、v059よりrolloutのthetaを平均255.412下げてスコアを0.376%下げた。統計的に正確な推定とsolverに有利な予測値は一致せず、現行構造では大きめthetaのrolloutが有利という理論を一側面から支持する。

## v062_no_move_deadline_shelves — 知見のみ有効: ハードな棚分割はコンパクト配置を壊す

系譜: series=no_move; base=v047_no_move_strong_biased; imports=[]
当時の判定: 棄却。
仮説: 池地形に適応した一方向の棚へ退去時刻が奥から手前へ単調減少するように詰めれば、退去後の内部空洞を防いで大きな空き連結成分を維持し、形状理由の拒否を減らせる。
変更: v047_no_move_strong_biasedを基準に、移動0・admission・因果veto・capacity ratio・theta推定・fast modeを固定し、全配置を方向適応型のprefix棚候補へ制限する。合法候補がなければ通常配置へ戻らず拒否する。
機構確認: 棚方向、候補検査・合法数、prefix穴・退去時刻順違反、regular/growth、新規棚数、deadline slack、拒否理由、不変条件検査をtraceし、違反0、`move_zero_output=100000`、再配置0を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: case 0000の単ケース機構確認は正常終了し、公式スコア22,034,892、solver CPU 1,408ms。方向は下→上、regular 394件・growth 172件を配置し、棚不変条件違反0、`move_zero_output=1000`、再配置0、`fallback_count=0`だった。100ケース評価は未実施。
学び: 行・列ごとのprefixと退去時刻順を全配置へ強制すると、盤面を独立レーンへ分断し、棚frontierのずれが恒常的な空き列になる。複数棚をまたぐコンパクト形状も排除するため、これは大きな空き連結成分を残す配置の検証にならない。今後は物理的な棚分割を置かず、コンパクト度と大きな空き連結成分を同時に保つ構成を考える。

## v063_no_move_size_gradient — 未決着: 実験中

系譜: series=no_move; base=v047_no_move_strong_biased; imports=[]
当時の判定: 未判定。
仮説: 同じ大きさのgroupを対角方向の近い領域へ緩く集めれば、小型groupが大型用空間へ散在するのを抑え、compact形状を維持したまま隙間と幾何棄却を減らせる。
変更: v047_no_move_strong_biasedを基準に、移動0・admission・因果veto・shape/growth/SA/LNS/biased・slot・rollout・時間比率を固定する。草地を`r+c,r,c`順に並べた面積rankと`p*Pr(P=p)`の累積中点を対応させ、最初に置ける周長は変えず、regularの同周長評価とgrowthの基準winnerと同周長の完成候補比較だけへ重み`12*sqrt(P)`の二乗距離penaltyを加える。
機構確認: 選択領域の面積rankを`P<=31`・`32<=P<=95`・`P>=96`で記録し、平均rankが順に増えること、位置penalty・目標誤差・regular/growthの比較と反転が正、選択周長契約、`move_zero_output=100000`、`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大経過時間が1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: `tools/in` 100ケースは全成功し、平均スコアはv047比-745,098（-1.122%）、記録済み最高のv053比-1,881,999（-2.786%）、最大solver CPUは1,417msだった。regularは54,311比較中10,048反転して位置誤差を16.967%削減し、growthは26,394比較中527反転して0.196%削減した。選択領域の平均面積rankはsmall 0.352、mid 0.513、large 0.534と単調増加し、選択周長契約の違反0、`move_zero_output=100000`、`fallback_count=0`だった。全件成功・時間・機構確認は基準を満たしたが、最高平均スコア超過は未達だった。
学び: 未確定。

## v064_no_move_contact_sync — 未決着: 実験中

系譜: series=no_move; base=v047_no_move_strong_biased; imports=[]
当時の判定: 未判定。
仮説: v047の同時存在時間による接触価値を維持しながら退出時刻の近さを小さく加点すれば、現在のcompact性を損なわず、隣接領域が近い時刻に解放されて後続の幾何棄却が減る。
変更: v047_no_move_strong_biasedを基準に、移動0・admission・因果veto・shape/growth/SA/LNS/biased・slot・rollout・合法候補列挙・最初に置ける周長・時間比率を固定する。既存groupとの共有辺だけへ、係数1.0の`overlap_ratio*exp(-abs(T[i]-T[g])/theta)`を現行の係数10.0の時間積分接触項に加え、壁・池の評価は変えない。
機構確認: `departure_sync_evaluated_edges`・`departure_sync_bonus_micros`・`departure_sync_selected_edges`・`departure_sync_selected_bonus_micros`が正で、case 0000の出力が保存済みv047から変化すること、`move_zero_output=100000`、`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。v047の平均66,408,521との差、受入数、幾何棄却数も診断する。
結果: `tools/in` 100ケースは全成功し、平均スコアはv047比-431,322（-0.649%）、記録済み最高のv053比-1,568,223（-2.322%）、最大solver CPUは1,418msだった。退出同期bonusは20,049,980共有辺で評価され、採用配置550,542共有辺のbonus総量は242,984.147、100ケースすべての出力が保存済みv047から変化した。受入数は61,104→60,657、幾何棄却は18,342→19,043、`move_zero_output=100000`、`fallback_count=0`だった。全件成功・時間・機構確認は基準を満たしたが、最高平均スコア超過は未達だった。
学び: 未確定。

## v065_no_move_canonical_rollout — 未決着: 実験中

系譜: series=no_move; base=v047_no_move_strong_biased; imports=[]
当時の判定: 未判定。
仮説: rollout内の将来配置を一つのcanonical near-rectangle形状族へ縮約すれば、候補盤面の幾何差を保ったまま1シナリオを軽量化でき、同じ時間で共通乱数シナリオを増やして通常配置の選択精度を上げられる。
変更: v047_no_move_strong_biasedを基準に、移動0・実配置・admission・因果veto・shape/growth/SA/LNS/biased・slot・22到着水平線・時間比率を固定する。rolloutの`quick_place`だけを、面積`P`、最小周長、外接長方形の余白、縦横差の辞書順で選んだcanonical near-rectangleとその回転へ限定し、最大長までのrun maskだけを構築する。保存盤面benchの現行/新方式時間比を使い、`floor(3*時間比)`を3以上9以下へ丸めた本数だけ共通乱数シナリオを使う。
機構確認: 補助checkで全`P=4..150`のcanonical形状が面積`P`・連結・最小周長であることを確認し、保存盤面benchで速度と受入一致率を記録する。solverではcanonical call/success/reject・形状検査数・scenario数・通常rollout反転が正、`move_zero_output=100000`、`fallback_count=0`を確認する。
採否基準: 補助benchでシナリオ本数が4以上となり、受入一致率が90%以上の場合だけsolverを評価する。`tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。v047の平均66,408,521との差も診断する。
結果: 保存済みv047 case 0000の補助benchはcanonical kernelが現行比2.227倍、受入可否一致率95.6%で、事前規則からscenario数を3本から6本へ確定した。`tools/in` 100ケースは全成功し、平均スコアはv047比+263,854（+0.397%、61勝39敗）、記録済み最高のv053比-873,047（-1.293%）、最大solver CPUは1,706msだった。受入数は61,104→61,465、100ケースすべての出力がv047から変化した。一方、保存された`.err`は全ケースで公式scoreのみとなりcanonical・scenario・通常rollout・move・fallbackのsolver traceを確認できなかった。最高平均スコア、最大時間、solver機構確認の基準は未達だった。
学び: 未確定。

## v066_dynamic_gap_relocation — 未決着: 実験中

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 未判定。
仮説: 将来到着に使いにくい小空き成分へ既存groupを能動的に移し、その旧領域を最大空き成分へ返せば、総空き面積と既存groupのcompactnessを保ったまま有用な連結空間が増え、幾何棄却が減る。
変更: v053_posterior_rolloutを基準に、admission・通常配置・既存relocation・biased swap・posterior rollout・時間比率を固定する。退去発生時だけ最大成分以外の4〜149マスの空き成分を列挙し、旧領域が最大成分に接するactive groupとのpairをtight-fit・低移動費順に最大8件調べる。現在のworst perimeter以下で隙間内に収まり、移動後の最大成分が厳密に増える候補を最大3件残し、現在のincomingを先頭に含む共通乱数rolloutでbaselineと比較して1件だけ移動する。gap処理はrelocation累積予算の20%までとする。
機構確認: `gap_reloc_trigger/gap_count/pair_checked/fit/topology_candidate/rollout/success`が正で、gap内配置、Pセル・連結・空き、総空き数不変、worst perimeter非悪化、最大成分厳密増加、同一ターン二重移動なしの全assertを通す。`gap_reloc_gain_sum`・移動費・残余gap・処理時間を記録し、`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが現行最高のv053_posterior_rolloutの67,545,422を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。v053との差、受入数、幾何棄却数、既存relocation回数も診断する。
結果: `tools/in` 100ケースは全成功し、v053比-948,665（-1.404%、30勝70敗）、最大solver CPUは1,307msだった。gap移動は33,734回の発動判定、223,333 pair検査、47,547 topology候補、19,158 rollout比較から2,900回採用され、最大空き成分を合計108,167マス増やした。gap処理は合計13.524秒、移動費12,987,174、移動後の残余gapは合計60,056マス、不変条件違反0、`fallback_count=0`だった。v053比で受入数は412件減り、幾何棄却は688件増え、既存relocation成功は610回減った。全成功・時間・機構確認は基準内だが、平均スコア基準は未達である。
学び: 未確定。

## v067_posterior_long_stay_veto — 未決着: 実験中

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 未判定。
仮説: `D/theta>=2`の長期groupは到着数では少数でもセル時間の大部分を占め、将来の配置を長期間固定するため、価値や現在盤面を見ず一律拒否して回転率を上げれば、失う利用料より後続受入価値と幾何余力が大きくなり現行最高を更新できる。
変更: v053_posterior_rolloutを基準に、配置・移動・admission価格式・posterior rollout・候補・時間比率を固定する。到着済みprefixから従来どおり推定した`theta`に対し`D/theta>=2`なら、価格prefilterや形状探索より前に必ず拒否する。`V`・`P`・混雑・配置位置による例外は設けない。
機構確認: `long_stay_veto_considered=100000`、`long_stay_veto_executed`が正で、全対象が`D/theta>=2`、対象がnormal search・受入・geometry rejectへ入らないことをassertする。対象数・`P`・`D`・セル時間・`V`・品質帯、受入数・幾何棄却・移動回数、`fallback_count=0`も記録する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが現行最高のv053_posterior_rolloutの67,545,422を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。v053との差、勝敗、受入数、幾何棄却数、移動回数も診断する。
結果: `tools/in` 100ケースは全成功し、平均はv053比`-16.788%`で0勝100敗、最大solver CPUは1,291msだった。hard vetoは12,285/100,000件で全件が専用棄却へ入り、通常探索・再配置・幾何棄却へ流れず、`fallback_count=0`だった。受入数は63,107→63,038、幾何棄却は19,420→10,512、移動数は8,929→6,241となり、スコア基準は未達、時間・機構基準は達成した。
学び: 未確定。

## v068_balanced_repack — 未決着: 実験中

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 未判定。
仮説: 複数blockerの移動先を逐次確定せず、全blockerの旧領域と空きセルを同時に再分割すれば、先行blockerが後続の逃げ場を塞ぐ失敗を避け、履歴最悪周長を抑えた採算可能なrelocationを増やせる。
変更: v053_posterior_rolloutを基準に、候補列挙・blocker上限4・最大12 attempt/3 plan・admission・通常配置・posterior rollout・時間比率を固定する。`repack_blockers`の逐次beamと規則形状不成立時の周長無制限growth fallbackを削除し、固定groupとincoming targetを除いたdomainへ、P降順に旧領域優先のseedを置き、一回の多始点priority growthで各blockerへ正確に`P`セルずつ割り当てる。履歴最悪周長の利用料低下は各利用料を四捨五入した整数差で計算する。
機構確認: `balanced_repack_attempt/seed_success/growth_success/assigned_cells/old_cells_reused/exact_fee_loss`が正で、成功ごとに各領域が芝生上の正確な`P`セル・連結・相互非重複・incoming/fixed group非重複であることを検査する。既存`repack_success/relocation_success/rollout_*`、`fallback_count=0`、逐次beamとfallbackの不在も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。10%増の74,299,965到達、v053との差・勝敗、repack/relocation成功数、移動費、履歴料金低下も別途照合する。
結果: 初回評価はC++由来の全blocker frontier更新が次数0セルを許して連結性assertに失敗し、100/100件`interactive_fail`だった。ユーザー指示後に自blocker frontierだけを更新して次数正を必須化した再評価は100/100件成功、平均65,621,228（v053比-1,924,194、-2.849%、16勝84敗）、最大1,313msで、10%目標には8,678,737届かなかった。構築58,473/試行187,827件が全検査を通り、次数0割当・fallback・時間中断0、relocation成功3,744件、移動費24,126,836、選択プランの履歴料金低下147,891,505だった。全成功・時間・機構基準は達成し、最高平均超過は未達だった。
学び: 未確定。

## v069_adaptive_time_budget — 後続への統合: 進捗予測によるadaptive探索水準が現行solverへ存続

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 棄却。
仮説: V053 は高負荷caseでrelocation予算が律速だが、軽いcaseにも同じ候補数・試行上限を課している。進捗から全体とrelocationの完走時間を予測し、余裕時だけ既存候補を保ったまま探索を深くすれば、低リスクで追加の良い通常配置・repack案を rollout 比較へ渡せる。
変更: v053_posterior_rolloutを基準に、400ターン以降で予測全体CPUと予測relocation CPUが閾値内のときだけlevel 1/2を選ぶ。通常配置はV053のtop 20を必ず残し、同一周長・同一成分で局所scoreまたは断片化が厳密優越する追加候補をtop 28/40から既存posterior rolloutへ加える。relocationは既存12試行・最大3案を保ち、level別に17/22試行へ増やす。他の判断・乱数・時間上限は固定する。
機構確認: `adaptive_level1_turn/level2_turn`、`adaptive_normal_search/candidate/selected`、`adaptive_repack_attempt/plan_collected/relocation_selected`が正で、追加候補・追加planは既存候補を削らず既存rolloutで選ばれること、`fallback_count=0`、最大solver CPU 1,450ms未満を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: `tools/in` 100/100成功、平均67,127,264（v053比-418,158、-0.619%）、平均solver CPU 951ms・最大1,315ms。level 1/2は12,720/8,179 turn、追加normal候補519件中14件選択、追加repack 8,098回から58 plan・30 relocationを選択し、`fallback_count=0`だった。全成功・時間・機構確認は満たしたが、最高平均スコア超過は未達。
学び: 追加案を選ばなかった74ケースだけでv053比合計-41.45Mとなり、追加探索が既存壁時計を進めて基準探索を置換した影響が支配的だった。一方、追加relocationだけを選んだ15ケースは合計+0.805Mであり、余剰探索は基準時計から分離して試す価値がある。

## v070_spare_time_deep_repack — 未決着: 実験中

系譜: series=current; base=v053_posterior_rollout; imports=[v069_adaptive_time_budget]
当時の判定: 未判定。
仮説: V053の判断順と時間判定を保ったまま、進捗別の実時間目標より余裕があるcaseだけで失敗済み上位targetのrepackを深く再試行すれば、平均CPUを約1,300msまで有効利用し、基準経路を置換せず追加の採算可能なrelocationを得られる。
変更: v053_posterior_rolloutを基準に、最初の12 target・最大3 plan・K=3/22 rolloutを先に固定どおり実行する。plan不足時かつ進捗別1.40秒目標より実時間が前なら、失敗済み上位targetだけをbeam幅18・分岐8・shape上限16・growth seed上限40で再試行する。追加処理中はV053用仮想時計を停止し、実時間1.48秒を全体hard stopとする。通常配置・admission・候補順位・基準repack・乱数は固定する。
機構確認: `adaptive_deep_repack_attempt/success/plan_collected/relocation_selected`と`adaptive_extra_time_us`が正で、深掘りは基準12試行後かつplan不足時だけ発動し、仮想経過時間が追加処理中に進まないこと、`adaptive_hard_stop`と`fallback_count`が0、平均solver CPUが1,200〜1,400msであることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。
結果: `tools/in` 100/100成功、v053比-430,162（-0.637%、27勝12分61敗）、平均solver CPU 1,042ms・最大1,382msだった。深掘りは80,568回試行、54,189回repack成功、448 planを得て129回選択され、追加処理は平均116.6ms、実時間/仮想時間は平均1,112.3/995.7ms、`adaptive_hard_stop=0`、`fallback_count=0`だった。全成功・深掘り発動・最大時間は基準内だが、最高平均超過と平均CPU 1,200〜1,400msは未達だった。
学び: 未確定。

## v071_articulation_growth — 現行採用: 現行solverとして使用中

系譜: series=current; base=v069_adaptive_time_budget; imports=[v035_no_move_growth_cutloss]
当時の判定: 採用。
仮説: 通常配置のgrowthで共有辺数を最優先に保ったまま、同率候補から空き成分を大きく切断しないセルを選べば、コンパクトさを犠牲にせず将来の連結roomを保ち、受入数と移動費を改善できる。
変更: 外部C++版をRustへ移植する。v069_adaptive_time_budgetを基準に、admission・posterior rollout・adaptive探索水準・relocation/repack・時間比率を固定し、通常配置のgrowth（seed上限44）だけへTarjan low-linkによる関節点損失を、共有辺数の次・既存距離/attractionの前のtie-breakとして加える。repackのgrowth（seed上限20）は従来順序のままとする。
機構確認: `cutloss_turn`・`cutloss_positive_turn`・`cutloss_seed_changed`・`cutloss_positive_selected`が正で、`cutloss_turn`は通常growth呼出し時だけ増え、全ケースの合法出力と`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv053_posterior_rolloutの67,545,422を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。v069との差、受入数、reject数、移動費、ケース別勝敗も診断する。
結果: `tools/in` 100/100成功、記録済み最高v053比+44,780/ケース（+0.0663%、54勝46敗）、base v069比+462,939/ケース（+0.6896%、62勝38敗）、平均/最大solver CPU 968/1,308msだった。v069比で受入+314、reject-314、移動費-528,431。`cutloss_turn=29,075`、同positive=29,075、`cutloss_seed_changed=555`、`cutloss_positive_selected=4,911,074`、`fallback_count=0`で、事前登録した機構確認と全機械的採否条件を満たした。後日、AtCoderの50ケース採点ではv053を下回った（絶対スコアは未記録）。
学び: local 100ケースの+0.0663%はAtCoderの別50ケースへ一般化せず、関節点tie-breakを本質的改善とみなす根拠には不足する。今後、同程度の小差だけでは主力採用を確定せず、外部採点との整合が取れるまで効果規模を誤差候補として扱う。

## v072_anytime_holdout_rollout — 未決着: 評価済み・判定待ち

系譜: series=current; base=v071_articulation_growth; imports=[v070_spare_time_deep_repack]
当時の判定: 未判定。
仮説: v071の通常配置winnerを先に固定し、余った時間だけで同一料金・同一受入条件の未使用候補を独立rolloutすれば、基準探索を置換せず空間配置の判断精度を上げ、早く終了するcaseの時間を得点へ変換できる。
変更: v071_articulation_growthを基準に、通常配置の現行choicesとK=3/22 rolloutを固定してwinnerを先に確定する。実時間が進捗比例の1.32秒目標より前なら、局所評価済みtop20のうち同一周長・同一componentの未使用候補を最大12個、freshな共通乱数でscreeningし、最良challengerだけを別seedのpaired holdoutで基準winnerと再比較する。追加処理時間は基準仮想時計から除外し、実時間1.48秒をhard limitとする。admission・候補生成・関節点growth・adaptive水準・relocation/repack・基準乱数は固定する。
機構確認: `anytime_pool_session/candidate`・`anytime_screen_session/candidate`・`anytime_holdout_session`・`anytime_selected`・`anytime_extra_time_us`が正で、screening対象が基準winnerと同一周長・同一componentかつ重複なし、holdoutが基準winnerとscreening最良challengerだけをfresh paired seedで比較し、追加時間だけ仮想時計から除外されること、`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv071_articulation_growthの67,590,203を上回り、平均solver CPUが1,150〜1,400ms、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。v071との差、ケース別勝敗、受入数、reject数、移動費も診断する。
結果: `tools/in` 100/100成功、現行v071比-105,486/ケース（-0.1561%、48勝52敗）、平均/最大solver CPU 1,112/1,359msだった。未使用候補103,612件を14,804 sessionでscreeningし、screening勝者1,678件を同数のpaired holdoutへ送り619件を選択した。追加時間は平均147.4msで、実時間/仮想時間は平均1,192.5/1,045.1ms、`anytime_hard_stop=0`、`fallback_count=0`だった。全成功・最大時間・機構確認は満たしたが、最高平均超過と平均CPU 1,150〜1,400msは未達だった。
学び: 未確定。

## calculate_temporal_upper_bounds — 現行採用: 時間容量上限をケース別スコア基準として使用中

系譜: series=auxiliary; base=-; imports=[]
当時の判定: 採用。
仮説: 各groupの最大利用料と滞在区間を使い、時刻ごとの芝生容量内でgroupを分割採択する最大値を求めれば、総セル時間だけの上限より狭く、空間配置の難しさを残差として読めるケース別スコア基準を再現可能に計算できる。
変更: solverは変更・実行せず、`tools/in`を読み、最大コンパクト度、時刻区間ごとの芝生容量、groupの分割採択を組み込んだ最小費用流で時間容量上限を求める補助binと、定義・定式化・全ケース一覧を記した[文書](deep/temporal_capacity_upper_bounds.md)を追加した。
機構確認: 各ケースで送流量が`G(空きマス)`と一致し、緩和後の占有量が全時間区間で容量以下、case 0000の上限が79,989,376点と一致し、保存済み正常スコアが上限値以下であることを検査する。
採否基準: `tools/in` 100ケースすべての上限値を決定的に算出し、全機構確認を通し、文書が上限になる根拠、計算の定式化、推定`θ`を含む全ケース一覧、現行v053との差を単独で再現可能な形で含めば補助基準として採用する。
結果: 100/100ケースを算出し、上限は平均101,830,218点、v053の合計到達率は66.33%だった。case 0000の推定`θ`は2,811、`G(空きマス)`は1,871、上限は79,989,376点だった。容量違反と保存済み正常スコアの上限超過は各0件で、補助binの単体検査5件と文書表100行の再計算照合を通した。
学び: 時刻ごとの容量制約により、総セル時間だけでは数えていた同時需要超過を除ける。上限に対するv053の合計到達率は66.33%で、残差にはgroup不可分性と空間配置制約が混在する。次に狭める場合は整数採択、連結成分容量、配置可能形状の順に制約を戻す。

## v_01_offline_reference — 現行採用: 未来既知practical基準として使用中

系譜: series=auxiliary; base=-; imports=[]
当時の判定: 採用。
仮説: 全groupを事前に読み、固定領域の基礎計画に加えて、追加groupの滞在中だけ衝突groupを一時退避する非重複の再配置区間を選べば、移動費と履歴最低コンパクト度を差し引いても固定領域基準を上回るpractical基準を得られる。
変更: 提出solverとは独立した単一ファイルのoffline補助binを用い、多スタート基礎計画、単体・衝突group置き直し、追加groupごとの退避先探索、利益最大の非重複再配置区間選択を行う。別の補助実行器で100ケースを直接読み、出力、スコア、[文書](deep/future_known_practical_upper_bounds.md)を保存する。
機構確認: 全ケースで60秒探索が完了し、内部replayと公式scorerを通り、移動回数・移動費・追加受入利益が記録され、保存出力をvisualizerで各時刻へ再生できることを確認する。
採否基準: `tools/in` 100ケースが全て合法で、各ケースのpractical基準、推定`θ`、`G(空きマス)`、時間容量上限、v053との差を文書化し、case 0000で移動が発動して既存の固定領域基準63,789,069点を上回れば補助基準として採用する。
結果: 100/100ケースの出力を公式規則で再生した。ケース別基準は平均73,102,563点でv053比+8.23%、時間容量上限への到達率71.79%だった。89ケースに移動があり、case 0000は32回の移動で63,951,466点となって採否基準を満たした。
学び: 未来既知の合法配置まで戻すとv053から平均8.23%の改善余地が残る。非重複の再配置437区間だけでも固定基礎計画から合計9,943,660点増えた一方、負荷率1.9以上では時間容量上限への到達率が68.78%まで下がり、高負荷時の幾何制約がなお支配的である。

## v074_expected_terminal_load — 知見のみ有効: ユーザー再指定に対して既存評価を正式報告

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 中断。
仮説: v053の有限端需要を時刻別offered loadとoccupancy目標の双方へ入れる代わりに、オンラインの期待絶対需要から道中価格を一度だけ決め、既知`[S,T]`の終端需要減衰を一つの平均係数として一度だけ掛ければ、終端を二重に価格化せず高価値の終端滞在groupを適切に受け入れられる。
変更: v053_posterior_rolloutを基準に、残り到着率`(M-i-1)/(100000-S[i])`・`E[P]`・posterior `theta`から道中offered areaを求め、その容量比の価格へ離散平均`w_end(t)=1-exp(-(100000-t)/theta)`を一度だけ掛ける。既存`boundary_load_factor`・時刻別`local_bid_at`を削除し、occupancy補正の目標にも終端係数を入れない。`D^0.1`の価値密度換算、配置、K=3/22 posterior rollout、relocation、時間配分は固定する。
機構確認: `expected_load_turn`・`expected_load_no_future`・`expected_load_under_capacity`、`terminal_discount_turn`・`terminal_price_discounted_turn`、`terminal_weight_ppm_sum`が整合し、最終groupで道中価格0、終端係数の乗算箇所が一つ、`boundary_load_factor`と`local_bid_at`が不在、既存`rollout_*`が発動し、`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv071_articulation_growthの67,590,203を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。base v053との差、ケース別勝敗、受入数、reject数、移動費も診断する。
結果: 同一実装を再評価せず既存の1回分を正式照合した。100/100ケース成功、平均66,395,618でv053比-1.7023%（25勝1分74敗）、現行最高v071比-1.7674%（15勝0分85敗）、平均CPU 956ms・最大1,315msだった。受入はv053比+394、移動費-442,066で、期待負荷・終端・posterior rolloutは発動し、最終groupの道中価格0、`fallback_count=0`だったため、機構・合法性・時間基準は達成したが平均スコア基準は未達だった。
学び: v053の推定・探索を維持したまま期待絶対需要と単一終端係数へ置換すると受入数と移動費は改善方向に動くが、平均スコアはv053を1.7023%下回る。旧thetaの選択だけが主因ではなく、admission価格モデル自体の差が支配的である。

## v075_prefix_terminal_load — 未決着: 実験中

系譜: series=current; base=v074_expected_terminal_load; imports=[v061_prefix_map_rollout]
当時の判定: 未判定。
仮説: 終端admissionの期待絶対需要と終端平均係数はthetaへ直接依存するため、未到着groupのprefix情報を含む7次補正MAPで中盤の上方biasを除けば、道中価格と終端補正が同じ有限horizon生成モデルに整合し、現行最高を上回る。
変更: v074_expected_terminal_loadを基準に、期待需要式・単一終端係数・配置・3本×22到着・relocation・時間配分を固定し、prefix未補正の121点posterior平均・thetaサンプルをv061と同じ7次prefix補正MAPへ置換する。最初32groupは12回区間探索、以降は前回値始動2回保護付きNewtonとし、得た単一thetaを全判断とrolloutで共有する。
機構確認: `theta_prefix_map_turn=100000`、`theta_prefix_interval_turn=3200`、`theta_prefix_newton_turn=96800`、`rollout_l_sample`と終端・期待負荷traceが正、posterior gridとthetaサンプル経路が不在、終端係数の乗算箇所が一つ、`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv071_articulation_growthの67,590,203を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。v053・v074との差、ケース別勝敗、受入数、reject数、移動費も診断する。
結果: 100/100ケース成功、平均は現行最高v071比-1.7600%（16勝0分84敗）、v053比-1.6949%（33勝0分67敗）、旧thetaのv074比+0.0075%（57勝0分43敗）、平均CPU 940ms・最大1,311msだった。prefix推定は全ケースで区間32回・Newton 968回、合計保護step 168、期待負荷・終端traceとrollout滞在サンプルは発動し、最終groupの道中価格0、`fallback_count=0`で、機構・合法性・時間基準は達成したが平均スコア基準は未達だった。
学び: 未確定。ユーザー判定待ち。

## v076_simple_terminal_weight — 未決着: 実験中

系譜: series=current; base=v053_posterior_rollout; imports=[v074_expected_terminal_load]
当時の判定: 未判定。
仮説: B-089の低下はv053の道中価格を期待絶対需要へ全面置換した影響が交絡している。v053の序盤補正と3点価格校正を残し、既存の非線形な終端負荷経路だけを区間平均`W_end`の一回乗算へ置換すれば、終端で長く価値を生むgroupを過剰reserveせず現行最高を上回る。
変更: v053_posterior_rolloutを基準に、`boundary_load_factor`の終端側`1-exp(-(100000-t)/theta)`を3点`local_bid_at`とoccupancy目標の双方から除き、序盤側`1-exp(-t/theta)`だけを残す。その3点平均価格と`(D/theta)^0.1`へ、既知`[S,T]`の離散平均`W_end`を一度だけ掛ける。posterior theta・配置・3本×22到着rollout・relocation・時間配分は固定する。
機構確認: `simple_terminal_weight_turn=100000`、`simple_terminal_discount_turn`・`simple_terminal_price_discounted_turn`・`simple_terminal_weight_ppm_sum`・`start_only_bid_eval`・`start_only_target_turn`が正で、`boundary_load_factor`が不在、終端係数の乗算箇所が一つ、既存posterior rolloutが発動し、`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv071_articulation_growthの67,590,203を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。v053・v074・v075との差、ケース別勝敗、受入数、reject数、移動費も診断する。
結果: 100/100ケース成功、平均67,153,906でv053比-0.5796%（36勝7分57敗）、現行最高v071比-0.6455%（34勝0分66敗）、v074比+1.1421%（67勝1分32敗）、平均CPU 916ms・最大1,313msだった。`W_end`は100,000回計算・40,230回discount、start-only価格300,000回・目標100,000回で、受入はv053比-189、移動費+297,234、`fallback_count=0`だったため、機構・合法性・時間基準は達成したが平均スコア基準は未達だった。
学び: 未確定。ユーザー判定待ち。

## v077_usable_capacity_shadow — 未決着: 評価済み・判定待ち

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 未判定。
仮説: 現在の総空き面積をそのまま将来容量とみなすと、将来groupが収まりにくい小連結成分まで過大評価する。各空き成分を既知の`P`分布に対するfit確率で割り引いた期待実効容量からadmission価格を決めれば、断片化した盤面で低採算groupを抑え、後続の配置価値を残して現行最高を上回る。
変更: v053_posterior_rollout相当の方針を持つユーザー提供C++ `v088`を単体Rust solverへ忠実に移植し、中心差分である期待実効容量価格と、実行経路にあるregular/growth探索、K=3/H=22 posterior rollout、relocationを保持する。到達不能なsnapshot・box・adaptive分岐は方針へ影響しないdead codeなので移植しない。時間上限だけは`v000_template.rs`に合わせ、提出1.90秒、`local` feature時0.80倍の1.52秒とし、各phaseの比率は原実装から変えない。
機構確認: `dynamic_fragment_waste_turn=100000`、`dynamic_fragment_waste_positive`と`dynamic_fragment_waste_milli_sum`が正で、`normal_rollout_session`と`relocation_attempt`が発動し、`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv071_articulation_growthの67,590,203を上回り、最大solver CPUが1,450ms未満で、上記の機構確認を満たしたら採用する。v053・v071との差、ケース別勝敗、受入数、reject数、移動費も診断する。
結果: 100/100ケース成功、平均67,502,013・最大CPU 1,314ms。v053比-0.0643%（42勝6分52敗）、現行最高v071比-0.1305%（43勝0分57敗）で平均基準は未達、合法性・時間基準は達成した。`dynamic_fragment_waste_turn=100000`、正値99,628回・合計19,816,408,683 milli、`normal_rollout_session=18891`、`relocation_attempt=31774`、`fallback_count=0`で機構確認を満たした。v053比で受入-1,742、reject+1,742、移動費-1,788,445だった。
学び: 未確定。ユーザー判定待ち。

## v078_hybrid_clearance — 未決着: 評価済み・判定待ち

系譜: series=current; base=v053_posterior_rollout; imports=[]
当時の判定: 未判定。
仮説: 静的草地clearanceが低い盤面ではv053相当の汎用連結領域配置より、低clearance専用のregion生成・peel・移動戦略が空間を有効利用できる。clearance 2.10を境に二つの専門家を選べば、smooth側の既存性能を保ちながらrough側で改善し、現行最高を上回る。
変更: v053_posterior_rolloutをsmooth側の基盤とし、ユーザー提供C++ `submission(1).cpp`の最終mainと到達経路に合わせ、静的clearanceが2.10以下なら新規rough専門家、それ以外ならv053相当のsmooth専門家を実行する。入力だけの事前照合ではrough 28ケース・smooth 72ケースである。時間の絶対値は`v000_template.rs`に合わせて提出1.90秒、local 1.52秒へ比例換算する。
機構確認: 全100ケースで`route_rough=28`・`route_smooth=72`となり、rough側のregion配置・peel・moveに対応するtraceとsmooth側の既存`rollout_*`・relocationが発動し、`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv071_articulation_growthの67,590,203を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。v053・v071との差、rough/smooth別のケース勝敗、受入数、reject数、移動費も診断する。
結果: `tools/in`は100/100ケース成功し、現行最高v071比-0.0056%（49勝0分51敗）、base v053比+0.0607%（29勝35分36敗）、平均CPU 1,179ms・最大1,415msだった。rough 28ケースはv071比+4.1085%、smooth 72ケースは同-0.9983%で、routeは事前照合どおり28/72、roughのregion・peel・moveとsmoothのrollout・relocationが発動し、全ケース`fallback_count=0`だった。さらにユーザーが同じ提出をAtCoderの外部50ケースへ提出したところスコアはローカル評価より低下したが、具体値は未記録である。
学び: 低clearance専用expertは既知28ケースでは明確な正効果を示した一方、統合後のsmooth側は同一実装でもwall-clock進行の差から`fast_mode`が増えて既知72ケースで低下した。外部50ケースでもさらに低下したため、乱数・実行時間ノイズと100ケースへの過学習を分離できておらず、汎化性能は未確認である。

## v079_causal_adjudication_hybrid — 未決着: 評価済み・判定待ち

系譜: series=current; base=v078_hybrid_clearance; imports=[v043_no_move_causal_veto]
当時の判定: 未判定。
仮説: v078のrough専門家による低clearance改善を維持しつつ、smoothのposterior上位2案が僅差のときだけcentral thetaで再裁定し、両expertの短期・compact・near-threshold受理を因果帯vetoし、低Rで安価なrelocationを追加すれば、判断分散と容量浪費を減らせる。固定配列・allocation再利用・高速出力で統合時のwall-clock劣化も抑えれば、事前調査値68,463,455.45を再現して現行最高を上回る。
変更: v078_hybrid_clearanceを基準に、ユーザー提供C++の差分どおり、smoothへ2.5%僅差時のcentral追加rollout、margin 1.00..1.13・`D/theta<2`・slack 14以下のv043型veto、`R<=0.02`のrelocation予算0.68→0.72を追加し、roughへmargin上限1.10の同型vetoを追加する。smooth hot pathの固定長配列・一時領域再利用・buffered outputも移植し、route・既存配置・rough探索・時間phase比率は固定する。
機構確認: `route_rough=28`・`route_smooth=72`、`central_adjudication`・`central_flip`、smooth/rough別`causal_veto`、`low_r_extra_relocation`が正で、roughのregion・peel・moveとsmoothのposterior rollout・relocationが発動し、全ケース`fallback_count=0`であることを確認する。v078比のsmooth `fast_mode_turn`・CPUも診断する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv071_articulation_growthの67,590,203を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。事前調査値68,463,455.45、v078・v071との差、route別勝敗、受入数、reject数、移動費も照合する。
結果: `tools/in`は100/100ケース成功し、現行最高v071比+0.2791%（51勝0分49敗）、直接base v078比+0.2847%（50勝0分50敗）、平均CPU 1,183ms・最大1,424msだった。rough 28ケースはv071比+3.8127%・v078比-0.2841%、smooth 72ケースは同-0.5735%・+0.4291%だった。central再裁定15,266回・反転740回、smooth/rough veto 2,331/615回、低R追加relocation 9回が発動し、全ケース`fallback_count=0`だった。事前調査値の99.000053%であり、添付C++の固定1.88秒に対して規定のlocal 1.52秒で評価したため直接同条件ではない。
学び: 既知100ケースでは因果帯veto等の統合により受入をv078比1,861件減らしつつ、平均利用料を213,451増やし、平均移動費21,005増を差し引いて平均純増192,446を得た。rough側はわずかに低下した一方smooth側が改善し、`fast_mode_turn`は減らなかったため、全体改善をhot path高速化だけには帰属できない。外部汎化は未確認である。

## v080_terminal_rollout_hybrid — 未決着: 評価済み・判定待ち

系譜: series=current; base=v079_causal_adjudication_hybrid; imports=[]
当時の判定: 未判定。
仮説: near-threshold受理を固定vetoせず、同じposterior到着列の後に残る多尺度compact配置容量まで含めて拒否候補と比較すれば容量reserveの判断精度が上がる。サイズ別clearance候補とbox侵食候補でsmoothの形状損失を減らし、roughの共有FreeState・探索回数quota・固定容量処理で時間依存の分岐差も抑えれば、v079の現行最高を上回る。
変更: v079_causal_adjudication_hybridを基準に、ユーザー提供C++ `submission_v??_terminal_rollout.cpp`の到達経路を移植する。smoothへ6面積帯の終端容量評価付きreject rollout、static-clearance面積rank候補、`P>=36`の近正方形box侵食候補を追加する。roughへFreeState再利用、region探索回数quotaと共有state探索、固定heap/hash、永続buffer、大型groupのterrain圧力補正を追加し、route cutoff・既存価格/配置/移動・中央再裁定・低R予算は保持する。時間絶対値だけ`v000_template.rs`準拠の提出1.90秒・local 1.52秒へ比例換算する。
機構確認: `route_rough=28`・`route_smooth=72`、smoothの`reject_rollout_offered/selected/accept_selected`・`terminal_rollout_session/value_evaluation`・`size_clearance_offered/selected`・`box_erosion_offered/selected`が正であることを確認する。roughは`region_search/shared_state_search/operation/quota_stop/free_state_build`が正で、hard stop回数も記録する。既存central・causal veto・relocation・rough region/peel/moveが発動し、全ケース`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv079_causal_adjudication_hybridの67,778,857を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。v079・v071との差、route別勝敗、受入数、reject数、移動費、fast modeとrough quota停止率も診断する。
結果: `tools/in`は100/100ケース成功し、直接base v079比+0.5910%（64勝0分36敗）、v071比+0.8718%（62勝0分38敗）、平均CPU 1,183ms・最大1,425msだった。rough 28ケースはv079比+0.1285%、smooth 72ケースは同+0.7075%だった。reject rolloutは2,003提示・500棄却選択・1,503受理選択・終端評価17,164回、size-clearanceは2,800提示・314選択、box侵食は1,900提示・1,370選択、rough共有状態探索51,862回・操作9,776回・quota/hard stop 2,876/6,883回で、全機構が発動し`fallback_count=0`だった。事前登録基準は機械的に全項目達成したが、評価時点の最高v081比は-0.2658%（45勝1分54敗）だった。
学び: 既知100ケースでは統合機構がv079比で両routeを改善し、smoothの`fast_mode_turn`を8,083から6,241へ減らしつつ平均CPUを維持し、受入を1,250件増やしながら移動費を合計1,479,247減らした。ただし複数機構の個別寄与は分離できず、roughではquota stopよりhard stopが多く、探索回数quotaだけではローカルの壁時計依存を解消できていない。v081比の差はsmooth -0.0653%に対してrough -1.0582%へ偏り、外部汎化は未確認である。

## v081_deep_terminal_hybrid — 未決着: 評価済み・判定待ち

系譜: series=current; base=v079_causal_adjudication_hybrid; imports=[]
当時の判定: 未判定。
仮説: v079の因果帯vetoと二専門家を維持し、smoothの標準rollout後も1%以内で未決着な通常配置だけを44到着・終端配置容量で再評価し、サイズ別clearance候補とbox侵食候補を同じ周長帯へ加え、rough探索を共有free stateと操作数quotaで安定化すれば、判断精度と候補品質を時間内で改善して既知100ケースの最高を更新できる。
変更: v079_causal_adjudication_hybridを基準に、ユーザー提供`submission_v??_from_pro73.cpp`どおり、smoothへ最大8 sessionのdeep rollout、6サイズ帯のterminal capacity value、clearance層に基づくサイズ配置候補、近正方形boxの局所侵食候補を追加する。roughは方策を保ちながらfixed heap/hash・共有`FreeState`・region探索回数quota・buffered outputへ置換し、route cutoff、central再裁定、因果veto、低R予算、提出/local時間比率は固定する。
機構確認: `route_rough=28`・`route_smooth=72`、`deep_rollout_session/flip`・`deep_terminal_value_evaluation`、`size_clearance_offered/selected`、`box_erosion_offered/selected`が正であることを確認する。roughの`shared_state_region_search`・`operation_count/quota_total`・`quota_stop`またはhard stop・`free_state_build`、既存central/veto/lowR、region/peel/move、rollout/relocationも発動し、全ケース`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv079_causal_adjudication_hybridの67,778,857を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。v079・v071との差、route別勝敗、受入数、reject数、移動費、CPU、fast modeも診断する。
結果: `tools/in`は100/100ケース成功し、直接baseかつ従来最高のv079比+0.8591%（65勝0分35敗）、v071比+1.1406%（69勝0分31敗）、平均CPU 1,171ms・最大1,417msだった。rough 28ケースはv079比+1.1994%、smooth 72ケースは同+0.7733%だった。deep rolloutは497 session・67反転・終端評価2,982回、size-clearance候補は2,928提示・341選択、box侵食候補は1,964提示・1,425選択、rough共有状態探索80,554回・操作11,807回・quota/hard stop 7,218/4,562回が発動し、routeはrough/smooth=28/72、全ケース`fallback_count=0`だった。事前調査値の99.8505%で、事前登録した採否基準は機械的に全項目達成した。
学び: 既知100ケースでは統合機構が両routeを同時に改善し、v079比でsmoothの`fast_mode_turn`を8,083から6,453へ減らしつつ平均CPUを12ms、最大CPUを7ms短縮した。smoothの移動費は合計2,749,654減り、roughの移動group数は1,293から2,496へ増えたため、長期通常配置比較とrough quotaはそれぞれ意図した仕事量を得た。ただし複数機構の組み合わせ実験なので個別寄与は分離できず、v078の外部50ケース低下を踏まえると外部汎化は未確認である。

## v082_continuous_topology_portfolio — 未決着: 評価済み・判定待ち

系譜: series=current; base=v081_deep_terminal_hybrid; imports=[]
当時の判定: 未判定。
仮説: 入力時の平均clearance `2.10`でsolver全体を二分せず、単一の現在状態に対してcompact packing候補とtopology保存region候補を同じ将来価値で裁定し、rough候補の探索量を静的・動的な局所riskに応じて連続配分すれば、cutoff近傍とopen room/狭路の混在盤面で誤った専門家へ固定される損失が減り、既知100ケース平均は69Mへ近づく。
変更: v081_deep_terminal_hybridを基準に、mainのrough/smooth routeを廃止してsmooth側を唯一の盤面状態・受否・relocation基盤とする。rough側のregion生成器は同じ占有状態を参照する候補提案器へ限定し、毎ターンの静的clearance・自由成分分断・dead-end・占有圧から得る連続topology riskで探索quotaを配分する。両候補源は候補ごとの成分価格を通した後、既存posterior/deep rolloutで共通裁定し、それ以外の価格・時間比率・候補・移動方策は固定する。
機構確認: 全100ケースで`unified_route=1`かつ旧`route_rough/route_smooth`を意思決定に使わず、集計で`topology_risk_turn`とlow/mid/high各帯、`topology_proposal_search/offered`、`cross_source_rollout`、`topology_selected`、`packing_selected`がすべて正となることを確認する。risk帯別の提示・選択、旧clearance 2.10上下および近傍での両候補源の勝敗、既存rollout/relocation、`fallback_count=0`、CPUも診断する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv081_deep_terminal_hybridの68,361,114.29を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。69,000,000以上を目標達成とし、v081との差・勝敗、旧rough/smoothおよびclearance近傍別の差、受入数、reject数、移動費も照合する。
結果: `tools/in`は100/100ケース成功し、直接baseかつ従来最高のv081比-1.3241%（33勝0分67敗）、目標69M比-2.2378%、平均CPU 1,094ms・最大1,340msだった。旧rough/smooth別はv081比-3.5805%/-0.7533%、clearance 1.9超2.3以下の10ケースは-3.4464%、static risk low/mid/highは-0.6432%/-1.6385%/-5.5025%だった。unified routeは全100ケース、risk low/mid/highは9,077/54,304/36,619ターン、topology候補は各帯900/24,024/31,643提示・105/2,698/3,266選択、cross-source 15,159回中topology/packing勝利は5,526/9,633回で、`fallback_count=0`だった。成功・CPU・機構確認は達成したが、平均基準と69M目標は未達だった。
学び: 単一状態上でriskに応じてtopology候補を増減する機構は成立したが、riskは探索頻度・深さにだけ使われ、候補が揃った後の共通rollout価値には入っていなかった。出力再生では両版が共通して受け入れたgroupの利用料はv082の方が高く、損失は受入group集合の入れ替わりが支配した。rough由来のdirect regionだけでは専門家のadmission・peel・separator move/upgradeを代替できず、smooth側では追加topology計算がfast modeを増やしてrelocationを減らしたため、「hard cutoffの不連続が主要損失」という仮説はこの実装では支持されなかった。

## v083_same_economics_topology_challenger — 未決着: 評価済み・判定待ち

系譜: series=current; base=v081_deep_terminal_hybrid; imports=[v082_continuous_topology_portfolio]
当時の判定: 未判定。
仮説: v082で両版共通受入groupの利用料がv081より高かったことがtopology regionの座標品質によるなら、v081の受否・移動方策を保ったまま同じ周長・同じ成分価格のregionだけを受理済みsmooth配置のchallengerにすれば、受入集合を直接変更せず既知100ケース平均を少しでも更新できる。
変更: v081_deep_terminal_hybridを基準にrough route・全admission・既存通常候補/rollout・biased swap・因果veto・relocationを固定する。smoothで既存配置の受理確定後だけ、v082由来advisorを現在盤面へ同期し、同一周長・同一component sizeの最良direct regionを最大1件生成してincumbentとK=3/22・deepなしのpaired rolloutで比較する。riskは提案creditだけへ連続利用し、advisor初期化を含む累積追加枠を`PROGRAM_TIME_LIMIT_SEC`の2%に制限して基準時計から除外し、reject救済・異周長比較・rough側変更は行わない。
機構確認: `route_rough=28`・`route_smooth=72`を維持し、`topology_challenger_search/offered/economic_match/compare/selected`、risk/credit、advisor初期化・追加時間・virtual excluded時間・budget checkと枯渇時のstopが正であること、全比較で周長・component sizeが一致し受理確定後だけ発動することをtraceで確認する。roughのregion/peel/upgrade/moveとsmoothの既存rollout/deep/relocationも発動し、全ケース`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv081_deep_terminal_hybridの68,361,114.29を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。v081との差・勝敗、route別差、受入group集合・共通受入group利用料・移動費、fast mode、追加時間も診断する。
結果: `tools/in`は100/100ケース成功し、直接baseかつ従来最高のv081比+0.1826%（40勝28分32敗）、平均CPU 1,168ms・最大1,418msだった。smooth 72ケースは+0.2647%、rough 28ケースは-0.1418%だった。challengerは943探索・564同一経済条件比較・43選択、追加・仮想除外時間は合計633.945ms（smooth 1ケース8.805ms）、budget stop 0、全ケース`fallback_count=0`で、事前登録基準は機械的に全項目達成した。一方、smoothの選択あり35ケースは+0.1754%、選択なし37ケースは+0.3433%で、共通受入groupの利用料差は負、正差は受入集合の入れ替わりが担った。
学び: 既知100ケースの最高は更新したが、smooth改善の68.97%がchallenger非選択ケースで生じ、選択回数とケース差の相関も-0.064だった。仮想時計除外に伴い`fast_mode_turn`がv081比537回減って非選択ケースでも探索軌道が変わり、無変更のrough routeにも-0.1418%の差が出たため、同一経済条件の座標効果はこの実験だけでは確立できない。また`topology_challenger_selected`はrelocation裁定前の計数で実配置への残存を保証しないため、外部汎化と直接寄与は未確認である。

## v084_value_aware_work_scheduler — 未決着: 評価済み・判定待ち

系譜: series=current; base=v081_deep_terminal_hybrid; imports=[]
当時の判定: 未判定。
仮説: v083のchallenger非選択smoothケースで`fast_mode_turn`減少と得点差が強く連動した原因が、高価値groupへfull searchとrelocationが残ったことなら、v081と同程度のfull/cheap総量を壁時計でなく価値priorityへ決定論的に配り直すだけで、時間依存を減らしつつ既知100ケース最高を更新できる。
変更: v081_deep_terminal_hybridを基準にrough route・受否式・候補内容・rollout・relocation/repack・時間比率を固定する。smoothの二値`fast_mode`だけを、turn 70%以降に線形増加するpressureと、`q^2/(1+q^2)`・期待上限利用料のprefix平均比・`P/MAX_P`の等重みpriorityからfull massを作るfractional-credit schedulerへ置換する。同じ入力ならwork mode列は決定的で、入力のみの事前計算ではcheap 6,552回とv081実測fast 6,453回が同程度である。既存時刻deadlineは提出安全停止として残す。
機構確認: `scheduler_full/cheap_turn`、pressure/priority/full-mass/credit、旧時刻基準に対するpromote/defer、high-q・large別full/cheap、safety cheapが整合し、予定cheap総量と実測の差を説明できることを確認する。rough/smooth=28/72、既存normal/deep rollout・relocation・rough region/peel/moveが発動し、全ケース`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv083_same_economics_topology_challengerの68,485,942.29を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。69,000,000以上を目標達成とし、v081/v083との差・勝敗、route別差、受入集合・価値帯・fast/relocation・CPUを診断する。
結果: `tools/in`は100/100ケース成功し、従来最高v083比-0.0183%（45勝3分52敗）、直接base v081比+0.1643%（44勝3分53敗）、平均CPU 1,148ms・最大1,420msだった。v081比でsmoothは+0.2097%、無変更roughは-0.0155%だった。予定cheapは事前計算どおり6,552回、safety override 755回により実cheapは7,307回となり、high-q/largeのfull率91.67%/92.59%は全体89.85%を上回った。rough/smooth=28/72、既存両route機構、`fallback_count=0`を確認し、成功・時間・機構基準は達成したが最高平均超過と69Mは未達だった。
学び: 価値schedulerはv081比でsmooth受入を275件減らしながら受入集合の利用料を12.382M入れ替え、共通受入groupの利用料-0.904Mと移動費+0.036Mを差し引いて正差を作ったため、計算を高価値・大型turnへ配る方向は有効だった。一方、旧時刻基準よりpromoteがdeferを上回る重い20ケースは全勝・平均+1.795M、defer優勢の軽い52ケースは9勝43敗・平均-0.470Mであり、全caseへturn 70%からpressureを掛けると余裕caseの探索を不必要に削る。再開条件: work不足の観測時だけpressureを立ち上げ、価値priorityはその中の配分に限定する。外部汎化は未確認である。

## v085_departure_event_time_price — 未決着: 評価済み・判定待ち

系譜: series=current; base=v081_deep_terminal_hybrid; imports=[]
当時の判定: 未判定。
仮説: [固定軌跡probe](deep/b087_time_price_probe.md)で、既知退去量と同時刻までの未到着期待負荷を足す曲線が未使用35 smoothケースの負荷RMSE・容量超過圧MAEをともに大幅低減したため、その区間価格をadmissionへ積分すれば、v036の過剰救済を避けつつ近く空く容量と長く塞がる容量を区別し、既知100ケース最高を更新できる。
変更: v081_deep_terminal_hybridを基準に、rough route・全配置候補/rollout・因果veto・relocation・時間配分を固定する。smoothだけで、到着groupの`[S,T]`を既知activeの退去時刻で分割し、各区間中点の既知予約量と`theta`事後による固定到着率・終端補正付き未到着期待負荷から容量価格を求め、区間長で積分した値へ`(D/theta)^0.1`を掛ける。旧閾値は探索量固定用prefilterだけに残し、空間評価へtime priceを入れない。
機構確認: `time_price_turn/interval/departure_split`、既知予約量・未到着期待量、旧/新閾値、price rescue/dropが正で、prefilterが旧閾値と一致することを確認する。rough/smooth=28/72、既存両routeのrollout・relocation・region/peel/moveが発動し、全ケース`fallback_count=0`、退去イベント価格処理時間も確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv083_same_economics_topology_challengerの68,485,942.29を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。69,000,000以上を目標達成とし、v081/v083との差・勝敗、route別差、受入集合・価格救済/脱落・移動費・CPUを診断する。
結果: `tools/in`は100/100ケース成功し、直接base v081比-5.4240%（13勝4分83敗）、従来最高v083比-5.5964%（16勝3分81敗）、69M比-6.2997%、平均CPU 1,285ms・最大1,425msだった。smoothはv081比-6.7188%（1勝1分70敗）、roughは同-0.3062%。`time_price_turn=72000`、区間1,216,751・退去分割1,144,751、新/旧閾値総量比48.5%、lower/higher=60,257/1,950、救済/脱落=6,326/42で、prefilter旧式照合・既存両route機構・`fallback_count=0`も確認した。成功・時間・機構基準は達成したが平均と69M基準は未達だった。
学び: 固定軌跡で将来の物理負荷を正確に予測できても、その総量を既存の`capacity/load`採択率へ直接入れれば正しい限界価格にはならない。既受理の確定占有は既に選別済みで取り消せないのに、未到着の未選別需要と合算して再び同じ採択率を掛けたため価格が系統的に低下し、受入を54.64件/ケース増やしながら受入集合の理想利用料を3.166M/ケース失い、形状損失も0.561M/ケース増やした。smoothでv085だけが受けた9,112件は平均`q=0.828`・理想利用料0.099M、v081だけが受けた3,663件は`q=1.585`・0.333Mであり、低価値案件が後続の大型高価値案件を押し出した。価格曲線処理47.7ms/ケース相当と軌跡変化でfast modeも6,453から14,510回へ増えたため、時間置換も副次損失として残る。

## v086_cpp_faithful_runtime — 未決着: 評価済み・判定待ち

系譜: series=current; base=v081_deep_terminal_hybrid; imports=[]
当時の判定: 未判定。
仮説: AtCoder 50ケースでC++比-2.0302%となった主因がRust移植時のrough時計起点・同点順序・hot path確保とsmoothの不要なcloneによる探索仕事量差なら、v081の数理方策を固定したままC++相当の実行構造へ戻せば、十分な探索量と近い時間挙動を回復して既知100ケース最高を更新できる。
変更: v081_deep_terminal_hybridを基準に、受否式・候補評価・探索quota・時間比率を固定する。roughの方策時計だけをC++同様に静的盤面前計算後から開始し、main開始時のhard clockは提出安全上限として残す。同点比較をC++ comparatorの比較キーへ限定し、same-region stamp・growth heap・pair接触表・free-state queueを再利用し、smoothは盤面解除時のcell cloneなど意味を変えない反復確保だけを除く。詳細は[静的移植監査](deep/b101_cpp_faithful_audit.md)。
機構確認: `rough_policy_clock_offset_us`・`rough_same_region_check`・`rough_grow_heap_reset`・`rough_pair_buffer_reset`が正で、rough/smooth=28/72、既存deep/terminal・clearance・box・central/veto/lowR・region/peel/moveが発動し、全ケース`fallback_count=0`であることを確認する。方策時計とmain起点hard clockの到達数、roughのregion search・operation・quota/hard stop、smoothのfast mode、CPUも照合する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv083_same_economics_topology_challengerの68,485,942.29を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。69,000,000以上を目標達成とし、v081/v083との差・勝敗、route別差、探索仕事量、受入数、移動費、CPUを診断する。
結果: `tools/in`は100/100ケース成功し、直接base v081比+0.1617%（33勝41分26敗）、従来最高v083比-0.0208%（36勝23分41敗）、69M比-0.7657%、平均CPU 1,174ms・最大1,424msだった。rough/smoothはv081比+0.1008%/+0.1772%、v083比+0.2430%/-0.0873%。方策時計offsetはrough平均1.364ms、same-region 198,434回・growth heap再利用2,367,885回・pair buffer再利用52回、全体hard stop 0で、region searchはv081比+6.91%に対しrough CPUは+0.75%だった。route=28/72、既存両route機構、`fallback_count=0`も確認した。後日の同一AtCoder 50ケースはv086 3,606,239,399対C++ 3,676,629,314で-70,389,915（-1.9145%）、旧Rust比+4,251,606（+0.1180%）だった。localの成功・時間・機構基準は達成したが、最高平均・69M・外部C++同等性は未達だった。
学び: roughの選定hot pathで仕事率を回復しても、外部50ケースで元のC++差の5.696%しか回復しなかったため、v086はC++忠実移植ではなく部分的な実行構造修正だった。Rust 1.90秒はC++ 1.88秒より約1.06%有利であり原因候補から外れる。本質的な未解決差は、roughの部分comparator後の打ち切りが候補集合とRNG列を変えること、1.90秒の優位を上回る既知CPU+14.4%により壁時計内の探索仕事量が減ること、浮動小数点関数/再計算の境界分岐である。次は固定仕事量のturn単位oracle比較で意味差を先に消し、その後にC++ 1.88秒/Rust 1.90秒の完了仕事量を同等化する必要がある。

## v087_faithful_latest_portfolio — 未決着: 評価済み・判定待ち

系譜: series=current; base=v086_cpp_faithful_runtime; imports=[v083_same_economics_topology_challenger, v084_value_aware_work_scheduler]
当時の判定: 未判定。
仮説: v086のC++相当実行構造によるrough側の仕事量回復と、v083の同一経済条件topology challengerおよびv084の高価値groupへの計算配分はsmooth側で補完し、既知100ケース平均の最高を更新する。
変更: v086_cpp_faithful_runtimeを基準にroughの方策・時計・hot pathと全admissionを固定する。smoothへv083の受理確定後・同一周長・同一component sizeのchallengerと2%累積追加枠、v084のturn 70%以降の価値priority付きfractional-credit schedulerをそれぞれ既評価仕様のまま統合する。v085のtime priceは取り込まない。
機構確認: `route_rough=28`・`route_smooth=72`を維持し、v086の`rough_policy_clock_offset_us`・stamp/heap/pair buffer、v083の`topology_challenger_search/offered/economic_match/compare/selected`・追加時間/仮想除外時間/budget、v084の`scheduler_full/cheap_turn`・pressure/priority/full-mass/credit・promote/defer・high-q/largeがすべて正となること、既存両route機構と全ケース`fallback_count=0`を確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv083_same_economics_topology_challengerの68,485,942.29を上回り、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。69,000,000以上を目標達成とし、v081/v083/v084/v086との差・勝敗、route別差、受入集合・work mode・challenger・移動費・CPUを診断する。
結果: `tools/in`は100/100ケース成功し、v081比+0.0230%（43勝2分55敗）だったが、従来最高v083比-0.1593%（38勝1分61敗）、v084比-0.1410%、直接base v086比-0.1385%、69M比-0.9031%だった。rough/smoothはv086比+0.1089%/-0.2011%、平均CPU 1,174ms・最大1,421msだった。route=28/72、challengerは913探索・537比較・40選択・追加時670.661ms・budget stop 0、schedulerは予定full/cheap=65,448/6,552・実測64,245/7,755・safety override 1,203・promote/defer=3,372/4,527、v086のstamp/heap/pair bufferと既存両route機構も発動し、全ケース`fallback_count=0`だった。成功・時間・機構基準は達成したが、最高平均と69M基準は未達だった。
学び: 受入集合の理想利用料はv084比+0.024M/ケースだったが、最終利用料は-0.099M、移動費は-0.003Mで、得点負差は受否でなく形状実現率の低下が支配した。v084と予定cheapは完全一致した一方、safety overrideは448回増、smooth relocationは86件減となり、challengerとschedulerは独立部品ではなく、配置・時間軌道を介して非加法に干渉する。またpromote優勢の22 smoothケースはv081比+1.249M/ケース（20勝2敗）、defer優勢の50ケースは-0.576M（9勝41敗）で、v084の「work不足のcaseだけpressureを発動する」という再開条件はさらに強まった。

## v088_v083_hotpath_runtime — 未決着: 評価済み・判定待ち

系譜: series=current; base=v083_same_economics_topology_challenger; imports=[]
当時の判定: 未判定。
仮説: v083の出力意味を保ったまま、既存traceで支配的な断片化全盤面評価・relocation走査・growth・rolloutとrough反復領域の命令数・allocation・初期化量を減らせば、同じ壁時計内の探索仕事量が増えて既知100ケース最高を更新できる。
変更: v083を基準に受否式・候補集合と順序・比較器・乱数列・浮動小数点式・時間比率を固定する。断片化BFSを同値な行bitset flood fillへ、FreeInfoの成分セルをflat poolへ、relocation blockerを固定長bufferへ、growthのseed/heap/markを再利用へ、quick rolloutを必要runだけの直接scanへ変え、roughのsame-region markと意味を変えない反復bufferを再利用する。近似・f32・SIMD・方策時計変更は行わない。
機構確認: 補助kernel照合で旧実装と新実装の断片化値・run判定が乱択盤面すべて一致し、新traceのbitset fragment・direct quick scan・fixed blocker・growth reuse・rough stampが正、既存両route機構が発動し全ケース`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv083_same_economics_topology_challengerの68,485,942.29を上回り、平均solver CPUがv083の1,168ms未満、最大solver CPUが1,500ms未満で、上記の同値性・機構確認を満たしたら採用する。69,000,000以上を目標達成とし、route別差・探索仕事量・受入数・移動費も照合する。
結果: `tools/in`は100/100ケース成功し、従来最高かつ直接baseのv083比+1.0527%（44勝42分14敗）、69M目標を+206,890で達成した。平均CPUは1,168msから933msへ20.1%短縮、最大1,400ms。smooth/roughは+1.2525%/+0.2596%、CPU-28.8%/-2.1%。smoothは断片化評価+9.27%・growth+6.05%・relocation option+8.03%に対し、normal search-47.6%・relocation-21.6%・rollout-68.0%。全runtime traceが対象route全ケースで発動し全ケース`fallback_count=0`、補助照合は断片化2,048盤面・run mask 500万件・top-k/heap/連結性各2万件・region set 2,000 batchで完全一致、断片化kernelは6.58倍だった。全採否基準を機械的に達成した。
学び: v083の方策を動かさず、全盤面評価・run table・反復heap/mark/初期化を同値なbitset・直接scan・再利用bufferへ変えるだけで、smoothは処理仕事量を6〜9%増やしつつ主要時間を22〜68%削減でき、得点も一貫して改善した。roughは機構発動とCPU短縮は成立したが仕事量が約3%減り勝敗も混在したため、今回の更新は主にsmooth hot pathの高速化が支配した。

## v089_repack_parent_arena — 未決着: 評価済み・判定待ち

系譜: series=current; base=v088_v083_hotpath_runtime; imports=[]
当時の判定: 未判定。
仮説: v088 smooth時間の68.5%を占めるrelocationで、最大4段beam childごとの過去Placement/セルVec深いcloneとscratch再確保を除けば、候補順と評価を変えずにrepack仕事率が上がり、残る`fast_mode`とrelocation budget hitを減らして既知最高を更新できる。
変更: v088を基準に受否・候補集合/順序・beam幅/枝数・比較器・乱数・浮動小数点式・時間比率を固定する。BeamStateの可変pathを固定長arena indexへ変え、規則形状セルは最終勝者だけ具体化し、beam/候補/FreeInfo/WeightData/RunTableをrepack間で再利用する。盤面Rowsのchild別値コピーは探索状態そのものなので維持する。
機構確認: 補助kernelで旧deep-clone beamと新arena beamの勝者score・path・盤面が乱択木ですべて一致し、`runtime_repack_workspace_call/child/path_ref/deferred_materialize`が正、既存B-104機構と両route機構が発動し全ケース`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv088_v083_hotpath_runtimeの69,206,890.30を上回り、平均solver CPUがv088の933ms未満、最大solver CPUが1,500ms未満で、同値性・機構確認を満たしたら採用する。smooth relocation時間・option/repack仕事率・fast turn・budget hitとroute別差を照合する。
結果: `tools/in`は100/100ケース成功し、直接baseかつ従来最高のv088比-0.0057%（20勝66分14敗）、69M目標は維持、平均CPUは933msから922msへ11.8ms短縮、最大1,419msだった。中心対象smoothは+0.0335%（7勝63分2敗）・CPU-23.2msで、relocation時間-3.73%に対しoption+0.42%・repack+0.51%となり仕事率はそれぞれ+4.31%/+4.40%、fast turn-51.5%、budget hit-15.3%。未変更roughはregion search+6.92%・CPU+17.8ms・得点-0.1629%で全体正差を相殺した。workspace/child/deferredは全72 smooth、path共有は71ケースで発動し、補助beam 2万木・direct mask 50万件・再利用run table 2万件は完全一致、全ケース`fallback_count=0`だった。成功・CPU・機構基準は達成、最高平均基準は未達だった。
学び: repackの過去Placement共有とscratch再利用は、方策を変えずsmooth relocation仕事率を約4.3%上げ、残存fast turnとbudget hitを減らしてsmooth得点を小幅に改善した。ただしv088でfast cliffの大半を既に解消した後は、意味保存高速化の追加利益が全体+0.03%規模まで縮み、別routeの時間依存軌道変動で相殺され得る。次段階ではさらなる一様高速化より、余剰時間を軽いsmoothケースの追加探索へ明示的に再投資する効果を独立検証する必要がある。

## v090_rough_compact_bitset — 未決着: 評価済み・判定待ち

系譜: series=current; base=v088_v083_hotpath_runtime; imports=[]
当時の判定: 未判定。
仮説: roughのcompact-template探索で各spec・各x・各yをprefix sum判定していることがhard stopを生む主要な残存費用の一つなら、全yの合法性を行run maskの積で一括列挙すれば候補順を保ったままroughの仕事率が上がり、既知100ケース平均の最高を更新できる。
変更: v088_v083_hotpath_runtimeを基準に、roughのFreeStateへ長さ1..25の行run maskを構築し、compact-templateの二重x/y走査だけを同値な`spec_valid_y_mask`と下位bit順列挙へ置換する。候補score・保持数・比較器・乱数列・探索quota/時計・smooth側は固定する。
機構確認: 補助kernelで旧prefix判定と新bitset判定の合法y maskおよび列挙順が乱択盤面・全compact specで一致し、`runtime_rough_compact_bitset_scan`が全roughケースで正、rough/smooth=28/72、既存両route機構が発動し全ケース`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv088_v083_hotpath_runtimeの69,206,890.30を上回り、平均solver CPUがv088の933ms未満、最大solver CPUが1,500ms未満で、上記の同値性・機構確認を満たしたら採用する。roughの得点・CPU・region search・hard/quota stopとsmoothの不変性を照合する。
結果: `tools/in`は100/100ケース成功し、直接baseかつ従来最高のv088比+0.0571%（20勝67分13敗）、平均CPUは933msから877msへ6.0%短縮、最大1,384msだった。roughは+0.2131%・CPU-12.2%、region search+36.84%、hard stop-95.47%、quota stop+80.67%。smoothは64/72ケースで得点不変、rough topology challengerが動く8ケースだけ変化して合計+0.0182%だった。bitset scanは全28 roughで合計136,572回発動し、補助照合は13,177,857 mask・39,723,001合法位置で完全一致、走査kernelは15.44倍、route=rough/smoothは28/72、全ケース`fallback_count=0`であり、全採否基準を機械的に達成した。
学び: rough compact-templateの全y prefix判定はhard stopを生む支配的な残存費用だった。同値bitset化は探索時計を変えずにregion searchを約37%増やし、hard stopをほぼquota stopへ置換してrough得点を改善した。一方smooth本体を固定しても、共有するrough topology challengerの所要時間と候補軌道を介して少数ケースは変化するため、今後の意味保存高速化でも子探索を含む時間依存を追跡する必要がある。

## v091_slack_holdout_rollout — 未決着: 評価済み・判定待ち

系譜: series=current; base=v090_rough_compact_bitset; imports=[]
当時の判定: 未判定。
仮説: smoothの基準deep quotaを使い切った後も実時間が進捗paceより前にあり、同一周長・同一component sizeの上位2候補が1%以内で未決着なら、freshなposterior scenarioを追加して標本誤差を減らすことで、基準探索を置換せず余剰時間を座標選択精度へ変換し既知100ケース最高を更新できる。
変更: v090_rough_compact_bitsetを基準に、候補集合・標準3 scenario・central adjudication・既存最大8 deep sessionを先に固定どおり実行する。paced deep枠を消費済みで実測slackが10ms相当以上あるsmooth比較だけ、進捗比例で最大32 session、各最大9本のfresh 44到着+terminal scenarioで同じ上位2候補をpaired holdout再判定する。追加時間は基準時計から除外し実時間1.84/1.90でhard stopし、admission・即時利用料・relocation・topology challenger・roughを固定する。
機構確認: 補助kernelでpaced session上限・slack/hard gate・fresh seedが既存seed 0..3と非重複・paired candidateのscenario一致・不完全batchが基準winnerを保持することを照合する。実評価では`spare_rollout_session/scenario/flip/extra_time_us`が正、48以上のsmoothケースで発動、smooth平均追加時間40〜200ms、rough/smooth=28/72、既存deep/両route機構が発動し全ケース`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv090_rough_compact_bitsetの69,246,417.48を上回り、平均solver CPUが920〜1,100ms、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。smooth/rough別得点、勝敗、CPU、通常/deep rollout、fast mode、relocation、追加sessionの完全率と反転率を照合する。
結果: 補助kernelはscheduler 500,500状態・標準4,000 seed・追加9,000 seed・不完全batch 9通りを完全照合した。`tools/in`は100/100成功し、直接baseかつ従来最高のv090比+0.0318%（46勝13分41敗）で最高を更新、平均CPUは877→881ms、最大1,381msだった。smoothは+0.0577%（38勝34敗）、roughは-0.0716%（8勝13分7敗）。追加holdoutは全72 smoothで2,127 session・19,143 scenarioが100%完了し517回（24.31%）反転、平均19.81ms、全smoothでsession cap、slack reject 6ケース、incomplete/budget stop/real hardは0、全ケース`fallback_count=0`だった。成功・得点・最大CPU・発動数は満たしたが、平均CPU 920ms以上とsmooth追加時間40ms以上は未達で、事前登録基準全体は未達だった。
学び: freshな同一経済条件holdoutはsmoothの候補選択を十分な頻度で反転し、v090実時間1,000ms未満の48 smoothでも25勝23敗の正差となったため、余剰時間を選択精度へ変える方向には正効果がある。一方、追加rollout平均+18.1msに対して後段relocationが平均-15.1msとなり全体CPUはほぼ増えず、全smoothで32 session上限へ達したので、現行機構は時計ではなく追加仕事量と後段軌道変化に律速される。

## v092_reserve_gated_holdout — 未決着: 評価済み・判定待ち

系譜: series=current; base=v091_slack_holdout_rollout; imports=[]
当時の判定: 未判定。
仮説: v091の正差が実時間1,000ms未満のsmoothへ集中し全smoothが32 session上限へ達したため、v091枠を保った上で大きな実測slackを持つcaseだけへ第二層holdoutを追加すれば、重いcaseの負差を広げず未使用時間を判断精度へ変換し、既知100ケース最高を更新できる。
変更: v091_slack_holdout_rolloutを基準に、最大32 session・最低slack 1/190の既存第一層を固定する。各進捗で第一層paced枠へ追いついた後だけ、最低slack 30/190を残す最大96 sessionの第二層をpaced配分し、標準seed 0..3・第一層seed 17..25と非重複のfresh seed 29..37で、同一周長・同一component sizeかつ標準判定差1%以内の同じ上位2候補を9本×44到着+terminalでpaired再判定する。追加時間は基準時計から除外し、実時間1.84/1.90 hard stop、admission・候補・即時利用料・relocation・topology challenger・roughを固定する。
機構確認: 補助kernelで二層paced上限・第一層優先・slack gate・3 seed集合の非重複・paired candidateのscenario一致・不完全batchのbase winner保持を照合する。実評価では`reserve_rollout_session/scenario/flip/extra_time_us`が正で36以上のsmoothケースに発動、v091実時間1,000ms未満群の平均第二層sessionが同1,000ms以上群の2倍以上、smooth平均第二層時間20〜100ms、全batch完了、rough/smooth=28/72、既存機構が発動し全ケース`fallback_count=0`であることを確認する。
採否基準: `tools/in` 100ケースが全成功し、平均絶対スコアが記録済み全実験中最高のv091_slack_holdout_rolloutの69,268,431.72を上回り、平均solver CPUが900〜1,050ms、最大solver CPUが1,500ms未満で、上記の機構確認を満たしたら採用する。v091比の全体・smooth/rough・旧軽量/重量群別得点、勝敗、CPU、通常/deep/第一層/第二層rollout、fast mode、relocationを照合する。
結果: 補助kernelはscheduler 500,500状態・標準4,000/第一層9,000/第二層9,000 seed・paired 9,000件・不完全batch 9通りを完全照合した。`tools/in`は100/100成功したがv091比-0.1799%（36勝22分42敗）、平均CPUは881→905ms、最大1,372msだった。smoothは-0.1993%（32勝7分33敗）、roughは-0.1024%（4勝15分9敗）。第二層は63/72 smoothで4,283 session・38,523 scenario、4,278完了・5不完全、950反転（完了比22.21%）、平均39.40msだった。旧1,000ms未満/以上群の平均sessionは74.48/29.50で2.52倍、全ケース`fallback_count=0`。成功・CPU・発動数・時間・配分集中は満たしたが、最高平均超過と全batch完了は未達だった。
学び: 大slack群へ追加仕事を集中するschedulerは成立したが、軽量smoothもv091比-0.1085%、重量smoothは-0.3557%となり、配分先の誤認ではなく32枠を越える比較介入の限界効用が負だった。第二層は既存判断の標本数を増やさず、固定9本判定を新たな比較へ広げたため、介入回数と後段軌道差を増やし、smoothの受理を47件・relocation配置を94件減らした。v091の32枠は単なる仕事量上限でなく、低確信な追加反転を抑える正則化として扱う必要がある。
