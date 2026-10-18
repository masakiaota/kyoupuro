#!/usr/bin/env python3
"""Git 管理外の config/local_profiles.json から端末設定を読む。"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import math
from pathlib import Path
import socket
import sys

CONFIG_PATH = Path(__file__).resolve().parent.parent / "config" / "local_profiles.json"


@dataclass(frozen=True)
class LocalProfile:
    name: str
    hostname: str
    local_time_ratio: float = 1.0
    eval_jobs: int = 1
    reason: str = ""


def parse_ratio(value: object) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float, str)):
        raise ValueError("local_time_ratio must be a positive finite number")
    ratio = float(value)
    if not math.isfinite(ratio) or ratio <= 0 or not math.isfinite(1.0 / ratio):
        raise ValueError("local_time_ratio must be a positive finite number")
    return ratio


def load_profile(
    profile_name: str | None = None,
    *,
    path: Path = CONFIG_PATH,
    hostname: str | None = None,
) -> LocalProfile:
    hostname = socket.gethostname() if hostname is None else hostname
    try:
        profiles = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(profiles, dict):
            raise ValueError("profiles must be a JSON object")
        if profile_name is not None:
            if profile_name not in profiles:
                raise ValueError(f"profile not found: {profile_name}")
            selected = profiles[profile_name]
        else:
            matches = [(name, value) for name, value in profiles.items()
                       if isinstance(value, dict) and value.get("hostname") == hostname]
            if len(matches) != 1:
                raise ValueError(f"expected one profile for hostname={hostname}, found {len(matches)}")
            profile_name, selected = matches[0]
        if not isinstance(selected, dict):
            raise ValueError("profile must be a JSON object")
        ratio = parse_ratio(selected["local_time_ratio"])
        jobs = selected["eval_jobs"]
        if type(jobs) is not int or jobs < 1:
            raise ValueError("eval_jobs must be an integer >= 1")
        return LocalProfile(profile_name, hostname, ratio, jobs)
    except (OSError, UnicodeError, ValueError, KeyError, OverflowError) as error:
        # 設定を読めない場合は倍率 1 / 1 並列にする、という運用上の既定値。
        # 呼び出し元で reason を表示し、補正なしで動いたことを分かるようにする。
        return LocalProfile("default", hostname, reason=str(error))


def describe_profile(profile: LocalProfile) -> str:
    message = (f"config: profile={profile.name} host={profile.hostname} "
               f"ratio={profile.local_time_ratio:g} jobs={profile.eval_jobs}")
    if profile.reason:
        message += f" (using defaults: {profile.reason})"
    return message


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", help="Select a profile instead of matching the hostname")
    parser.add_argument("--ratio", type=parse_ratio, help="Validate an explicitly supplied build ratio")
    args = parser.parse_args()
    if args.ratio is not None:
        ratio = args.ratio
    else:
        profile = load_profile(args.profile)
        print(describe_profile(profile), file=sys.stderr)
        ratio = profile.local_time_ratio
    # stdout は build_solver.sh がコンパイル引数として受け取る数値だけにする。
    print(repr(ratio))


if __name__ == "__main__":
    main()
