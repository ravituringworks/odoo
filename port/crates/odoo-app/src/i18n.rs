//! Server-side translation of metadata (field labels, selections, menus, view strings) from catalogs generated out of
//! Odoo's own .po files (`tools/extract_i18n.py`). Catalogs load lazily per language and are cached.
use serde::Deserialize;
use serde_json::Value as J;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Deserialize, Default)]
pub struct Catalog {
    #[serde(default)] pub fields: HashMap<String, String>,
    #[serde(default)] pub selection: HashMap<String, String>,
    #[serde(default)] pub menus: HashMap<String, String>,
    #[serde(default)] pub terms: HashMap<String, String>,
}

pub struct I18n { dir: PathBuf, cache: Mutex<HashMap<String, Option<Arc<Catalog>>>> }

impl I18n {
    pub fn dir(&self) -> &std::path::Path { &self.dir }
    pub fn new(dir: PathBuf) -> I18n { I18n { dir, cache: Mutex::new(HashMap::new()) } }

    /// Catalog for an Odoo language code (`fr`, `pt_BR`...); `None` for English/unknown (callers then return source strings).
    pub fn get(&self, lang: &str) -> Option<Arc<Catalog>> {
        if lang.is_empty() || lang.starts_with("en") || !lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { return None; }  // also blocks path tricks
        let mut c = self.cache.lock().ok()?;
        c.entry(lang.to_string()).or_insert_with(|| std::fs::read_to_string(self.dir.join(format!("{lang}.json"))).ok().and_then(|s| serde_json::from_str(&s).ok()).map(Arc::new)).clone()
    }
}

impl Catalog {
    fn term(&self, s: &str) -> Option<&String> { self.terms.get(s) }

    /// `fields_get` output: field label + selection labels, looking through `_inherit` parents for mixin fields.
    pub fn fields_get(&self, model: &str, parents: &[String], out: &mut J) {
        let Some(map) = out.as_object_mut() else { return };
        for (name, f) in map.iter_mut() {
            let label = std::iter::once(model).chain(parents.iter().map(|s| s.as_str())).find_map(|m| self.fields.get(&format!("{m}.{name}")))
                .or_else(|| f["string"].as_str().and_then(|s| self.term(s)));
            if let Some(l) = label { f["string"] = J::String(l.clone()); }
            if let Some(sel) = f["selection"].as_array_mut() {
                for pair in sel.iter_mut() {
                    let (Some(k), Some(en)) = (pair.get(0).and_then(|x| x.as_str()).map(String::from), pair.get(1).and_then(|x| x.as_str()).map(String::from)) else { continue };
                    let tr = std::iter::once(model).chain(parents.iter().map(|s| s.as_str())).find_map(|m| self.selection.get(&format!("{m}.{name}.{k}"))).or_else(|| self.term(&en));
                    if let Some(t) = tr { pair[1] = J::String(t.clone()); }
                }
            }
        }
    }

    pub fn menus(&self, tree: &mut J) {
        match tree {
            J::Array(a) => a.iter_mut().for_each(|n| self.menus(n)),
            J::Object(o) => {
                let tr = o.get("id").and_then(|i| i.as_str()).and_then(|i| self.menus.get(i)).or_else(|| o.get("name").and_then(|n| n.as_str()).and_then(|n| self.term(n))).cloned();
                if let Some(t) = tr { o.insert("name".into(), J::String(t)); }
                if let Some(c) = o.get_mut("children") { self.menus(c); }
            }
            _ => {}
        }
    }

    /// Translate user-visible strings of a merged view arch in place.
    pub fn view(&self, node: &mut J) {
        let Some(o) = node.as_object_mut() else { return };
        if let Some(attrs) = o.get_mut("attrs").and_then(|a| a.as_object_mut()) {
            for k in ["string", "placeholder", "help", "title", "confirm"] { if let Some(v) = attrs.get(k).and_then(|v| v.as_str()).and_then(|s| self.term(s)).cloned() { attrs.insert(k.into(), J::String(v)); } }
        }
        if let Some(t) = o.get("text").and_then(|t| t.as_str()).and_then(|s| self.term(s)).cloned() { o.insert("text".into(), J::String(t)); }
        if let Some(ch) = o.get_mut("children").and_then(|c| c.as_array_mut()) { ch.iter_mut().for_each(|c| self.view(c)); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn cat() -> Catalog { serde_json::from_value(json!({"fields": {"sale.order.partner_id": "Client", "mail.thread.message_ids": "Messages"}, "selection": {"sale.order.state.sale": "Commande"}, "menus": {"sale.menu_root": "Ventes"}, "terms": {"Confirm": "Confirmer", "Quotation": "Devis"}})).unwrap() }
    #[test] fn translates_fields_selection_menus_views() {
        let c = cat();
        let mut f = json!({"partner_id": {"string": "Customer"}, "state": {"string": "Status", "selection": [["draft", "Quotation"], ["sale", "Sales Order"]]}, "message_ids": {"string": "Messages EN"}});
        c.fields_get("sale.order", &["mail.thread".into()], &mut f);
        assert_eq!(f["partner_id"]["string"], "Client");
        assert_eq!(f["state"]["selection"][0][1], "Devis");         // via generic term
        assert_eq!(f["state"]["selection"][1][1], "Commande");      // via exact selection key
        assert_eq!(f["message_ids"]["string"], "Messages");         // mixin parent lookup
        let mut m = json!([{"id": "sale.menu_root", "name": "Sales", "children": [{"id": "x.y", "name": "Confirm", "children": []}]}]);
        c.menus(&mut m); assert_eq!(m[0]["name"], "Ventes"); assert_eq!(m[0]["children"][0]["name"], "Confirmer");
        let mut v = json!({"tag": "button", "attrs": {"string": "Confirm", "name": "action_confirm"}, "text": null, "children": [{"tag": "span", "attrs": {}, "text": "Quotation", "children": []}]});
        c.view(&mut v); assert_eq!(v["attrs"]["string"], "Confirmer"); assert_eq!(v["children"][0]["text"], "Devis"); assert_eq!(v["attrs"]["name"], "action_confirm");
    }
    #[test] fn english_and_hostile_codes_get_no_catalog() { let i = I18n::new(PathBuf::from("/nonexistent")); assert!(i.get("en_US").is_none()); assert!(i.get("../../etc/passwd").is_none()); assert!(i.get("fr").is_none()); }
}
