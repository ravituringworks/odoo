//! base: user provisioning defaults.
use crate::util::*;
use odoo_core::orm::{self};
use odoo_core::{Rules, Value};

/// res.partner.complete_name: "Parent, Name" for contacts under a company, else just the name (Odoo `_compute_complete_name`).
fn complete_name(env: &odoo_core::orm::Env, r: &odoo_core::Row) -> odoo_core::Result<Value> {
    let name = text(r, "name").unwrap_or_default();
    let parent = match id_of(r, "parent_id") { Some(p) => text(&rec(env, "res.partner", p)?, "name"), None => None };
    Ok(Value::Text(match (parent, name.is_empty()) { (Some(p), false) => format!("{p}, {name}"), (Some(p), true) => p, (None, _) => name }))
}

/// res.country: Odoo's create()/write() store ISO codes in upper case (the seed data ships them lower case).
fn upper_code(_: &odoo_core::Env, mut v: odoo_core::Row) -> odoo_core::Result<odoo_core::Row> {
    if let Some(Value::Text(c)) = v.get_mut("code") { *c = c.to_uppercase(); }
    Ok(v)
}
pub fn country_rules() -> Rules { Rules::default().before_create("res.country", upper_code).before_write("res.country", upper_code) }

pub fn partner_rules() -> Rules { Rules::default().compute("res.partner", "complete_name", complete_name) }

pub fn rules() -> Rules {
    Rules::default()
        // mail.canned.response: a new response is editable by whoever is creating it; saved ones by their author and the administrator
        .default_for("mail.canned.response", "is_editable", |_| Ok(Value::Bool(true)))
        .compute("mail.canned.response", "is_editable", |env, r| Ok(Value::Bool(env.uid == 1 || id_of(r, "create_uid").map_or(true, |u| u == env.uid))))
        .after_create("res.users", |env, ids, vals| {
        // new users land in "Internal User" unless groups were given explicitly
        if vals.contains_key("groups_id") || !env.reg.models.contains_key("ir.model.data") { return Ok(()); }
        let e = env.sudo();
        let Some(row_id) = find_one(&e, "ir.model.data", term("module", "=", "base").and(term("name", "=", "group_user")))? else { return Ok(()) };
        let gid = id_of(&rec(&e, "ir.model.data", row_id)?, "res_id");
        if let Some(g) = gid { orm::write(&e, "res.users", ids, row(&[("groups_id", Value::List(vec![Value::List(vec![4.into(), g.into()])]))]))?; }
        Ok(())
    })
}
