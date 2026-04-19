#!/usr/bin/env python3
import json
import pathlib
import subprocess
import sys


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: resolve_bin_src.py <root_dir> <bin_name>", file=sys.stderr)
        return 2

    root_dir = pathlib.Path(sys.argv[1]).resolve()
    bin_name = sys.argv[2]
    manifest_path = root_dir / "Cargo.toml"

    cp = subprocess.run(
        [
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
            str(manifest_path),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    metadata = json.loads(cp.stdout)

    package = None
    manifest_abs = manifest_path.resolve()
    for candidate in metadata.get("packages", []):
        if pathlib.Path(candidate["manifest_path"]).resolve() == manifest_abs:
            package = candidate
            break
    if package is None:
        print(f"error: package not found for manifest: {manifest_path}", file=sys.stderr)
        return 1

    for target in package.get("targets", []):
        if target.get("name") == bin_name and "bin" in target.get("kind", []):
            print(pathlib.Path(target["src_path"]).resolve())
            return 0

    print(f"error: bin target not found: {bin_name}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
