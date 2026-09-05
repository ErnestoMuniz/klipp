#!/usr/bin/env python3
"""Gera klipp/assets/emoji-*.json a partir do dataset do klipp-old (Electron).

Uso: python3 scripts/gen-emoji.py   (na pasta klipp/)
Mantém grupos/subgrupos/nomes em inglês + índice de busca pt-BR (CLDR).
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = ROOT.parent / "klipp-old" / "src" / "components" / "Soundboard"
ASSETS = ROOT / "assets"


def parse_groups() -> list:
    txt = (OLD / "emojiData.ts").read_text(encoding="utf-8")
    groups: list = []
    cur_g = cur_s = None
    for line in txt.splitlines():
        m = re.match(r'^    label: "(.*)",\s*$', line)
        if m:
            cur_g = {"label": m.group(1), "subgroups": []}
            groups.append(cur_g)
            cur_s = None
            continue
        m = re.match(r'^        label: "(.*)",\s*$', line)
        if m:
            assert cur_g is not None, "subgroup sem group"
            cur_s = {"label": m.group(1), "emojis": []}
            cur_g["subgroups"].append(cur_s)
            continue
    # Opções na ordem do arquivo, ligadas ao subgrupo corrente via índice.
    opt_re = re.compile(r'\{\s*emoji:\s*"(.*?)"\s*,\s*name:\s*"(.*?)"', re.S)
    labels = [(m.start(), m) for m in re.finditer(r'^        label: "(.*)",\s*$', txt, re.M)]
    flat_subs = [s for g in groups for s in g["subgroups"]]
    assert len(labels) == len(flat_subs), f"{len(labels)} labels x {len(flat_subs)} subs"
    for i, opt in enumerate(opt_re.finditer(txt)):
        pass  # preenchido abaixo por posição
    # Associa cada opção ao subgrupo cujo label a precede.
    bounds = [pos for pos, _ in labels] + [len(txt)]
    idx = 0
    for opt in opt_re.finditer(txt):
        while idx + 1 < len(bounds) and opt.start() >= bounds[idx + 1]:
            idx += 1
        flat_subs[idx]["emojis"].append({"e": opt.group(1), "n": opt.group(2)})
    return groups


def parse_search_pt() -> dict:
    out: dict = {}
    pat = re.compile(r'^\s*"([^"]+)"\s*:\s*["\'](.*)["\'],?\s*$')
    for line in (OLD / "emojiSearchData.ts").read_text(encoding="utf-8").splitlines():
        m = pat.match(line)
        if m and m.group(1) != "pt-BR":
            out[m.group(1)] = m.group(2)
    return out


def main() -> None:
    groups = parse_groups()
    search_pt = parse_search_pt()
    n_opt = sum(len(e["emojis"]) for g in groups for e in g["subgroups"])
    n_sub = sum(len(g["subgroups"]) for g in groups)
    print(f"groups={len(groups)} subgroups={n_sub} emojis={n_opt} search_pt={len(search_pt)}")
    assert len(groups) == 9 and n_opt > 3000, "dataset incompleto?"
    ASSETS.mkdir(parents=True, exist_ok=True)
    (ASSETS / "emoji-groups.json").write_text(
        json.dumps(groups, ensure_ascii=False), encoding="utf-8")
    (ASSETS / "emoji-search-pt.json").write_text(
        json.dumps(search_pt, ensure_ascii=False), encoding="utf-8")
    print("ok:", ASSETS / "emoji-groups.json", ASSETS / "emoji-search-pt.json")


if __name__ == "__main__":
    sys.exit(main())
