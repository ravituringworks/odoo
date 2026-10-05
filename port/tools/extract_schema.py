#!/usr/bin/env python3
"""Extract Odoo model/field declarations into a declarative JSON schema.

Pure AST walk (no Odoo import). Output: port/schema/<module>.json, plus
port/schema/_manifest.json with module dependency order and coverage stats.

Each model entry:  {name, inherit[], inherits{}, description, order, rec_name,
                    table, abstract, transient, fields{...}, methods[...]}
Each field entry:  {type, string, required, readonly, comodel, inverse_name,
                    selection, default, compute, related, store, index, size,
                    digits, relation_table, ondelete, translate, copy, groups}
"""
import ast, json, re, sys, pathlib

import sys; sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import ROOT, addon_dirs
OUT = ROOT / "port" / "schema"
ADDON_DIRS = addon_dirs()

FIELD_TYPES = {
    "Char", "Text", "Html", "Integer", "Float", "Monetary", "Boolean", "Date",
    "Datetime", "Selection", "Many2one", "One2many", "Many2many", "Binary",
    "Image", "Json", "Reference", "Id", "Properties", "PropertiesDefinition",
    "Many2oneReference", "Float",
}
KW = ["string", "required", "readonly", "comodel_name", "inverse_name", "compute",
      "related", "store", "index", "size", "digits", "relation", "ondelete",
      "translate", "copy", "groups", "currency_field", "tracking", "domain",
      "column1", "column2", "help", "default", "selection", "selection_add", "states"]


def lit(node):
    try:
        return ast.literal_eval(node)
    except Exception:
        if isinstance(node, ast.Name):
            return {"$ref": node.id}
        if isinstance(node, ast.Attribute):
            return {"$ref": ast.unparse(node)}
        if isinstance(node, ast.Lambda):
            return {"$lambda": ast.unparse(node.body)[:200]}
        if isinstance(node, ast.Call):
            return {"$call": ast.unparse(node)[:200]}
        return {"$expr": ast.unparse(node)[:200]}


def field_of(call):
    f = call.func
    if not (isinstance(f, ast.Attribute) and isinstance(f.value, ast.Name)
            and f.value.id == "fields" and f.attr in FIELD_TYPES):
        return None
    d = {"type": f.attr}
    pos = [lit(a) for a in call.args]
    rel = f.attr in ("Many2one", "One2many", "Many2many")
    if rel and pos:
        d["comodel"] = pos.pop(0)
        if f.attr == "One2many" and pos:
            d["inverse_name"] = pos.pop(0)
        if f.attr == "Many2many" and pos:
            d["relation_table"] = pos.pop(0)
            if pos: d["column1"] = pos.pop(0)
            if pos: d["column2"] = pos.pop(0)
        if pos: d["string"] = pos.pop(0)
    elif f.attr == "Selection" and pos:
        d["selection"] = pos.pop(0)
        if pos: d["string"] = pos.pop(0)
    elif pos:
        d["string"] = pos.pop(0)
    for k in call.keywords:
        if k.arg == "comodel_name": d["comodel"] = lit(k.value)
        elif k.arg == "relation": d["relation_table"] = lit(k.value)
        elif k.arg in KW: d[k.arg] = lit(k.value)
    # callable selections: `selection='_get_x'` or `lambda self: self._get_x()` -> resolved from the model's selection methods
    sel = d.get("selection")
    if isinstance(sel, str) and sel.startswith("_"):
        d["selection_method"] = d.pop("selection")
    elif isinstance(sel, dict):
        mm = re.search(r"self\.(_\w+)\(", json.dumps(sel))
        if mm: d["selection_method"] = mm.group(1); d.pop("selection")
    return d


def _pair(n):
    """('key', 'Label') or ('key', _('Label')) -> [key, label]"""
    if isinstance(n, (ast.Tuple, ast.List)) and len(n.elts) >= 2:
        k, l = n.elts[0], n.elts[1]
        # translation wrappers: _('x'), _lt('x'), self.env._('x')
        if isinstance(l, ast.Call) and l.args and isinstance(l.args[0], ast.Constant) and (getattr(l.func, "id", "") in ("_", "_lt") or getattr(l.func, "attr", "") == "_"): l = l.args[0]
        if isinstance(k, ast.Constant) and isinstance(l, ast.Constant): return [k.value, l.value]
    return None

def _is_super_call(n):
    return isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute) and isinstance(n.func.value, ast.Call) and getattr(n.func.value.func, "id", "") == "super"

