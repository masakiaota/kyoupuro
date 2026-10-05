#!/usr/bin/env python3
"""Inspect the measured binary without executing or rebuilding it."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v035_tower_capacity"
BINARY = OUT / "bench_production"
TARGETS = {"original": "parent::TemporalLNS::insertAt(",
           "full": "bool child::TemporalLNS::insertAt<400ul>(",
           "compact192": "bool child::TemporalLNS::insertAt<192ul>("}


def main():
    symbols = []
    raw = subprocess.check_output(["xcrun", "llvm-nm", "--numeric-sort", "--demangle", str(BINARY)], text=True)
    for line in raw.splitlines():
        match = re.match(r"([0-9a-fA-F]+)\s+[Tt]\s+(.*)", line)
        if match:
            symbols.append((int(match[1], 16), match[2]))
    addresses = sorted({address for address, _ in symbols})
    result = {"binary_sha256": hashlib.sha256(BINARY.read_bytes()).hexdigest(), "functions": {}}
    for key, prefix in TARGETS.items():
        matches = [(a, n) for a, n in symbols if n.startswith(prefix) and "'lambda'" not in n]
        if len(matches) != 1:
            raise RuntimeError(f"Expected one function: {key}")
        start, name = matches[0]
        stop = next(address for address in addresses if address > start)
        text = subprocess.check_output(["xcrun", "llvm-objdump", "--disassemble", "--no-show-raw-insn",
                                        f"--start-address={start}", f"--stop-address={stop}", str(BINARY)], text=True)
        (OUT / ("assembly_" + key + ".txt")).write_text(text)
        instructions = []
        for line in text.splitlines():
            match = re.match(r"\s*([0-9a-fA-F]+):\s+([a-z][a-z0-9.]*)(?:\s|$)", line)
            if match and start <= int(match[1], 16) < stop:
                instructions.append(match[2])
        if not instructions:
            raise RuntimeError("No disassembled instructions")
        result["functions"][key] = {"name": name, "text_bytes_to_next_symbol": stop - start,
                                    "instructions": len(instructions),
                                    "call_sites": sum(op in ("bl", "blr") for op in instructions)}
    (OUT / "assembly_summary.json").write_text(json.dumps(result, indent=2) + "\n")
    for name, stats in result["functions"].items():
        print(name, {k: v for k, v in stats.items() if k != "name"})


if __name__ == "__main__":
    main()
