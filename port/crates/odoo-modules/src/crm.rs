//! crm: leads/opportunities, stage pipeline, won/lost.
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Rules, Value};

pub fn rules() -> Rules {
    Rules::default()
        .before_create("crm.lead", |env, mut v| {
            v.entry("type".into()).or_insert("lead".into());
            v.entry("active".into()).or_insert(true.into());
            if !v.contains_key("stage_id") { if let Some(s) = orm::search(&env.sudo(), "crm.stage", &Domain::True, Some("sequence, id"), Some(1), 0)?.first() { v.insert("stage_id".into(), (*s).into()); } }
            Ok(v)
        })
        .action("crm.lead", "action_set_won", |env, ids, _| {
            let e = env.sudo();
            let won = find_one(&e, "crm.stage", term("is_won", "=", true))?.ok_or_else(|| OdooError::User("no 'won' stage configured".into()))?;
            orm::write(&e, "crm.lead", ids, row(&[("stage_id", won.into()), ("probability", 100.0.into()), ("date_closed", orm::now().into())]))?; Ok(Value::Bool(true))
        })
        .action("crm.lead", "action_set_lost", |env, ids, args| {
            let mut v = row(&[("active", false.into()), ("probability", 0.0.into()), ("date_closed", orm::now().into())]);
            if let Some(r) = args.get("lost_reason_id") { v.insert("lost_reason_id".into(), r.clone()); }
            orm::write(&env.sudo(), "crm.lead", ids, v)?; Ok(Value::Bool(true))
        })
        .action("crm.lead", "convert_opportunity", |env, ids, _| { orm::write(&env.sudo(), "crm.lead", ids, row(&[("type", "opportunity".into())]))?; Ok(Value::Bool(true)) })
}
