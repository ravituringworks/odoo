"""Shared helpers for the extractors: where addons live (repo + user-imported paths)."""
import pathlib
ROOT = pathlib.Path(__file__).resolve().parents[2]
PORT = ROOT / "port"
EXTRA_FILE = PORT / "addons_paths.txt"      # one extra addons directory per line (written by import_addons.py)

def addon_dirs():
    dirs = [ROOT / "addons", ROOT / "odoo" / "addons"]
    if EXTRA_FILE.exists():
        for line in EXTRA_FILE.read_text().splitlines():
            line = line.strip()
            if line and not line.startswith("#") and pathlib.Path(line).is_dir(): dirs.append(pathlib.Path(line))
    return dirs

def module_dir(name):
    for b in addon_dirs():
        if (b / name / "__manifest__.py").exists(): return b / name
    return None