def analyze_selection_body(fn):
    """Imperative form: `sel = [..]; if cond: sel.append(..); sel += [..]; return sel` -> union of all literal contributions
    (conditions cannot be evaluated statically, so conditional appends are included)."""
    rets = [s for s in ast.walk(fn) if isinstance(s, ast.Return) and isinstance(s.value, ast.Name)]
    if not rets: return None
    var = rets[-1].value.id; items, seen_init = [], False
    for node in ast.walk(fn):
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and getattr(node.targets[0], "id", None) == var and isinstance(node.value, (ast.List, ast.Tuple)):
            got = analyze_selection(node.value)
            if got is None: return None
            items += got; seen_init = True
        elif isinstance(node, ast.AugAssign) and getattr(node.target, "id", None) == var:
            got = analyze_selection(node.value)
            if got is None: return None
            items += got
        elif isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and getattr(node.func.value, "id", None) == var:
            if node.func.attr == "append" and node.args:
                p = _pair(node.args[0])
                if p is None: return None
                items.append(p)
            elif node.func.attr == "extend" and node.args:
                got = analyze_selection(node.args[0])
                if got is None: return None
                items += got
    return items if seen_init else None

def analyze_selection(e):
    """Static value of a selection-method return expression: literal items, `super()._m() + [items]`, `x if cond else []`; None when not analyzable."""
    if isinstance(e, (ast.List, ast.Tuple)):
        items = [_pair(x) for x in e.elts]; return None if any(i is None for i in items) else items
    if _is_super_call(e): return []
    if isinstance(e, ast.BinOp) and isinstance(e.op, ast.Add):
        l, r = analyze_selection(e.left), analyze_selection(e.right); return None if l is None or r is None else l + r
    if isinstance(e, ast.IfExp): return analyze_selection(e.body)   # condition ignored: contributions are static
    return None

def scan_class(cls):
    attrs, fields, methods, sel_methods = {}, {}, [], {}
    for st in cls.body:
        if isinstance(st, ast.AnnAssign) and isinstance(st.target, ast.Name) and st.value is not None:
            st = ast.Assign(targets=[st.target], value=st.value, lineno=st.lineno)
        if isinstance(st, ast.Assign) and len(st.targets) == 1 and isinstance(st.targets[0], ast.Name):
            n = st.targets[0].id
            if n.startswith("_") and n in ("_name", "_inherit", "_inherits", "_description",
                                          "_order", "_rec_name", "_table", "_auto",
                                          "_abstract", "_transient", "_parent_name",
                                          "_parent_store", "_check_company_auto", "_sql_constraints"):
                attrs[n] = lit(st.value)
            elif isinstance(st.value, ast.Call):
                fd = field_of(st.value)
                if fd: fields[n] = fd
        elif isinstance(st, ast.FunctionDef):
            deco = []
            for d in st.decorator_list:
                s = ast.unparse(d)
                if s.startswith("api."): deco.append(s)
            rets = [s for s in st.body if isinstance(s, ast.Return) and s.value is not None]
            if st.name.startswith("_") and rets:
                items = analyze_selection(rets[-1].value)
                if items is None: items = analyze_selection_body(st)     # imperative list building
                if items is not None: sel_methods[st.name] = items
            methods.append({"name": st.name, "decorators": deco,
                            "lines": (st.end_lineno or st.lineno) - st.lineno + 1,
                            "action": st.name.startswith("action_")})
    return attrs, fields, methods, sel_methods


def is_model_class(cls):
    for b in cls.bases:
        s = ast.unparse(b)
        if s.endswith("Model") or s.endswith("AbstractModel") or s.endswith("TransientModel"):
            return s
    return None


GLOBAL_CONSTS = {}   # top-level NAME = [literal pairs] across all addons (e.g. SALE_ORDER_STATE)

def collect_consts(tree):
    for n in tree.body:
        if isinstance(n, (ast.Assign, ast.AnnAssign)):
            tgt = n.targets[0] if isinstance(n, ast.Assign) else n.target
            if isinstance(tgt, ast.Name) and n.value is not None and isinstance(n.value, (ast.List, ast.Tuple)):
                try: GLOBAL_CONSTS.setdefault(tgt.id, ast.literal_eval(n.value))
                except Exception: pass

