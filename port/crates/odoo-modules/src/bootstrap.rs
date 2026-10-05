//! First-run seed: the minimum ambient records Odoo's `env.company/env.user` defaults rely on.
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, Result, Value};

pub fn run(env: &Env) -> Result<()> {
    let e = env.sudo();
    let has = |m: &str| e.reg.models.contains_key(m);
    if has("res.currency") { find_or_create(&e, "res.currency", term("name", "=", "USD"), row(&[("name", "USD".into()), ("symbol", "$".into()), ("rounding", 0.01.into())]))?; }
    if has("res.company") {
        let cur = find_one(&e, "res.currency", Domain::True)?;
        let mut v = row(&[("name", "My Company".into())]);
        if let Some(c) = cur { v.insert("currency_id".into(), c.into()); }
        find_or_create(&e, "res.company", Domain::True, v)?;
    }
    if has("res.users") && find_one(&e, "res.users", Domain::True)?.is_none() {
        orm::create(&e, "res.users", row(&[("name", "Administrator".into()), ("login", "admin".into())]))?;
    }
    if has("uom.uom") {
        let mut v = row(&[("name", "Units".into())]);
        if has("uom.category") { v.insert("category_id".into(), find_or_create(&e, "uom.category", Domain::True, row(&[("name", "Unit".into())]))?.into()); }
        find_or_create(&e, "uom.uom", Domain::True, v)?;
    }
    if has("product.category") { find_or_create(&e, "product.category", Domain::True, row(&[("name", "All".into())]))?; }
    let _ = Value::Null;
    Ok(())
}
