#!/usr/bin/env python3
"""英汉表的数据门禁；不运行 Rust 引擎，不声称验证 Windows 上屏或全表准确率。"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
GLOSSARY = "assets/glossary/glossary-zh.tsv"
FIXTURE = ROOT / "tests/english_to_chinese/cases.tsv"
MANIFEST = ROOT / "tests/english_to_chinese/glossary_changes.json"
POS = {"n.", "v.", "adj.", "adv.", "pron.", "prep.", "conj.", "num.", "m.", "part.", "int.", "phr."}


def parse_table(data: bytes) -> dict[str, list[str]]:
    if data.startswith(b"\xef\xbb\xbf"):
        raise ValueError("glossary must not have a UTF-8 BOM")
    entries: dict[str, list[str]] = {}
    for number, line in enumerate(data.decode("utf-8").splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        key, senses = fields[0], fields[1:]
        if key in entries or not key or key != key.lower():
            raise ValueError(f"line {number}: duplicate, empty or non-lowercase key {key!r}")
        if not 1 <= len(senses) <= 65535 or any(not s for s in senses):
            raise ValueError(f"line {number}: expected 1..65535 nonempty senses")
        if len(key.encode("utf-8")) > 65535:
            raise ValueError(f"line {number}: key exceeds storage field width")
        values = []
        for sense in senses:
            # 与原生 Glossary 一样，词性和 |读音 都是可选字段。
            field = sense.partition("|")[0].strip()
            head, separator, text = field.partition(" ")
            text = text.strip() if separator and head in POS else field
            reading = sense.partition("|")[2].strip()
            if len(text.encode("utf-8")) > 65535 or len(reading.encode("utf-8")) > 65535:
                raise ValueError(f"line {number}: text or reading exceeds storage field width")
            if not text:
                raise ValueError(f"line {number}: empty sense {sense!r}")
            values.append(text)
        entries[key] = values
    return entries


def fixture_results(entries: dict[str, list[str]]) -> list[dict]:
    results = []
    for line in FIXTURE.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        source, expected, forbidden, limitation = line.split("\t")
        key = source
        if key.startswith('"'):
            key = key[1:-1] if len(key) >= 2 and key.endswith('"') else ""
        actual = entries.get(key.lower(), [])
        desired = expected.split("|") if expected else []
        banned = forbidden.split("|") if forbidden else []
        good = actual == desired and not any(b in text for b in banned for text in actual)
        results.append({"input": source, "expected": desired, "actual": actual,
                        "passed": good, "known_scope_or_omission": limitation})
    return results


def verify_changes(data: bytes) -> dict:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    baseline = subprocess.check_output(
        ["git", "show", f"{manifest['baseline_commit']}:{GLOSSARY}"], cwd=ROOT
    )
    if hashlib.sha256(baseline).hexdigest() != manifest["baseline_sha256"]:
        raise ValueError("pinned upstream glossary hash differs")
    original = {l.split("\t")[0]: l for l in baseline.decode("utf-8").splitlines() if l and not l.startswith("#")}
    current = {l.split("\t")[0]: l for l in data.decode("utf-8").splitlines() if l and not l.startswith("#")}
    declared = {c["key"]: c for c in manifest["changes"]}
    changed = {k for k in original.keys() | current.keys() if original.get(k) != current.get(k)}
    if changed != declared.keys():
        raise ValueError(f"undeclared or missing glossary changes: {changed ^ declared.keys()}")
    for key, change in declared.items():
        if original.get(key) != change["before"] or current.get(key) != change["after"]:
            raise ValueError(f"change manifest differs: {key}")
    if len(original) != manifest["baseline_entries"] or len(current) != manifest["modified_entries"]:
        raise ValueError("glossary entry count differs from the change manifest")
    return {"baseline_commit": manifest["baseline_commit"],
            "baseline_entries": len(original), "current_entries": len(current),
            "declared_changes": len(changed), "baseline_sha256": manifest["baseline_sha256"],
            "current_sha256": hashlib.sha256(data).hexdigest()}


def verify_retained_notices(commit: str) -> list[str]:
    paths = ["LICENSE", "assets/lexicon/README.md", "assets/lexicon/05_english/README.md",
             "assets/lexicon/05_english/sources/ESDB_Copyright.txt",
             "assets/lexicon/05_english/sources/CSpell_LICENSE-MIT.txt",
             "assets/lexicon/05_english/sources/typos_LICENSE-MIT.txt"]
    for path in paths:
        original = subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT)
        # Git for Windows may check text files out with CRLF.
        current = (ROOT / path).read_bytes().replace(b"\r\n", b"\n")
        if current != original.replace(b"\r\n", b"\n"):
            raise ValueError(f"retained notice changed: {path}")
    return paths


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    data = (ROOT / GLOSSARY).read_bytes()
    entries = parse_table(data)
    manifest = verify_changes(data)
    cases = fixture_results(entries)
    notices = verify_retained_notices(manifest["baseline_commit"])
    required = {"studio", "thwart", "set", "apple", "develop", "bank", "right", "privacy"}
    if not required <= {c["input"] for c in cases}:
        raise ValueError("required semantic fixtures missing")
    # 确认门禁会抓到旧错例及错误地删除短语限定；只改内存副本。
    for key, bad in [("studio", "学习"), ("thwart", "银行"), ("set", "创造")]:
        mutated = dict(entries)
        mutated[key] = [bad]
        if all(c["passed"] for c in fixture_results(mutated)):
            raise ValueError(f"semantic regression detector did not reject {key}->{bad}")
    report = {"scope": "data-only: not Rust tests, UI tests, human review or global accuracy",
              "status": "passed" if all(c["passed"] for c in cases) else "failed",
              **manifest, "fixture_cases": len(cases),
              "passed_cases": sum(c["passed"] for c in cases),
              "retained_notices": notices, "regression_mutations_rejected": 3,
              "cases": cases}
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k != "cases"}, ensure_ascii=False, indent=2))
    for case in cases:
        if not case["passed"]:
            print(json.dumps(case, ensure_ascii=False), file=sys.stderr)
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"data audit failed: {error}", file=sys.stderr)
        raise SystemExit(1)
