//! hr / hr_holidays: leave requests with approval workflow and day counting.
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{OdooError, Result, Row, Rules, Value};

/// Inclusive calendar days between two ISO dates (Mon–Fri only, matching the default 5-day calendar).
pub fn working_days(from: &str, to: &str) -> Option<f64> {
    fn jd(s: &str) -> Option<i64> {
        let p: Vec<i64> = s.get(..10)?.split('-').filter_map(|x| x.parse().ok()).collect();
        if p.len() != 3 { return None; }
        let (y, m, d) = (p[0], p[1], p[2]);
        let a = (14 - m) / 12; let yy = y + 4800 - a; let mm = m + 12 * a - 3;
        Some(d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045)
    }
    let (a, b) = (jd(from)?, jd(to)?);
    if b < a { return None; }
    Some((a..=b).filter(|j| (j + 1) % 7 < 5).count() as f64)  // JDN % 7: 0 = Monday
}

fn set_state(env: &Env, ids: &[i64], from: &[&str], to: &str) -> Result<Value> {
    let e = env.sudo();
    for id in ids {
        let r = rec(&e, "hr.leave", *id)?;
        let st = text(&r, "state").unwrap_or_default();
        if !from.contains(&st.as_str()) { return Err(OdooError::User(format!("Leave cannot move from '{st}' to '{to}'"))); }
        orm::write(&e, "hr.leave", &[*id], row(&[("state", to.into())]))?;
    }
    Ok(Value::Bool(true))
}

pub fn rules() -> Rules {
    Rules::default()
        .before_create("hr.leave", |_, mut v| {
            v.entry("state".into()).or_insert("confirm".into());
            if let (Some(a), Some(b)) = (text(&v, "request_date_from"), text(&v, "request_date_to")) {
                let d = working_days(&a, &b).ok_or_else(|| OdooError::Validation("The end date must be on or after the start date".into()))?;
                v.entry("number_of_days".into()).or_insert(d.into());
            }
            Ok(v)
        })
        .action("hr.leave", "action_approve", |env, ids, _| set_state(env, ids, &["confirm", "validate1"], "validate"))
        .action("hr.leave", "action_refuse", |env, ids, _| set_state(env, ids, &["confirm", "validate1", "validate"], "refuse"))
        .action("hr.leave", "action_draft", |env, ids, _| set_state(env, ids, &["refuse", "cancel"], "confirm"))
        .action("hr.leave", "action_cancel", |env, ids, _| set_state(env, ids, &["confirm", "validate1", "validate"], "cancel"))
}

#[allow(dead_code)]
fn _unused(_: Row) {}

#[cfg(test)]
mod tests {
    use super::working_days;
    #[test] fn week() { assert_eq!(working_days("2026-10-05", "2026-10-11"), Some(5.0)); }  // Mon..Sun
    #[test] fn single_saturday() { assert_eq!(working_days("2026-10-10", "2026-10-10"), Some(0.0)); }
    #[test] fn reversed() { assert_eq!(working_days("2026-10-11", "2026-10-05"), None); }
}
