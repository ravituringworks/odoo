#!/usr/bin/env python3
"""Extract declarative data (menus, window actions, ACL, groups, sequences) per module -> port/data/<module>.json"""
import ast, csv, json, pathlib, xml.etree.ElementTree as ET

import sys; sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import ROOT, module_dir
OUT = ROOT / "port" / "data"; OUT.mkdir(exist_ok=True, parents=True)
SCHEMA = ROOT / "port" / "schema"

def conv(expr):
    """Safe subset of Odoo `eval=` -> JSON; ref('x') => {"$ref": "x"}."""
    def go(n):
        if isinstance(n, ast.Call) and getattr(n.func, "id", "") == "ref" and n.args:
            return {"$ref": ast.literal_eval(n.args[0])}
        if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute) and getattr(n.func.value, "id", "") == "Command":
            a = [go(x) for x in n.args]; k = n.func.attr
            return {"link": [4] + a[:1], "unlink": [3] + a[:1], "set": [6, 0] + a[:1], "clear": [5], "create": [0, 0] + a[:1], "delete": [2] + a[:1], "update": [1] + a[:2]}[k]
        if isinstance(n, (ast.List, ast.Tuple)): return [go(e) for e in n.elts]
        if isinstance(n, ast.Dict): return {ast.literal_eval(k): go(v) for k, v in zip(n.keys, n.values)}
        return ast.literal_eval(n)
    try: return go(ast.parse(expr.strip(), mode="eval").body)
    except Exception: return {"$unevaluated": expr[:120]}

def pylit(s):
    """python-literal domain/context text -> JSON (tuples become lists); None if not a pure literal."""
    try: return ast.literal_eval(s.strip())
    except Exception: return None

def field_val(f):
    if f.get("name") in ("domain", "context") and f.text and f.get("eval") is None:
        v = pylit(f.text)
        return v if v is not None else {"$unevaluated": f.text.strip()[:160]}
    if f.get("ref"): return {"$ref": f.get("ref")}
    if f.get("eval") is not None: return conv(f.get("eval"))
    if f.get("search"): return {"$unevaluated": "search"}
    return (f.text or "").strip() if len(f) == 0 else {"$unevaluated": "xml"}

def parse_xml(path, module):
    recs, menus = [], []
    try: root = ET.parse(path).getroot()
    except ET.ParseError: return recs, menus
    def walk(el, parent_menu=None):
        if el.tag == "menuitem":
            d = {k: el.get(k) for k in ("id", "name", "parent", "action", "sequence", "groups", "web_icon") if el.get(k)}
            if parent_menu and "parent" not in d: d["parent"] = parent_menu
            menus.append(d)
            for c in el: walk(c, el.get("id"))
            return
        if el.tag == "record" and el.get("model"):
            handle(el)
        for c in el: walk(c, parent_menu)
    def handle(el):
        if True:
            m = el.get("model")
            if m in ("ir.ui.view", "ir.ui.menu.dummy", "ir.actions.report", "mail.template", "ir.cron"): return
            recs.append({"model": m, "id": el.get("id"), "values": {f.get("name"): field_val(f) for f in el.findall("field") if f.get("name")}})
    walk(root)
    return recs, menus

def main():
    tot = {"records": 0, "menus": 0, "access": 0, "modules": 0}
    for sf in sorted(SCHEMA.glob("*.json")):
        if sf.name.startswith("_"): continue
        info = json.loads(sf.read_text()); name = info["module"]
        base = module_dir(name)
        if not base: continue
        recs, menus, access = [], [], []
        for rel in info.get("data_files", []):
            p = base / rel
            if not p.exists(): continue
            if rel.endswith(".xml"):
                r, m = parse_xml(p, name); recs += r; menus += m
            elif p.name == "ir.model.access.csv" or rel.endswith("ir.model.access.csv"):
                for row in csv.DictReader(p.open(newline="", encoding="utf-8")):
                    access.append({"id": row.get("id"), "model": (row.get("model_id:id") or row.get("model_id/id") or "").replace("model_", "", 1).replace("_", "."),
                                   "model_xmlid": row.get("model_id:id") or row.get("model_id/id"),
                                   "group": row.get("group_id:id") or row.get("group_id/id") or None,
                                   "read": row.get("perm_read") == "1", "write": row.get("perm_write") == "1",
                                   "create": row.get("perm_create") == "1", "unlink": row.get("perm_unlink") == "1"})
        (OUT / f"{name}.json").write_text(json.dumps({"module": name, "records": recs, "menus": menus, "access": access}, indent=1))
        tot["records"] += len(recs); tot["menus"] += len(menus); tot["access"] += len(access); tot["modules"] += 1
    print(tot)
main()
