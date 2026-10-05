//! Pure DDL generation: Registry × Dialect → ordered SQL statements.
use crate::dialect::Dialect;
use crate::schema::{ModelDef, Registry};
use std::collections::BTreeSet;

fn magic_columns(d: &dyn Dialect) -> Vec<String> {
    let ts = d.column_type(&crate::schema::FieldDef { ty: "Datetime".into(), ..Default::default() });
    let int = d.column_type(&crate::schema::FieldDef { ty: "Integer".into(), ..Default::default() });
    vec![format!("create_uid {int}"), format!("create_date {ts}"), format!("write_uid {int}"), format!("write_date {ts}")]
}

/// One column definition ("name TYPE [DEFAULT ...]"); DEFAULTs only where the literal is valid for the column type.
pub fn column_def(n: &str, f: &crate::schema::FieldDef, d: &dyn Dialect) -> String {
    let mut c = format!("{} {}", d.quote(n), d.column_type(f));
    if let Some(def) = f.static_default() {
        let numeric = matches!(f.ty.as_str(), "Integer" | "Float" | "Monetary");
        let texty = matches!(f.ty.as_str(), "Char" | "Text" | "Selection" | "Html" | "Reference");
        match def {
            serde_json::Value::Bool(b) if f.ty == "Boolean" => c += &format!(" DEFAULT {}", d.bool_lit(b)),
            serde_json::Value::Number(x) if numeric => c += &format!(" DEFAULT {x}"),
            serde_json::Value::String(s) if texty => c += &format!(" DEFAULT '{}'", s.replace('\'', "''")),
            _ => {}
        }
    }
    c
}

pub fn create_table(m: &ModelDef, d: &dyn Dialect, installed: &BTreeSet<String>) -> Vec<String> {
    let t = m.table();
    let mut cols = vec![format!("{} {}", d.quote("id"), d.pk_for(&t))];
    for (n, f) in m.stored_fields() {
        if n == "id" { continue; }
        cols.push(column_def(n, f, d));
    }
    for mc in magic_columns(d) { let (n, ty) = mc.split_once(' ').unwrap(); if !m.fields.contains_key(n) { cols.push(format!("{} {}", d.quote(n), ty)); } }
    let _ = installed;
    let mut out = vec![];
    if let Some(p) = d.pre_table(&t) { out.push(p); }
    out.push(format!("CREATE TABLE IF NOT EXISTS {} (\n  {}\n)", d.quote(&t), cols.join(",\n  ")));
    out
}

/// Full install plan: tables, m2m relation tables, indexes, then deferred foreign keys.
pub fn install_plan(reg: &Registry, d: &dyn Dialect) -> Vec<String> {
    let installed: BTreeSet<String> = reg.models.values().filter(|m| m.has_table()).map(|m| m.name.clone()).collect();
    let mut tables = vec![]; let mut rels = vec![]; let mut idx = vec![]; let mut fks = vec![];
    let mut seen_rel = BTreeSet::new();
    // models may share one physical table (e.g. ir.actions.*): plan each table once over the union of columns
    let mut by_table: std::collections::BTreeMap<String, ModelDef> = std::collections::BTreeMap::new();
    for m in reg.models.values().filter(|m| m.has_table()) {
        by_table.entry(m.table()).and_modify(|e| { for (k, f) in &m.fields { e.fields.entry(k.clone()).or_insert_with(|| f.clone()); } }).or_insert_with(|| m.clone());
    }
    let mut seen_stmt = BTreeSet::new();
    for m in by_table.values() {
        tables.extend(create_table(m, d, &installed));
        let t = m.table();
        for (n, f) in &m.fields {
            if f.ty == "Many2many" {
                if let Some((rt, c1, c2)) = f.relation_table_name(&m.name, n, reg) {
                    if seen_rel.insert(rt.clone()) {
                        let it = d.column_type(&crate::schema::FieldDef { ty: "Integer".into(), ..Default::default() });
                        rels.push(format!("CREATE TABLE IF NOT EXISTS {} ({} {it} NOT NULL, {} {it} NOT NULL, PRIMARY KEY ({}, {}))", d.quote(&rt), d.quote(&c1), d.quote(&c2), d.quote(&c1), d.quote(&c2)));
                        idx.push(format!("CREATE INDEX IF NOT EXISTS {} ON {} ({})", d.quote(&format!("{rt}_{c2}_idx")), d.quote(&rt), d.quote(&c2)));
                    }
                }
            }
            if !f.is_stored() { continue; }
            let truthy = f.index.as_ref().map(|v| v.as_bool().unwrap_or(v.is_string())).unwrap_or(false);
            if f.ty == "Many2one" || truthy {
                idx.push(format!("CREATE INDEX IF NOT EXISTS {} ON {} ({})", d.quote(&format!("{t}_{n}_idx")), d.quote(&t), d.quote(n)));
            }
            if d.alter_fk() && f.ty == "Many2one" {
                if let Some(co) = f.comodel_name() {
                    if installed.contains(&co) {
                        let od = match f.ondelete.as_ref().and_then(|v| v.as_str()) { Some("cascade") => "CASCADE", Some("restrict") => "RESTRICT", _ => "SET NULL" };
                        fks.push(format!("ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} (id) ON DELETE {od}", d.quote(&t), d.quote(&format!("{t}_{n}_fkey")), d.quote(n), d.quote(&reg.table(&co))));
                    }
                }
            }
        }
    }
    tables.into_iter().chain(rels).chain(idx).chain(fks).filter(|s| seen_stmt.insert(s.clone())).collect()
}

