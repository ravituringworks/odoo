//! project: tasks with stages and close/reopen transitions.
use crate::util::*;
use odoo_core::orm::{self};
use odoo_core::{Domain, Rules, Value};

pub fn rules() -> Rules {
    Rules::default()
        .before_create("project.task", |env, mut v| {
            if !v.contains_key("stage_id") {
                if let Some(s) = orm::search(&env.sudo(), "project.task.type", &Domain::True, Some("sequence, id"), Some(1), 0)?.first() { v.insert("stage_id".into(), (*s).into()); }
            }
            v.entry("state".into()).or_insert("01_in_progress".into());
            Ok(v)
        })
        .action("project.task", "action_done", |env, ids, _| { orm::write(&env.sudo(), "project.task", ids, row(&[("state", "1_done".into())]))?; Ok(Value::Bool(true)) })
        .action("project.task", "action_cancel", |env, ids, _| { orm::write(&env.sudo(), "project.task", ids, row(&[("state", "1_canceled".into())]))?; Ok(Value::Bool(true)) })
        .action("project.task", "action_reopen", |env, ids, _| { orm::write(&env.sudo(), "project.task", ids, row(&[("state", "01_in_progress".into())]))?; Ok(Value::Bool(true)) })
}
