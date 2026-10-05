#!/usr/bin/env python3
"""Resolve Odoo ir.ui.view inheritance (xpath/field positions) into one merged arch per (model, view type).
Output: port/views/<model>.json  {"form": <node>, "list": <node>, "kanban": <node>, "search": <node>}
node = {"tag", "attrs", "text", "children"}.  Unsupported xpath expressions are counted, not fatal."""
import json, pathlib, re, copy, xml.etree.ElementTree as ET
import sys; sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import ROOT, module_dir
OUT = ROOT / "port" / "views"; OUT.mkdir(exist_ok=True, parents=True)
SCHEMA = ROOT / "port" / "schema"
TYPES = {"form", "list", "tree", "kanban", "search"}

def base_dir(m): return module_dir(m)

def q(mod, x): return x if "." in x else f"{mod}.{x}"

def load_views():
    order = json.load(open(SCHEMA / "_manifest.json"))["order"]
    views = {}
    for mod in order:
        info = json.load(open(SCHEMA / f"{mod}.json")); base = base_dir(mod)
        if not base: continue
        for rel in info.get("data_files", []):
            if not rel.endswith(".xml"): continue
            p = base / rel
            if not p.exists(): continue
            try: root = ET.parse(p).getroot()
            except ET.ParseError: continue
            for rec in root.iter("record"):
                if rec.get("model") != "ir.ui.view": continue
                f = {x.get("name"): x for x in rec.findall("field")}
                arch = f.get("arch")
                if arch is None or len(arch) == 0: continue
                vid = q(mod, rec.get("id") or "")
                inh = f["inherit_id"].get("ref") if "inherit_id" in f else None
                views[vid] = {"id": vid, "module": mod, "model": (f["model"].text or "").strip() if "model" in f else None,
                              "inherit": q(mod, inh) if inh else None, "mode": (f["mode"].text or "extension").strip() if "mode" in f else "extension",
                              "priority": int((f["priority"].text or "16").strip()) if "priority" in f and (f["priority"].text or "").strip().isdigit() else 16,
                              "arch": arch[0], "seq": len(views)}
    return views

def parent_map(root): return {c: p for p in root.iter() for c in p}

XP = re.compile(r"^//?(\w+|\*)(?:\[@(\w+)=['\"]([^'\"]+)['\"]\])?(?:\[(\d+)\])?$")
def find_target(root, spec):
    """spec: element <field name=..> shorthand or <xpath expr=..>; returns element or None"""
    if spec.tag == "xpath":
        m = XP.match(spec.get("expr", "").strip())
        if not m: return None
        tag, attr, val, idx = m.groups()
        cands = [e for e in root.iter() if (tag == "*" or e.tag == tag) and (attr is None or e.get(attr) == val)]
        if idx: return cands[int(idx) - 1] if len(cands) >= int(idx) else None
        return cands[0] if cands else None
    attrs = {k: v for k, v in spec.attrib.items() if k not in ("position", "string") or spec.tag != "field"}
    key = {k: v for k, v in attrs.items() if k in ("name",)}
    for e in root.iter():
        if e.tag == spec.tag and all(e.get(k) == v for k, v in key.items()): return e
    return None

def tag(el, module):
    """Remember which module contributed a node (`_m`) so the server can drop it when that module is not installed."""
    if module: el.set("_m", module)
    return el

def apply_ext(root, ext, stats, module=None):
    # root may itself be the spec (<xpath>/<field position=..>) or a wrapper (<data>, <form>...) holding specs
    specs = [ext] if (ext.tag == "xpath" or ext.get("position")) else list(ext)
    for spec in specs:
        pos = spec.get("position", "inside")
        t = find_target(root, spec)
        if t is None: stats["miss"] += 1; continue
        par = parent_map(root)
        if pos == "attributes":
            for a in spec.findall("attribute"):
                n = a.get("name"); v = (a.text or "").strip()
                if v == "" and a.get("add") is None: t.attrib.pop(n, None)
                else: t.set(n, v)
        elif pos == "replace":
            p = par.get(t)
            if p is None: continue
            i = list(p).index(t); p.remove(t)
            for j, c in enumerate(list(spec)): p.insert(i + j, tag(copy.deepcopy(c), module))
        elif pos in ("before", "after"):
            p = par.get(t)
            if p is None: continue
            i = list(p).index(t) + (1 if pos == "after" else 0)
            for j, c in enumerate(copy.deepcopy(list(spec))): p.insert(i + j, tag(c, module))   # content = children of the locator
        elif pos == "inside":
            for c in copy.deepcopy(list(spec)): t.append(tag(c, module))
        stats["ok"] += 1

def to_json(e):
    """Element -> node. Tail text (`( <field/> )`) becomes separate "#text" children so mixed content survives."""
    kids = []
    for c in e:
        kids.append(to_json(c))
        tail = (c.tail or "").strip()
        if tail: kids.append({"tag": "#text", "attrs": {}, "text": tail, "children": []})
    return {"tag": e.tag, "attrs": dict(e.attrib), "text": (e.text or "").strip() or None, "children": kids}

def main():
    views = load_views()
    primaries = {}
    for v in sorted(views.values(), key=lambda v: (v["priority"], v["seq"])):
        if v["inherit"] is None or v["mode"] == "primary":
            t = v["arch"].tag; t = "list" if t == "tree" else t
            if t in TYPES and v["model"]: primaries.setdefault((v["model"], t), v)
    # extensions per root view (follow chains)
    children = {}
    for v in views.values():
        if v["inherit"] and v["mode"] != "primary": children.setdefault(v["inherit"], []).append(v)
    stats = {"ok": 0, "miss": 0}; out = {}
    for (model, t), pv in primaries.items():
        root = copy.deepcopy(pv["arch"])
        def walk(vid):
            for ext in sorted(children.get(vid, []), key=lambda v: (v["priority"], v["seq"])):
                apply_ext(root, ext["arch"], stats, ext["module"]); walk(ext["id"])
        walk(pv["id"])
        out.setdefault(model, {})[t] = to_json(root)
    for model, d in out.items(): (OUT / f"{model}.json").write_text(json.dumps(d, separators=(",", ":")))
    print(f"models={len(out)} views={sum(len(d) for d in out.values())} ext_applied={stats['ok']} ext_missed={stats['miss']}")
main()