def scan_module(path):
    mf = path / "__manifest__.py"
    if not mf.exists():
        return None
    try:
        man = ast.literal_eval(mf.read_text())
    except Exception:
        man = {}
    models = {}
    stats = {"py_lines": 0}
    for py in sorted(path.rglob("*.py")):
        if "/tests/" in str(py) or "/migrations/" in str(py) or py.name == "__manifest__.py":
            continue
        src = py.read_text(errors="ignore")
        stats["py_lines"] += src.count("\n")
        try:
            tree = ast.parse(src)
        except SyntaxError:
            continue
        collect_consts(tree)
        for cls in [n for n in tree.body if isinstance(n, ast.ClassDef)]:
            base = is_model_class(cls)
            if not base:
                continue
            attrs, fields, methods, sel_methods = scan_class(cls)
            name = attrs.get("_name")
            inh = attrs.get("_inherit")
            if isinstance(inh, str): inh = [inh]
            if not name and inh:
                name = inh[0]   # Odoo: without _name, the first _inherit entry is the extended model (the rest are mixins)
            if not name:
                continue
            m = models.setdefault(name, {
                "name": name, "inherit": [], "inherits": {}, "fields": {}, "methods": [],
                "abstract": False, "transient": False, "extension": False})
            if inh: m["inherit"] = sorted(set(m["inherit"]) | {i for i in inh if i != name})
            if attrs.get("_name") is None: m["extension"] = True
            if "_inherits" in attrs and isinstance(attrs["_inherits"], dict): m["inherits"].update(attrs["_inherits"])
            for k, out in (("_description", "description"), ("_order", "order"), ("_rec_name", "rec_name"),
                           ("_table", "table"), ("_parent_name", "parent_name"),
                           ("_parent_store", "parent_store")):
                if k in attrs: m[out] = attrs[k]
            # abstract/transient apply only to classes that declare their own `_name`; extensions (name inferred from _inherit)
            # never change what the extended model is (e.g. an AbstractModel-based extension of the concrete ir.cron)
            if attrs.get("_name"):
                if base.endswith("AbstractModel"): m["abstract"] = True
                if base.endswith("TransientModel"): m["transient"] = True
            if "_sql_constraints" in attrs: m["sql_constraints"] = attrs["_sql_constraints"]
            m["fields"].update(fields)
            for k, v in sel_methods.items(): m.setdefault("sel_methods", {}).setdefault(k, []).extend(v)
            m["methods"] += [x for x in methods if not x["name"].startswith("__")]
    data_files = man.get("data", []) + man.get("demo", [])
    return {
        "module": path.name,
        "depends": man.get("depends", []),
        "category": man.get("category"),
        "title": man.get("name") or path.name,
        "summary": (man.get("summary") or man.get("description") or "").strip().split("\n")[0][:200],
        "author": man.get("author"),
        "path": str(path),
        "application": bool(man.get("application")),
        "version": man.get("version"),
        "data_files": data_files,
        "models": models,
        "stats": {**stats, "models": len(models),
                  "fields": sum(len(m["fields"]) for m in models.values()),
                  "methods": sum(len(m["methods"]) for m in models.values())},
    }


def toposort(mods):
    order, seen = [], set()
    def visit(n):
        if n in seen or n not in mods: return
        seen.add(n)
        for d in mods[n]["depends"]: visit(d)
        order.append(n)
    for n in sorted(mods): visit(n)
    return order


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    mods = {}
    for base in ADDON_DIRS:
        for p in sorted(base.iterdir()):
            if p.is_dir():
                r = scan_module(p)
                if r:
                    mods[r["module"]] = r
                    (OUT / f"{r['module']}.json").write_text(json.dumps(r, indent=1, sort_keys=True))
    # resolve selection=NAME references against collected module-level constants
    resolved = 0
    for m in mods.values():
        for model in m["models"].values():
            for f in model["fields"].values():
                s = f.get("selection")
                if isinstance(s, dict) and "$ref" in s and s["$ref"].split(".")[-1] in GLOBAL_CONSTS:
                    f["selection"] = [list(x) for x in GLOBAL_CONSTS[s["$ref"].split(".")[-1]]]; resolved += 1
        (OUT / f"{m['module']}.json").write_text(json.dumps(m, indent=1, sort_keys=True))
    print("resolved selection refs:", resolved)
    order = toposort(mods)
    tot = {k: sum(m["stats"][k] for m in mods.values()) for k in ("models", "fields", "methods", "py_lines")}
    manifest = {"order": order, "totals": tot, "modules": {
        n: {"depends": m["depends"], "application": m["application"], "title": m["title"], "summary": m["summary"], "category": m["category"], **m["stats"]} for n, m in mods.items()}}
    (OUT / "_manifest.json").write_text(json.dumps(manifest, indent=1))
    print(f"modules={len(mods)} {tot}")


if __name__ == "__main__":
    main()
