#!/usr/bin/env python3
"""Build per-language catalogs from Odoo's own .po files -> port/i18n/<lang>.json
   {"fields": {"model.field": label}, "selection": {"model.field.key": label}, "menus": {xmlid: name}, "terms": {english: translated}}
   `terms` are restricted to strings that really occur in the extracted backend views (keeps the files small)."""
import json, pathlib, re, sys
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import PORT, module_dir

LANGS = ["es", "fr", "de", "pt_BR", "it", "ru", "hi", "ar", "zh_CN", "ja"]
OUT = PORT / "i18n"; OUT.mkdir(exist_ok=True)

def unq(s):
    return bytes(s, "utf-8").decode("unicode_escape").encode("latin-1", "ignore").decode("utf-8", "ignore") if "\\" in s and not s.isascii() is False else s

def parse_po(path):
    """yield (refs, msgid, msgstr)"""
    refs, msgid, msgstr, cur = [], None, None, None
    def flush():
        nonlocal refs, msgid, msgstr
        if msgid and msgstr: yield_list.append((refs, msgid, msgstr))
        refs, msgid, msgstr = [], None, None
    yield_list = []
    def lit(line):
        v = line[line.index('"') + 1: line.rindex('"')]
        return v.replace('\\"', '"').replace("\\n", "\n").replace("\\t", "\t").replace("\\\\", "\\")
    for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        if line.startswith("#:"):
            if msgid is not None: flush()
            refs += line[2:].split()
        elif line.startswith("msgid "):
            if msgid is not None and msgstr is not None: flush()
            msgid, cur = lit(line), "id"
        elif line.startswith("msgstr "): msgstr, cur = lit(line), "str"
        elif line.startswith('"') and cur:
            if cur == "id": msgid += lit(line)
            else: msgstr += lit(line)
        elif not line.strip():
            if msgid is not None: flush()
            cur = None
    if msgid is not None: flush()
    return yield_list

def view_strings():
    out = set()
    for f in (PORT / "views").glob("*.json"):
        def walk(n):
            for k in ("string", "placeholder", "help", "title", "confirm"): 
                v = n["attrs"].get(k)
                if v: out.add(v)
            if n.get("text"): out.add(n["text"])
            for c in n["children"]: walk(c)
        for v in json.load(open(f)).values(): walk(v)
    return out

def main():
    manifest = json.load(open(PORT / "schema" / "_manifest.json"))
    table2model = {}
    for m in manifest["modules"]:
        for model in json.load(open(PORT / "schema" / f"{m}.json"))["models"]: table2model[model.replace(".", "_")] = model
    vstr = view_strings()
    for lang in LANGS:
        fields, selection, menus, terms = {}, {}, {}, {}
        for mod in manifest["modules"]:
            d = module_dir(mod)
            po = d / "i18n" / f"{lang}.po" if d else None
            if not po or not po.exists(): continue
            for refs, mid, mstr in parse_po(po):
                if mid == mstr: continue
                for r in refs:
                    if r.startswith("model:ir.model.fields,field_description:"):
                        m = re.match(r"model:ir\.model\.fields,field_description:[\w.]*?field_(.+?)__(.+)$", r)
                        if m and m.group(1) in table2model: fields[f"{table2model[m.group(1)]}.{m.group(2)}"] = mstr
                    elif r.startswith("model:ir.model.fields.selection,name:"):
                        m = re.match(r"model:ir\.model\.fields\.selection,name:[\w.]*?selection__(.+?)__(.+?)__(.+)$", r)
                        if m and m.group(1) in table2model: selection[f"{table2model[m.group(1)]}.{m.group(2)}.{m.group(3)}"] = mstr
                    elif r.startswith("model:ir.ui.menu,name:"): menus[r.split(":", 2)[2]] = mstr
                    elif r.startswith("model_terms:ir.ui.view,arch_db:") or r.startswith("model:ir.actions.act_window,name:"):
                        if mid in vstr and "<" not in mid: terms.setdefault(mid, mstr)
                # menus/selections/field labels also serve as generic terms (buttons, headings reuse them)
                if mid in vstr and ("model:ir.ui.menu" in " ".join(refs) or "field_description" in " ".join(refs)) and "<" not in mid: terms.setdefault(mid, mstr)
        (OUT / f"{lang}.json").write_text(json.dumps({"fields": fields, "selection": selection, "menus": menus, "terms": terms}, ensure_ascii=False, separators=(",", ":")))
        print(f"{lang}: fields={len(fields)} selection={len(selection)} menus={len(menus)} terms={len(terms)} size={(OUT / f'{lang}.json').stat().st_size // 1024}KB")
main()