/// Statements to move a database installed from `old` to `new` (new modules): `ALTER TABLE .. ADD COLUMN` for fields
/// that extended existing tables, plus every new table / relation table / index / foreign key. Pure and idempotent-friendly.
pub fn upgrade_plan(old: &Registry, new: &Registry, d: &dyn Dialect) -> Vec<String> {
    let collect = |r: &Registry| -> std::collections::BTreeMap<String, ModelDef> {
        let mut by: std::collections::BTreeMap<String, ModelDef> = std::collections::BTreeMap::new();
        for m in r.models.values().filter(|m| m.has_table()) { by.entry(m.table()).and_modify(|e| { for (k, f) in &m.fields { e.fields.entry(k.clone()).or_insert_with(|| f.clone()); } }).or_insert_with(|| m.clone()); }
        by
    };
    let (ot, nt) = (collect(old), collect(new));
    let old_plan: BTreeSet<String> = install_plan(old, d).into_iter().collect();
    let mut out = vec![];
    for (table, m) in &nt {
        let Some(om) = ot.get(table) else { continue };
        for (n, f) in m.stored_fields() {
            if n == "id" || om.fields.get(n).map_or(false, |x| x.is_stored()) { continue; }
            out.push(format!("ALTER TABLE {} ADD COLUMN {}", d.quote(table), column_def(n, f, d)));
        }
    }
    for s in install_plan(new, d) {
        if old_plan.contains(&s) { continue; }
        if s.starts_with("CREATE TABLE IF NOT EXISTS") && ot.keys().any(|t| s.starts_with(&format!("CREATE TABLE IF NOT EXISTS {} (", d.quote(t)))) { continue; }  // existing table: columns handled above
        out.push(s);
    }
    out
}

/// Self-healing schema sync: statements that bring an existing database up to `reg` (missing tables with their relation tables,
/// indexes and foreign keys; missing columns on existing tables). `existing` maps table -> columns as introspected from the DB.
pub fn sync_plan(reg: &Registry, existing: &std::collections::BTreeMap<String, BTreeSet<String>>, d: &dyn Dialect) -> Vec<String> {
    let q = d.quote("x").chars().next().unwrap_or('"');
    let first_ident = |s: &str, after: &str| -> Option<String> {
        let i = s.find(after)? + after.len(); let rest = &s[i..]; let a = rest.find(q)?; let b = rest[a + 1..].find(q)?; Some(rest[a + 1..a + 1 + b].to_string())
    };
    let stmt_table = |s: &str| -> Option<String> {
        if s.starts_with("CREATE TABLE IF NOT EXISTS") { first_ident(s, "EXISTS") }
        else if s.starts_with("CREATE INDEX") { first_ident(s, " ON ") }
        else if s.starts_with("ALTER TABLE") { first_ident(s, "ALTER TABLE") }
        else if s.starts_with("CREATE SEQUENCE") { s.split_whitespace().last().map(|n| n.trim_end_matches("_id_seq").to_string()) }
        else { None }
    };
    let mut by: std::collections::BTreeMap<String, ModelDef> = std::collections::BTreeMap::new();
    for m in reg.models.values().filter(|m| m.has_table()) { by.entry(m.table()).and_modify(|e| { for (k, f) in &m.fields { e.fields.entry(k.clone()).or_insert_with(|| f.clone()); } }).or_insert_with(|| m.clone()); }
    let mut out = vec![];
    for (table, m) in &by {
        let Some(cols) = existing.get(table) else { continue };
        for (n, f) in m.stored_fields() {
            if n == "id" || cols.contains(n) { continue; }
            out.push(format!("ALTER TABLE {} ADD COLUMN {}", d.quote(table), column_def(n, f, d)));
            if f.ty == "Many2one" { out.push(format!("CREATE INDEX IF NOT EXISTS {} ON {} ({})", d.quote(&format!("{table}_{n}_idx")), d.quote(table), d.quote(n))); }
        }
    }
    for s in install_plan(reg, d) { if let Some(t) = stmt_table(&s) { if !existing.contains_key(&t) { out.push(s); } } }
    out
}
