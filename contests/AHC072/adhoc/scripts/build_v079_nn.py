#!/usr/bin/env python3
"""v076と凍結済みv078の重みから、提出可能な単一C++を作る。再学習はしない。"""
import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
import torch

ROOT = Path(__file__).resolve().parents[2]
PARENT_SHA = "88b6eadd6d328bf980b53afc96b17320cff1df15f3717cb457ab7df91e3316f4"
MODEL_SHA = "93a9a319564a93161c874fdd08838c0f98441824b2c0793d2c64270a4ebb61fa"


def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    source = ROOT / "src/bin/v076_relative_tuned.cpp"
    training = ROOT / "results/nn_rank/v078/20261002T095446_studio"
    model_path = training / "models/immediate/latest.pt"
    assert sha(source) == PARENT_SHA and sha(model_path) == MODEL_SHA
    state = torch.load(model_path, map_location="cpu", weights_only=False)
    assert state["epoch"] == 120 and state["offset"] == 0
    norm = json.loads((training / "dataset.json").read_text())
    weights = {k: v.numpy().tolist() for k, v in state["model"].items()}
    model = {"mean": norm["mean"], "scale": norm["scale"], "features": norm["features"], "weights": weights}
    (args.output / "model.json").write_text(json.dumps(model, indent=2) + "\n")
    def literal(a):
        if isinstance(a, list):
            return "{" + ",".join(map(literal, a)) + "}"
        assert np.isfinite(a)
        return float(np.float32(a)).hex() + "f"
    constants = ["// v078 immediate、120 epochの固定重み。低い出力の候補を選ぶ。", "namespace neural_rank {"]
    for name, dimensions, values in (("mean", "[32]", model["mean"]), ("scale", "[32]", model["scale"]),
        ("w1", "[16][32]", weights["0.weight"]), ("b1", "[16]", weights["0.bias"]),
        ("w2", "[16]", weights["2.weight"][0]), ("b2", "[1]", weights["2.bias"])):
        constants.append(f"constexpr float {name}{dimensions}={literal(values)};")
    constants += ["""float predict(const array<double,32>& raw) {
    array<float,32> x;
    // 学習時と同じく、rawをfloat32へ丸めてから正規化する。
    for(int j=0;j<32;j++)x[j]=(float(raw[j])-mean[j])/scale[j];
    float answer=b2[0];
    for(int i=0;i<16;i++){
        float value=b1[i];
        for(int j=0;j<32;j++)value+=w1[i][j]*x[j];
        answer+=w2[i]*max(0.0f,value);
    }
    return answer;
}
}
#ifdef LOCAL
struct NeuralRankStats {
    int64_t calls=0,scored=0,changed=0,rank_sum=0;
    double feature_ms=0,inference_ms=0,total_ms=0;
    void summary() const {
        trace.count_by("nn_calls",calls);trace.count_by("nn_candidates_scored",scored);
        trace.count_by("nn_changed_choices",changed);trace.count_by("nn_selected_original_rank_sum",rank_sum);
        trace.add_time_ms("nn_features",feature_ms);trace.add_time_ms("nn_inference",inference_ms);
        trace.add_time_ms("nn_selection",total_ms);
    }
} local_nn;
#endif
"""]
    teacher = (ROOT / "adhoc/scripts/v077_teacher_support.cpp.txt").read_text()
    begin = teacher.index("    array<double,32> nn_features(")
    features = teacher[begin:teacher.index("    void nn_snapshot(", begin)]
    members = (ROOT / "adhoc/scripts/v079_nn_members.cpp.txt").read_text()
    s = source.read_text()
    replacements = []
    def replace(old, new):
        nonlocal s
        assert s.count(old) == 1, (old[:100], s.count(old))
        s = s.replace(old, new)
        replacements.append((old, new))
    replace("// v076_relative_tuned.cpp", "// v079_nn_immediate.cpp")
    replace("class TemporalLNS {", "\n".join(constants) + "\nclass TemporalLNS {\n#ifdef AHC072_NN_PROBE\n    friend struct NeuralRankProbe;\n#endif")
    begin = s.index("#ifdef LOCAL\n    int removed_commands(")
    end = s.index("#endif", begin) + len("#endif")
    old = s[begin:end]
    replace(old, "// NN特徴にも使うため、提出ビルドでも有効にする。\n" + old[len("#ifdef LOCAL\n"):-len("#endif")])
    replace("    void optimize(vector<Move>& best) {", features + members + "\n    void optimize(vector<Move>& best) {")
    replace("                if(choice<0)choice=rng(int(candidates.size()));\n                Removal cand=candidates[choice];last_tried[cand.hash]=iteration;", """                // 元の順位選択で適格候補がある場合だけ、同じ待ち条件でNNを使う。
                const bool nn_eligible=!dependency&&mode>=3&&choice>=0;
                if(choice<0)choice=rng(int(candidates.size()));
                if(nn_eligible){
                    const double progress=clamp((time_keeper.exact_elapsed_sec()-lns_start)/max(LOCAL_SECONDS(0.02),end-lns_start),0.0,1.0);
                    choice=nn_choose(current,best,stagnant,progress,cooldown);
                }
                Removal cand=candidates[choice];last_tried[cand.hash]=iteration;""")
    replace("        trace.summary();", "        local_nn.summary();trace.summary();")
    # 学習対象以外の差分が紛れ込まないよう、変更を逆適用して親の全バイトに戻す。
    restored = s
    for old, new in reversed(replacements):
        assert restored.count(new) == 1
        restored = restored.replace(new, old)
    assert restored == source.read_text()
    result = ROOT / "src/bin/v079_nn_immediate.cpp"
    result.write_text(s)
    audit = {"parent_sha256": PARENT_SHA, "checkpoint_sha256": MODEL_SHA, "solver_sha256": sha(result),
             "model_json_sha256": sha(args.output / "model.json"), "normalization_sha256": sha(training / "dataset.json"),
             "registered_replacements": len(replacements), "inverse_patch_exact_parent": True,
             "local_and_nonlocal_nn": True}
    (args.output / "source_audit.json").write_text(json.dumps(audit, indent=2) + "\n")
    print(json.dumps(audit))


if __name__ == "__main__":
    main()
