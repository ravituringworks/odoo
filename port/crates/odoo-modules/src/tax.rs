//! Pure tax computation (account.tax subset: percent / fixed / division, with optional price-included).
use crate::util::*;
use odoo_core::orm::Env;
use odoo_core::Result;

#[derive(Clone, Debug)]
pub struct Tax { pub amount: f64, pub kind: String, pub price_include: bool }

pub fn load(env: &Env, ids: &[i64]) -> Result<Vec<Tax>> {
    ids.iter().map(|i| { let r = rec(env, "account.tax", *i)?; Ok(Tax { amount: num(&r, "amount"), kind: text(&r, "amount_type").unwrap_or_else(|| "percent".into()), price_include: r.get("price_include").map_or(false, |v| v.truthy()) }) }).collect()
}

/// (untaxed, tax, total) for qty*price*(1-disc) under `taxes`, in sequence order.
pub fn compute(qty: f64, price: f64, discount: f64, taxes: &[Tax]) -> (f64, f64, f64) {
    let gross = qty * price * (1.0 - discount / 100.0);
    let (fixed_inc, pct_inc): (f64, f64) = taxes.iter().filter(|t| t.price_include).fold((0.0, 0.0), |(f, p), t| match t.kind.as_str() { "fixed" => (f + t.amount * qty, p), "percent" => (f, p + t.amount), _ => (f, p) });
    let base = if fixed_inc != 0.0 || pct_inc != 0.0 { (gross - fixed_inc) / (1.0 + pct_inc / 100.0) } else { gross };
    let tax: f64 = taxes.iter().map(|t| match t.kind.as_str() { "fixed" => t.amount * qty, "division" => base / (1.0 - t.amount / 100.0) - base, _ => base * t.amount / 100.0 }).sum();
    (r2(base), r2(tax), r2(base + tax))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn percent() { let t = vec![Tax { amount: 15.0, kind: "percent".into(), price_include: false }]; assert_eq!(compute(2.0, 100.0, 10.0, &t), (180.0, 27.0, 207.0)); }
    #[test] fn included() { let t = vec![Tax { amount: 20.0, kind: "percent".into(), price_include: true }]; assert_eq!(compute(1.0, 120.0, 0.0, &t), (100.0, 20.0, 120.0)); }
    #[test] fn fixed() { let t = vec![Tax { amount: 5.0, kind: "fixed".into(), price_include: false }]; assert_eq!(compute(3.0, 10.0, 0.0, &t), (30.0, 15.0, 45.0)); }
}
