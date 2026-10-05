#![allow(dead_code)]
//! Shared fixtures for the point_of_sale test files.
use odoo_core::{ddl, orm::{self, AllowAll, Env}, store::{self, Store}, Registry, Row, Value};
use odoo_modules::{account, stock, util::*};
use odoo_sqlite::SqliteStore;
use std::path::Path;

pub fn setup(mods: &[&str]) -> (Registry, SqliteStore) {
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), mods).unwrap();
    let st = SqliteStore::open(":memory:").unwrap();
    let plan = ddl::install_plan(&reg, st.dialect());
    store::run(&st, |c| { for s in &plan { c.execute(s, &[])?; } Ok(()) }).unwrap();
    (reg, st)
}
pub fn call(env: &Env, model: &str, method: &str, ids: &[i64], kw: Row) -> odoo_core::Result<Value> { orm::call(env, model, method, ids, &kw) }
pub fn mk(env: &Env, model: &str, v: &[(&str, Value)]) -> i64 { orm::create(env, model, row(v)).unwrap_or_else(|e| panic!("create {model}: {e}")) }
/// Run `f` in a savepoint that is rolled back, expecting an error; returns its message.
pub fn err_of<T: std::fmt::Debug>(c: &dyn odoo_core::store::Conn, f: impl FnOnce() -> odoo_core::Result<T>) -> String { match store::nested(c, f) { Ok(v) => panic!("expected an error, got {v:?}"), Err(e) => e.to_string() } }
pub fn cmd6(ids: &[i64]) -> Value { Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(ids.iter().map(|i| Value::Int(*i)).collect())])]) }
pub fn rd(env: &Env, model: &str, id: i64, f: &str) -> Value { orm::read(env, model, &[id], &[f.to_string()]).unwrap().remove(0).remove(f).unwrap_or(Value::Null) }
pub fn flag(r: &Row, k: &str) -> bool { r.get(k).map_or(false, |v| v.truthy()) }
pub fn rid(v: &Value) -> Option<i64> { match v { Value::Int(i) => Some(*i), Value::List(l) => l.first().and_then(|x| x.as_i64()), _ => None } }
pub fn new_env<'a>(reg: &'a Registry, c: &'a dyn odoo_core::store::Conn, rules: &'a odoo_core::Rules) -> Env<'a> { let env = Env::new(reg, c, rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env).unwrap(); env }

pub struct Fx { pub cfg: i64, pub cash: i64, pub bank: i64, pub later: i64, pub journal: i64, pub pen: i64, pub partner: i64, pub tax: i64 }
/// A configured register: a cash method on a cash journal with loss/profit accounts, a bank method and customer account.
pub fn fixture(env: &Env) -> Fx {
    let pt = stock::ensure_picking_type(env, "outgoing").unwrap();
    let loss = account::ensure_account(env, "expense", "Cash Loss").unwrap(); let profit = account::ensure_account(env, "income_other", "Cash Profit").unwrap(); let cash_acc = account::ensure_account(env, "asset_cash", "Cash").unwrap();
    let journal = mk(env, "account.journal", &[("name", "Cash".into()), ("code", "CSH1".into()), ("type", "cash".into()), ("loss_account_id", loss.into()), ("profit_account_id", profit.into()), ("default_account_id", cash_acc.into())]);
    let bank_j = account::ensure_journal(env, "bank", "BNK1", "Bank").unwrap();
    let cash = mk(env, "pos.payment.method", &[("name", "Cash".into()), ("journal_id", journal.into())]);
    let bank = mk(env, "pos.payment.method", &[("name", "Card".into()), ("journal_id", bank_j.into())]);
    let later = mk(env, "pos.payment.method", &[("name", "Customer Account".into()), ("split_transactions", true.into())]);
    let sale_j = account::ensure_journal(env, "sale", "INV", "Customer Invoices").unwrap();
    let cfg = mk(env, "pos.config", &[("name", "Shop".into()), ("picking_type_id", pt.into()), ("payment_method_ids", cmd6(&[cash, bank, later])), ("invoice_journal_id", sale_j.into()), ("journal_id", sale_j.into())]);
    let tax = mk(env, "account.tax", &[("name", "VAT 10%".into()), ("amount", 10.0.into()), ("amount_type", "percent".into()), ("type_tax_use", "sale".into())]);
    let pen = mk(env, "product.product", &[("name", "Pen".into()), ("list_price", 10.0.into()), ("type", "consu".into()), ("available_in_pos", true.into())]);
    let partner = mk(env, "res.partner", &[("name", "Azure".into())]);
    Fx { cfg, cash, bank, later, journal, pen, partner, tax }
}

/// A draft order of `qty` pens (10 + 10% tax each) in session `sid`, priced like the back-office does it.
pub fn draft_order(env: &Env, f: &Fx, sid: i64, qty: f64) -> i64 {
    let line = row(&[("product_id", f.pen.into()), ("qty", qty.into()), ("price_unit", 10.0.into()), ("price_subtotal", 0.0.into()), ("price_subtotal_incl", 0.0.into()), ("tax_ids", cmd6(&[f.tax]))]);
    let o = mk(env, "pos.order", &[("session_id", sid.into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()), ("amount_tax", 0.0.into()), ("amount_total", 0.0.into()), ("lines", Value::List(vec![Value::List(vec![0.into(), 0.into(), Value::Map(line)])]))]);
    let lines = ids(&rec(env, "pos.order", o).unwrap(), "lines");
    call(env, "pos.order.line", "_onchange_amount_line_all", &lines, Row::new()).unwrap();
    call(env, "pos.order", "_compute_prices", &[o], Row::new()).unwrap();
    o
}
pub fn pay(env: &Env, o: i64, method: i64, amount: f64) { call(env, "pos.order", "add_payment", &[o], row(&[("payment_method_id", method.into()), ("amount", amount.into())])).unwrap(); }
pub fn opened_session(env: &Env, f: &Fx, float: f64) -> i64 {
    let s = mk(env, "pos.session", &[("config_id", f.cfg.into())]);
    call(env, "pos.session", "set_opening_control", &[s], row(&[("cashbox_value", float.into()), ("notes", "".into())])).unwrap();
    s
}
