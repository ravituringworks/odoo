//! Declarative data: seed records (XML data files), menus/actions catalog, ACL (ir.model.access).
use crate::util::*;
use odoo_core::orm::{self, AccessEntry, AccessTable, Env};
use odoo_core::schema::table_of;
use odoo_core::{Registry, Result, Row, Value};
use serde_json::Value as J;
use std::collections::BTreeMap;
use std::path::Path;

fn read_module(dir: &Path, m: &str) -> Option<J> { serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{m}.json"))).ok()?).ok() }

/// Menus + window actions, kept in memory for the UI shell.
#[derive(Default, Clone, Debug)]
pub struct Catalog { pub menus: Vec<J>, pub actions: BTreeMap<String, J> }

impl Catalog {
    pub fn load(dir: &Path, modules: &[String]) -> Catalog {
        let mut c = Catalog::default();
        for m in modules {
            let Some(d) = read_module(dir, m) else { continue };
            for mn in d["menus"].as_array().into_iter().flatten() {
                let mut mn = mn.clone();
                if let Some(o) = mn.as_object_mut() { o.insert("module".into(), J::String(m.clone())); }
                c.menus.push(mn);
            }
            for r in d["records"].as_array().into_iter().flatten() {
                if r["model"] == "ir.actions.act_window" {
                    if let Some(id) = r["id"].as_str() { let mut a = r["values"].clone(); a["xmlid"] = J::String(format!("{m}.{id}")); c.actions.insert(format!("{m}.{id}"), a); }
                }
            }
        }
        c
    }
    /// Menu tree with each leaf resolved to `{res_model, view_mode, domain}`; menus whose model isn't installed are pruned.
    pub fn menu_tree(&self, reg: &Registry) -> J {
        let qualify = |module: &str, id: &str| if id.contains('.') { id.to_string() } else { format!("{module}.{id}") };
        let mut nodes: BTreeMap<String, J> = BTreeMap::new();
        let mut parent_of: BTreeMap<String, String> = BTreeMap::new();
        for m in &self.menus {
            let module = m["module"].as_str().unwrap_or("");
            let Some(id) = m["id"].as_str() else { continue };
            let key = qualify(module, id);
            let action = m["action"].as_str().and_then(|a| self.actions.get(&qualify(module, a)));
            let res_model = action.and_then(|a| a["res_model"].as_str()).map(String::from);
            let name = m["name"].as_str().map(String::from).or_else(|| action.and_then(|a| a["name"].as_str().map(String::from))).unwrap_or_else(|| id.to_string());
            nodes.insert(key.clone(), serde_json::json!({
                "id": key, "name": name, "sequence": m["sequence"].as_str().and_then(|s| s.parse::<i64>().ok()).unwrap_or(10),
                "model": res_model, "view_mode": action.and_then(|a| a["view_mode"].as_str()), "children": [],
                "domain": action.map(|a| a["domain"].clone()).filter(|d| d.is_array()), "context": action.map(|a| a["context"].clone()).filter(|d| d.is_object()),
            }));
            if let Some(p) = m["parent"].as_str() { parent_of.insert(key, qualify(module, p)); }
        }
        // order-independent assembly: group children by parent, then build from roots recursively
        let mut kids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut roots_k = vec![];
        for k in nodes.keys() { match parent_of.get(k).filter(|p| nodes.contains_key(*p)) { Some(p) => kids.entry(p.clone()).or_default().push(k.clone()), None => roots_k.push(k.clone()) } }
        fn build(k: &str, nodes: &BTreeMap<String, J>, kids: &BTreeMap<String, Vec<String>>, reg: &Registry, depth: usize) -> Option<J> {
            let mut n = nodes.get(k)?.clone();
            let mut ch: Vec<J> = if depth > 12 { vec![] } else { kids.get(k).into_iter().flatten().filter_map(|c| build(c, nodes, kids, reg, depth + 1)).collect() };
            ch.sort_by_key(|c| c["sequence"].as_i64().unwrap_or(10));
            n["children"] = J::Array(ch);
            let ok_model = n["model"].as_str().map_or(false, |m| reg.models.contains_key(m));
            if ok_model || !n["children"].as_array().map_or(true, |c| c.is_empty()) { Some(n) } else { None }
        }
        let mut roots: Vec<J> = roots_k.iter().filter_map(|k| build(k, &nodes, &kids, reg, 0)).collect();
        roots.sort_by_key(|c| c["sequence"].as_i64().unwrap_or(10));
        J::Array(roots)
    }
}

/// Build the ACL table from `ir.model.access.csv` data of the installed modules.
pub fn access_table(reg: &Registry, dir: &Path, modules: &[String]) -> AccessTable {
    let by_table: BTreeMap<String, String> = reg.models.keys().map(|n| (table_of(n), n.clone())).collect();
    let mut t = AccessTable::default();
    for m in modules {
        let Some(d) = read_module(dir, m) else { continue };
        for a in d["access"].as_array().into_iter().flatten() {
            let Some(x) = a["model_xmlid"].as_str() else { continue };
            let tail = x.rsplit('.').next().unwrap_or(x).trim_start_matches("model_");
            let Some(model) = by_table.get(tail) else { continue };
            t.entries.push(AccessEntry { model: model.clone(), group: a["group"].as_str().map(String::from), read: a["read"].as_bool().unwrap_or(false), write: a["write"].as_bool().unwrap_or(false), create: a["create"].as_bool().unwrap_or(false), unlink: a["unlink"].as_bool().unwrap_or(false) });
        }
    }
    t
}

#[derive(Default, Debug, serde::Serialize)]
pub struct LoadReport { pub loaded: usize, pub skipped: usize, pub failed: BTreeMap<String, usize>, pub first_errors: Vec<String> }

const SKIP: &[&str] = &["ir.actions.act_window", "ir.actions.server", "ir.actions.client", "ir.actions.act_url", "ir.actions.act_window.view", "ir.ui.menu", "ir.rule", "ir.cron", "ir.config_parameter", "mail.template", "ir.model.access", "ir.filters", "ir.actions.todo", "ir.mail_server", "website.page", "website.menu", "ir.asset", "ir.model.fields", "ir.model", "base.automation", "ir.default", "digest.tip"];

struct Ctx<'a> { env: &'a Env<'a>, ids: BTreeMap<String, (String, i64)> }

fn resolve(cx: &Ctx, j: &J, module: &str) -> Option<Value> {
    match j {
        J::Object(o) if o.contains_key("$ref") => { let r = o["$ref"].as_str()?; let key = if r.contains('.') { r.to_string() } else { format!("{module}.{r}") }; cx.ids.get(&key).map(|(_, i)| Value::Int(*i)) }
        J::Object(o) if o.contains_key("$unevaluated") => None,
        J::Array(a) => { let v: Option<Vec<Value>> = a.iter().map(|x| resolve(cx, x, module)).collect(); v.map(Value::List) }
        // nested values inside create-commands, e.g. (0, 0, {'attribute_id': ref(...)}); any unresolved ref invalidates the whole value
        J::Object(o) => { let m: Option<std::collections::BTreeMap<String, Value>> = o.iter().map(|(k, x)| resolve(cx, x, module).map(|v| (k.clone(), v))).collect(); m.map(Value::Map) }
        other => Some(Value::from_json(other)),
    }
}

fn coerce(f: &odoo_core::schema::FieldDef, v: Value) -> Option<Value> {
    Some(match (f.ty.as_str(), v) {
        ("Integer" | "Many2one", Value::Text(s)) => Value::Int(s.trim().parse().ok()?),
        ("Float" | "Monetary", Value::Text(s)) => Value::Float(s.trim().parse().ok()?),
        ("Boolean", Value::Text(s)) => Value::Bool(matches!(s.trim().to_lowercase().as_str(), "1" | "true" | "yes")),
        ("Boolean", Value::Int(i)) => Value::Bool(i != 0),
        ("Many2one", Value::Text(_)) => return None,
        ("Many2many" | "One2many", Value::List(l)) if l.iter().all(|x| matches!(x, Value::Int(_))) => Value::List(l),
        (_, v) => v,
    })
}

/// Load `<record>` seed data for models present in the registry. Errors are collected, never fatal.
pub fn seed(env: &Env, dir: &Path, modules: &[String]) -> Result<LoadReport> {
    let mut cx = Ctx { env, ids: BTreeMap::new() };
    let mut rep = LoadReport::default();
    let e = env.sudo();
    // records of modules installed earlier stay referenceable (runtime install seeds only the new modules)
    if env.reg.models.contains_key("ir.model.data") {
        for r in env.conn.query("SELECT module, name, model, res_id FROM ir_model_data", &[]).unwrap_or_default() {
            if let (Some(m), Some(n), Some(md), Some(id)) = (r["module"].as_str(), r["name"].as_str(), r["model"].as_str(), r["res_id"].as_i64()) { cx.ids.insert(format!("{m}.{n}"), (md.to_string(), id)); }
        }
    }
    for m in modules {
        let Some(d) = read_module(dir, m) else { continue };
        for r in d["records"].as_array().into_iter().flatten() {
            let model = r["model"].as_str().unwrap_or("");
            if SKIP.contains(&model) || !cx.env.reg.models.contains_key(model) || cx.env.reg.models[model].abstract_ { rep.skipped += 1; continue; }
            let md = &cx.env.reg.models[model];
            let mut vals = Row::new(); let mut bad = false;
            for (k, v) in r["values"].as_object().into_iter().flatten() {
                let Some(f) = md.fields.get(k) else { continue };
                let editable = f.readonly.as_ref().and_then(|r| r.as_bool()) == Some(false);
                if f.is_readonly() && !f.is_stored() && !editable { continue; }
                match resolve(&cx, v, m).and_then(|x| coerce(f, x)) { Some(x) => { vals.insert(k.clone(), x); } None => { if f.is_required() { bad = true; } } }
            }
            let key = r["id"].as_str().map(|x| if x.contains('.') { x.to_string() } else { format!("{m}.{x}") });
            // re-declaration of a known xmlid is an update (Odoo semantics), not a second record
            if let Some((km, kid)) = key.as_ref().and_then(|k| cx.ids.get(k)).cloned() {
                if km == model { match odoo_core::store::nested(env.conn, || orm::write(&e, model, &[kid], vals)) { Ok(_) => rep.loaded += 1, Err(err) => { *rep.failed.entry(model.to_string()).or_default() += 1; if rep.first_errors.len() < 25 { rep.first_errors.push(format!("{m}:{} update {model}: {err}", r["id"].as_str().unwrap_or("?"))); } } } continue; }
            }
            if bad { rep.skipped += 1; continue; }
            match odoo_core::store::nested(env.conn, || orm::create(&e, model, vals)) {
                Ok(id) => { rep.loaded += 1; if let Some(k) = key { if env.reg.models.contains_key("ir.model.data") { if let Some((km, kn)) = k.split_once('.') { let _ = orm::create(&e, "ir.model.data", row(&[("module", km.into()), ("name", kn.into()), ("model", model.into()), ("res_id", id.into())])); } } cx.ids.insert(k, (model.to_string(), id)); } }
                Err(err) => { *rep.failed.entry(model.to_string()).or_default() += 1; if rep.first_errors.len() < 25 { rep.first_errors.push(format!("{m}:{} {model}: {err}", r["id"].as_str().unwrap_or("?"))); } }
            }
        }
    }
    let _ = (text, &cx.env.uid);
    Ok(rep)
}
