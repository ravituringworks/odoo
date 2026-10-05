#!/usr/bin/env python3
"""Import Odoo Python addons into odoo-rs.

  python3 tools/import_addons.py /path/to/addons_dir_or_single_addon [more paths...]

Imports models, fields, selections, views (with xpath inheritance), menus, actions, ACL, record rules and seed data.
Python method bodies (computes, onchanges, buttons) are NOT translated: they are listed in port/imports/<module>.md
as the Rust porting backlog, and calling them returns "method ... is not ported yet" until a Rust rule exists.
After importing, restart nothing: the module shows up in the Apps page and can be installed at runtime."""
import json, pathlib, subprocess, sys
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import PORT, EXTRA_FILE

def main(paths):
    if not paths: print(__doc__); return 2
    known = set(EXTRA_FILE.read_text().split("\n")) if EXTRA_FILE.exists() else set()
    new = []
    for raw in paths:
        p = pathlib.Path(raw).expanduser().resolve()
        if not p.is_dir(): print(f"skip (not a directory): {p}"); continue
        base = p.parent if (p / "__manifest__.py").exists() else p     # single addon -> its parent dir
        if not any((c / "__manifest__.py").exists() for c in base.iterdir() if c.is_dir()): print(f"skip (no addons found): {base}"); continue
        if str(base) not in known: known.add(str(base)); new.append(str(base))
    EXTRA_FILE.write_text("\n".join(sorted(x for x in known if x)) + "\n")
    tools = pathlib.Path(__file__).parent
    for t in ("extract_schema.py", "extract_data.py", "extract_views.py"):
        r = subprocess.run([sys.executable, str(tools / t)], capture_output=True, text=True)
        print(r.stdout.strip().splitlines()[-1] if r.stdout.strip() else r.stderr[-300:])
    manifest = json.load(open(PORT / "schema" / "_manifest.json"))
    out = PORT / "imports"; out.mkdir(exist_ok=True)
    imported = [m for m, v in manifest["modules"].items() if any(str(pathlib.Path(b)) in json.load(open(PORT / "schema" / f"{m}.json")).get("path", "") for b in new)]
    for m in imported:
        s = json.load(open(PORT / "schema" / f"{m}.json"))
        lines = [f"# {m} — import report", "", f"models: {s['stats']['models']}, fields: {s['stats']['fields']}, methods: {s['stats']['methods']}", "", "## Python methods to port to Rust (rules)", ""]
        for name, model in s["models"].items():
            ms = [x for x in model["methods"] if x["decorators"] or x["action"] or x["name"].startswith(("_compute", "_onchange", "_inverse", "create", "write", "unlink"))]
            if ms: lines += [f"### {name}"] + [f"- `{x['name']}` {' '.join(x['decorators'])} ({x['lines']} lines)" for x in ms] + [""]
        (out / f"{m}.md").write_text("\n".join(lines))
        print(f"imported {m}: {s['stats']['models']} models, {s['stats']['fields']} fields; port backlog -> port/imports/{m}.md")
    if not imported: print("nothing new imported (already registered?)")
    return 0
sys.exit(main(sys.argv[1:]))
