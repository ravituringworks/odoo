//! Declarative model registry, built by folding module schemas (extracted from Odoo)
//! in dependency order. Pure data + pure functions; no I/O besides `load_dir`.
use crate::error::{OdooError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as J;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FieldDef {
    #[serde(rename = "type")] pub ty: String,
    #[serde(default)] pub string: Option<J>,
    #[serde(default)] pub required: Option<J>,
    #[serde(default)] pub readonly: Option<J>,
    #[serde(default)] pub comodel: Option<J>,
    #[serde(default)] pub inverse_name: Option<J>,
    #[serde(default)] pub selection: Option<J>,
    #[serde(default)] pub selection_add: Option<J>,
    #[serde(default)] pub selection_method: Option<J>,
    #[serde(default)] pub default: Option<J>,
    #[serde(default)] pub compute: Option<J>,
    #[serde(default)] pub related: Option<J>,
    #[serde(default)] pub store: Option<J>,
    #[serde(default)] pub index: Option<J>,
    #[serde(default)] pub ondelete: Option<J>,
    #[serde(default)] pub relation_table: Option<J>,
    #[serde(default)] pub column1: Option<J>,
    #[serde(default)] pub column2: Option<J>,
    #[serde(default)] pub domain: Option<J>,
    #[serde(default)] pub currency_field: Option<J>,
    #[serde(default)] pub size: Option<J>,
    #[serde(default)] pub copy: Option<J>,
}

fn s(o: &Option<J>) -> Option<String> { o.as_ref().and_then(|v| v.as_str().map(String::from)) }
fn b(o: &Option<J>) -> Option<bool> { o.as_ref().and_then(|v| v.as_bool()) }

impl FieldDef {
    pub fn label(&self, name: &str) -> String {
        s(&self.string).unwrap_or_else(|| name.strip_suffix("_ids").or_else(|| name.strip_suffix("_id")).unwrap_or(name).replace('_', " ").split(' ').map(|w| { let mut c = w.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() }).collect::<Vec<_>>().join(" "))
    }
    pub fn comodel_name(&self) -> Option<String> { s(&self.comodel) }
    pub fn inverse(&self) -> Option<String> { s(&self.inverse_name) }
    pub fn is_required(&self) -> bool { b(&self.required).unwrap_or(false) }
    /// Odoo: computed and related fields are readonly unless explicitly `readonly=False`.
    pub fn is_readonly(&self) -> bool { b(&self.readonly).unwrap_or(self.is_computed()) }
    /// Odoo `copy` default: True except one2many, computed (non-stored-writable) and `id`; explicit `copy=` wins.
    pub fn is_copyable(&self) -> bool { b(&self.copy).unwrap_or(self.ty != "One2many" && !self.is_computed()) }
    pub fn is_x2many(&self) -> bool { matches!(self.ty.as_str(), "One2many" | "Many2many") }
    pub fn is_computed(&self) -> bool { self.compute.is_some() || self.related.is_some() }
    fn is_computed_nostore(&self) -> bool { self.is_computed() && !self.is_stored() }
    /// Odoo rule: computed/related fields are stored only if `store=True`; x2many never have columns.
    pub fn is_stored(&self) -> bool {
        match self.ty.as_str() { "One2many" | "Many2many" | "Id" | "PropertiesDefinition" => return false, _ => {} }
        if self.is_computed() { b(&self.store).unwrap_or(false) } else { b(&self.store).unwrap_or(true) }
    }
    pub fn selection_values(&self) -> Vec<(String, String)> {
        match &self.selection {
            Some(J::Array(a)) => a.iter().filter_map(|p| { let p = p.as_array()?; Some((p.first()?.as_str()?.to_string(), p.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string())) }).collect(),
            _ => vec![],
        }
    }
    /// Static default (literals only; lambdas are ported as Rust `defaults` rules).
    pub fn static_default(&self) -> Option<J> {
        match &self.default { Some(v) if !v.is_object() => Some(v.clone()), _ => None }
    }
    pub fn relation_table_name(&self, model: &str, name: &str, registry: &Registry) -> Option<(String, String, String)> {
        if self.ty != "Many2many" { return None; }
        let co = self.comodel_name()?;
        let t = s(&self.relation_table).unwrap_or_else(|| {
            let (a, b) = (registry.table(model), registry.table(&co));
            let mut p = [a, b]; p.sort(); format!("{}_{}_rel", p[0], p[1])
        });
        let _ = name;
        let c1 = s(&self.column1).unwrap_or_else(|| format!("{}_id", registry.table(model)));
        let c2 = s(&self.column2).unwrap_or_else(|| if registry.table(&co) == registry.table(model) { format!("{}_id2", registry.table(&co)) } else { format!("{}_id", registry.table(&co)) });
        Some((t, c1, c2))
    }
}

impl FieldDef {
    /// Overlay a later redefinition onto `self` (Odoo semantics: only specified attributes change).
    pub fn overlay(mut self, n: FieldDef) -> FieldDef {
        macro_rules! o { ($($f:ident),*) => { $( if n.$f.is_some() { self.$f = n.$f; } )* } }
        o!(selection_method, string, required, readonly, comodel, inverse_name, selection, default, compute, related, store, index, ondelete, relation_table, column1, column2, domain, currency_field, size, copy);
        // selection_add: append new options, overriding labels of existing keys (Odoo semantics)
        if let Some(J::Array(add)) = n.selection_add {
            let mut base: Vec<J> = match self.selection.take() { Some(J::Array(a)) => a, _ => vec![] };
            for item in add {
                let key = item.get(0).cloned();
                if let Some(pos) = base.iter().position(|b| b.get(0) == key.as_ref()) { base[pos] = item; } else { base.push(item); }
            }
            self.selection = Some(J::Array(base));
        }
        self.ty = n.ty; self
    }
}

pub fn table_of(model: &str) -> String { model.replace('.', "_") }

impl Registry {
    /// Physical table for a model, honouring `_table` overrides (falls back to the dotted-name convention).
    pub fn table(&self, model: &str) -> String { self.models.get(model).map(|m| m.table()).unwrap_or_else(|| table_of(model)) }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MethodDef { pub name: String, #[serde(default)] pub decorators: Vec<String>, #[serde(default)] pub lines: u32, #[serde(default)] pub action: bool }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModelDef {
    pub name: String,
    #[serde(default)] pub inherit: Vec<String>,
    #[serde(default)] pub inherits: BTreeMap<String, String>,
    #[serde(default)] pub description: Option<String>,
    #[serde(default)] pub order: Option<String>,
    #[serde(default)] pub rec_name: Option<String>,
    #[serde(default)] pub table: Option<String>,
    #[serde(default)] pub parent_name: Option<String>,
    #[serde(default)] pub abstract_: bool,
    #[serde(default)] pub transient: bool,
    #[serde(default)] pub fields: BTreeMap<String, FieldDef>,
    #[serde(default)] pub methods: Vec<MethodDef>,
    #[serde(default)] pub module: String,
    /// Statically evaluated selection methods (`_get_payment_terminal_selection`): items contributed by each installed module.
    #[serde(default)] pub sel_methods: BTreeMap<String, Vec<J>>,
}

impl ModelDef {
    pub fn table(&self) -> String { self.table.clone().unwrap_or_else(|| table_of(&self.name)) }
    pub fn has_table(&self) -> bool { !self.abstract_ }
    pub fn rec_name(&self) -> &str { self.rec_name.as_deref().unwrap_or("name") }
    pub fn display_field(&self) -> Option<&str> {
        let r = self.rec_name();
        if self.fields.contains_key(r) { Some(r) } else if self.fields.contains_key("name") { Some("name") } else { None }
    }
    pub fn stored_fields(&self) -> impl Iterator<Item = (&String, &FieldDef)> { self.fields.iter().filter(|(_, f)| f.is_stored()) }
}

#[derive(Debug, Deserialize)]
struct ModuleFile {
    module: String,
    #[serde(default)] depends: Vec<String>,
    #[serde(default)] models: BTreeMap<String, RawModel>,
}
#[derive(Debug, Deserialize, Default)]
struct RawModel {
    #[serde(default)] inherit: Vec<String>, #[serde(default)] inherits: BTreeMap<String, String>,
    #[serde(default)] description: Option<String>, #[serde(default)] order: Option<String>,
    #[serde(default)] rec_name: Option<String>, #[serde(default)] table: Option<String>,
    #[serde(default)] parent_name: Option<String>,
    #[serde(default)] r#abstract: bool, #[serde(default)] transient: bool,
    #[serde(default)] fields: BTreeMap<String, FieldDef>, #[serde(default)] methods: Vec<MethodDef>,
    #[serde(default)] name: String,
    #[serde(default)] sel_methods: BTreeMap<String, Vec<J>>,
}

#[derive(Clone, Debug, Default)]
pub struct Registry {
    pub models: BTreeMap<String, ModelDef>,
    pub modules: Vec<String>,
}

impl Registry {
    pub fn model(&self, name: &str) -> Result<&ModelDef> { self.models.get(name).ok_or_else(|| OdooError::UnknownModel(name.into())) }
    pub fn field(&self, model: &str, f: &str) -> Result<&FieldDef> {
        self.model(model)?.fields.get(f).ok_or_else(|| OdooError::UnknownField(model.into(), f.into()))
    }

    /// Fold a module's declarations into the registry (pure: returns a new registry).
    fn with_module(mut self, raw: ModuleFileView) -> Self {
        for (name, m) in raw.models {
            let e = self.models.entry(name.clone()).or_insert_with(|| ModelDef { name: name.clone(), module: raw.module.clone(), ..Default::default() });
            for (k, f) in m.fields { let merged = match e.fields.remove(&k) { Some(old) => old.overlay(f), None => f }; e.fields.insert(k, merged); }
            e.methods.extend(m.methods);
            for i in m.inherit { if !e.inherit.contains(&i) && i != name { e.inherit.push(i); } }
            e.inherits.extend(m.inherits);
            e.description = m.description.or(e.description.take());
            e.order = m.order.or(e.order.take());
            e.rec_name = m.rec_name.or(e.rec_name.take());
            e.table = m.table.or(e.table.take());
            e.parent_name = m.parent_name.or(e.parent_name.take());
            e.abstract_ |= m.abstract_; e.transient |= m.transient;
            for (k, items) in m.sel_methods { let slot = e.sel_methods.entry(k).or_default(); for it in items { if !slot.iter().any(|x| x.get(0) == it.get(0)) { slot.push(it); } } }
        }
        self.modules.push(raw.module);
        self
    }

    /// Resolve `_inherit` mixins (copy missing fields) and `_inherits` delegation
    /// (parent fields exposed as related, FK field ensured). Fixed-point, cycle-safe.
    fn resolve_inheritance(mut self) -> Self {
        let names: Vec<String> = self.models.keys().cloned().collect();
        for n in &names {
            let mut seen = BTreeSet::new();
            let mut extra: BTreeMap<String, FieldDef> = BTreeMap::new();
            collect_mixin_fields(&self.models, n, &mut seen, &mut extra);
            let m = self.models.get_mut(n).unwrap();
            // a model may re-declare a mixin field with only extra keywords (`campaign_id = Many2one(ondelete=..)`):
            // the mixin definition is the base, the model's own attributes overlay it
            for (k, v) in extra { match m.fields.remove(&k) { Some(own) => { m.fields.insert(k, v.overlay(own)); } None => { m.fields.insert(k, v); } } }
        }
        self.resolve_related_comodels();
        self.resolve_selection_methods();
        // implicit magic fields on every concrete model (searchable/readable like any other)
        for m in self.models.values_mut().filter(|m| !m.abstract_) {
            for (n, ty, co) in [("create_uid", "Many2one", Some("res.users")), ("write_uid", "Many2one", Some("res.users")), ("create_date", "Datetime", None), ("write_date", "Datetime", None)] {
                m.fields.entry(n.to_string()).or_insert_with(|| FieldDef { ty: ty.into(), readonly: Some(J::Bool(true)), comodel: co.map(|c| J::String(c.into())), ..Default::default() });
            }
        }
        for n in &names {
            let delegates: Vec<(String, String)> = self.models[n].inherits.iter().map(|(a, b)| (a.clone(), b.clone())).collect();
            for (parent, fk) in delegates {
                let pf = self.models.get(&parent).map(|p| p.fields.clone()).unwrap_or_default();
                let m = self.models.get_mut(n).unwrap();
                m.fields.entry(fk.clone()).or_insert_with(|| FieldDef { ty: "Many2one".into(), comodel: Some(J::String(parent.clone())), required: Some(J::Bool(true)), ondelete: Some(J::String("cascade".into())), ..Default::default() });
                for (k, f) in pf {
                    if k == "id" || f.is_x2many() { continue; }
                    m.fields.entry(k.clone()).or_insert_with(|| FieldDef { related: Some(J::String(format!("{fk}.{k}"))), store: Some(J::Bool(false)), readonly: Some(J::Bool(false)), ..f.clone() });
                }
            }
        }
        self
    }

    /// Fields with `selection='_get_x'` / `lambda self: self._get_x()`: options are the union of what every *installed*
    /// module's `_get_x` contributes (looked up on the model and its `_inherit` parents).
    fn resolve_selection_methods(&mut self) {
        fn lookup(all: &BTreeMap<String, ModelDef>, model: &str, method: &str, seen: &mut BTreeSet<String>) -> Vec<J> {
            if !seen.insert(model.to_string()) { return vec![]; }
            let Some(m) = all.get(model) else { return vec![] };
            let mut out: Vec<J> = m.sel_methods.get(method).cloned().unwrap_or_default();
            for p in &m.inherit { for it in lookup(all, p, method, seen) { if !out.iter().any(|x| x.get(0) == it.get(0)) { out.push(it); } } }
            out
        }
        let jobs: Vec<(String, String, String)> = self.models.iter().flat_map(|(mn, m)| m.fields.iter().filter(|(_, f)| f.ty == "Selection" && !matches!(f.selection, Some(J::Array(_)))).filter_map(|(fname, f)| f.selection_method.as_ref().and_then(|s| s.as_str()).map(|s| (mn.clone(), fname.clone(), s.to_string())))).collect();
        for (mn, fname, method) in jobs {
            let items = lookup(&self.models, &mn, &method, &mut BTreeSet::new());
            if let Some(f) = self.models.get_mut(&mn).and_then(|m| m.fields.get_mut(&fname)) { f.selection = Some(J::Array(items)); }
        }
    }

    /// `Many2one(related='a.b')` and friends carry no comodel of their own: take it from the field the path ends at.
    fn resolve_related_comodels(&mut self) {
        for _ in 0..3 {
            let mut fixes: Vec<(String, String, J)> = vec![];
            for (mn, m) in &self.models {
                for (fname, f) in &m.fields {
                    if !matches!(f.ty.as_str(), "Many2one" | "One2many" | "Many2many") || f.comodel_name().is_some() { continue; }
                    let Some(path) = f.related.as_ref().and_then(|r| r.as_str()) else { continue };
                    let mut cur = mn.clone(); let mut found: Option<String> = None;
                    for (i, hop) in path.split('.').enumerate() {
                        let Some(tf) = self.models.get(&cur).and_then(|m| m.fields.get(hop)) else { found = None; break };
                        let co = tf.comodel_name();
                        if i + 1 == path.split('.').count() { found = co; } else { match co { Some(c) => cur = c, None => { found = None; break } } }
                    }
                    if let Some(c) = found { fixes.push((mn.clone(), fname.clone(), J::String(c))); }
                }
            }
            if fixes.is_empty() { break; }
            for (m, f, c) in fixes { if let Some(fd) = self.models.get_mut(&m).and_then(|m| m.fields.get_mut(&f)) { fd.comodel = Some(c); } }
        }
    }

    pub fn build(modules: Vec<(String, Vec<String>, BTreeMap<String, ModelDef>)>) -> Registry {
        let mut r = Registry::default();
        for (module, _deps, models) in modules { r = r.with_module(ModuleFileView { module, models }); }
        r.resolve_inheritance()
    }

    /// Load `schema/_manifest.json` order + module files, restricted to the dependency closure of `roots`.
    pub fn load_dir(dir: &std::path::Path, roots: &[&str]) -> Result<Registry> {
        let rd = |p: std::path::PathBuf| -> Result<String> { std::fs::read_to_string(&p).map_err(|e| OdooError::Storage(format!("{}: {e}", p.display()))) };
        let manifest: J = serde_json::from_str(&rd(dir.join("_manifest.json"))?).map_err(|e| OdooError::Storage(e.to_string()))?;
        let order: Vec<String> = manifest["order"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        let deps = |m: &str| -> Vec<String> { manifest["modules"][m]["depends"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default() };
        let mut wanted = BTreeSet::new();
        let mut stack: Vec<String> = roots.iter().map(|s| s.to_string()).collect();
        while let Some(m) = stack.pop() { if wanted.insert(m.clone()) { stack.extend(deps(&m)); } }
        let mut mods = vec![];
        for m in order.into_iter().filter(|m| wanted.contains(m)) {
            let f: ModuleFile = serde_json::from_str(&rd(dir.join(format!("{m}.json")))?).map_err(|e| OdooError::Storage(format!("{m}: {e}")))?;
            let models = f.models.into_iter().map(|(k, v)| (k.clone(), ModelDef { name: if v.name.is_empty() { k } else { v.name }, inherit: v.inherit, inherits: v.inherits, description: v.description, order: v.order, rec_name: v.rec_name, table: v.table, parent_name: v.parent_name, abstract_: v.r#abstract, transient: v.transient, fields: v.fields, methods: v.methods, module: f.module.clone(), sel_methods: v.sel_methods })).collect();
            mods.push((f.module, f.depends, models));
        }
        Ok(Registry::build(mods))
    }
}

struct ModuleFileView { module: String, models: BTreeMap<String, ModelDef> }

fn collect_mixin_fields(all: &BTreeMap<String, ModelDef>, name: &str, seen: &mut BTreeSet<String>, out: &mut BTreeMap<String, FieldDef>) {
    if !seen.insert(name.to_string()) { return; }
    let Some(m) = all.get(name) else { return };
    for p in &m.inherit {
        if let Some(pm) = all.get(p) { for (k, f) in &pm.fields { out.entry(k.clone()).or_insert_with(|| f.clone()); } }
        collect_mixin_fields(all, p, seen, out);
    }
}
