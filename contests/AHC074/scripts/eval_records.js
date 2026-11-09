import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

export function recordPaths(directory, suffix = ".jsonl", excludedEntries = []) {
  if (!fs.existsSync(directory)) return [];
  const files = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (excludedEntries.includes(entry.name)) continue;
    const name = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...recordPaths(name, suffix));
    else if (entry.isFile() && name.endsWith(suffix)) files.push(name);
  }
  return files.sort();
}

export function* iterateEvalRecordFile(file) {
  for (const line of fs.readFileSync(file, "utf-8").split(/\r?\n/)) {
    if (!line.trim()) continue;
    try {
      yield JSON.parse(line);
    } catch (error) {
      throw new Error(`Failed to parse ${file}: ${String(error)}`);
    }
  }
}

export function readEvalRecords(directory) {
  return recordPaths(directory).flatMap((file) => Array.from(iterateEvalRecordFile(file)));
}

export function evalRecordsVersion(directory, suffix = ".jsonl", excludedEntries = []) {
  const files = recordPaths(directory, suffix, excludedEntries);
  if (!files.length) return { exists: false, mtimeMs: 0, size: 0, signature: "missing:0" };
  const entries = files.map((file) => {
    const stat = fs.statSync(file);
    return [path.relative(directory, file), stat.mtimeMs, stat.size];
  });
  return {
    exists: true,
    mtimeMs: Math.max(...entries.map((entry) => entry[1])),
    size: entries.reduce((sum, entry) => sum + entry[2], 0),
    // ファイルの増減・同サイズの置換も更新検知の対象にする。
    signature: crypto.createHash("sha256").update(JSON.stringify(entries)).digest("hex"),
  };
}
