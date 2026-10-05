//! sale / sale_management / delivery behaviour ported from addons/sale/models/*.py (state transitions, invoicing,
//! down payments, pricelists). Each test builds an in-memory SQLite database from the real schema.
use odoo_core::{ddl, orm::{self, AllowAll, Env}, store::{self, Store}, Domain, Registry, Row, Value};
use odoo_modules::{rules_for, sale_util::flag, util::*};
use odoo_sqlite::SqliteStore;
use std::path::Path;

fn setup() -> (Registry, SqliteStore) {
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["sale_stock", "sale_management", "delivery", "account"]).unwrap();
    let st = SqliteStore::open(":memory:").unwrap();
    let plan = ddl::install_plan(&reg, st.dialect());
    store::run(&st, |c| { for s in &plan { c.execute(s, &[])?; } Ok(()) }).unwrap();
    (reg, st)
}
fn lines(v: Vec<Row>) -> Value { Value::List(v.into_iter().map(|m| Value::List(vec![0.into(), 0.into(), Value::Map(m)])).collect()) }
fn m2m(ids: &[i64]) -> Value { Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(ids.iter().map(|i| Value::Int(*i)).collect())])]) }
fn call(env: &Env, model: &str, method: &str, ids: &[i64], args: &[(&str, Value)]) -> odoo_core::Result<Value> { orm::call(env, model, method, ids, &row(args)) }
fn err(r: odoo_core::Result<Value>) -> String { r.expect_err("expected an error").to_string() }
fn product(env: &Env, name: &str, price: f64, ty: &str, policy: &str) -> i64 {
    orm::create(env, "product.product", row(&[("name", name.into()), ("list_price", price.into()), ("type", ty.into()), ("invoice_policy", policy.into())])).unwrap()
}
fn tax(env: &Env, name: &str, amount: f64) -> i64 {
    orm::create(env, "account.tax", row(&[("name", name.into()), ("amount", amount.into()), ("amount_type", "percent".into()), ("type_tax_use", "sale".into())])).unwrap()
}
fn partner(env: &Env, name: &str) -> i64 { orm::create(env, "res.partner", row(&[("name", name.into())])).unwrap() }
fn line(product: i64, qty: f64) -> Row { row(&[("product_id", product.into()), ("product_uom_qty", qty.into())]) }
fn order(env: &Env, partner: i64, ls: Vec<Row>) -> i64 { orm::create(env, "sale.order", row(&[("partner_id", partner.into()), ("order_line", lines(ls))])).unwrap() }
fn sol(env: &Env, order: i64) -> Vec<Row> { let mut v = children(env, "sale.order.line", "order_id", order).unwrap(); v.sort_by_key(|l| (l["sequence"].as_i64().unwrap_or(0), l["id"].as_i64().unwrap())); v }
fn ids_of(v: Value) -> Vec<i64> { match v { Value::List(l) => l.iter().filter_map(|x| x.as_i64()).collect(), o => panic!("not a list: {o:?}") } }
fn env_of<'a>(reg: &'a Registry, c: &'a dyn odoo_core::store::Conn, rules: &'a odoo_core::Rules) -> Env<'a> { let env = Env::new(reg, c, rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env).unwrap(); env }

#[test]
fn cancel_draft_lock_and_delete_rules() {
    let (reg, st) = setup(); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = env_of(&reg, c, &rules);
        let p = partner(&env, "Azure"); let prod = product(&env, "Desk", 100.0, "consu", "order");
        let so = order(&env, p, vec![line(prod, 2.0)]);
        let o = rd(&env, "sale.order", so)?;
        assert_eq!(text(&o, "state").unwrap(), "draft");
        // partner addresses default to the partner itself (address_get)
        assert_eq!(id_of(&o, "partner_invoice_id"), Some(p));
        assert_eq!(id_of(&o, "partner_shipping_id"), Some(p));
        assert_eq!(text(&o, "type_name").unwrap(), "Quotation");

        // cancelling a draft goes straight through (no wizard), and action_draft brings it back
        call(&env, "sale.order", "action_cancel", &[so], &[])?;
        assert_eq!(text(&rd(&env, "sale.order", so)?, "state").unwrap(), "cancel");
        assert_eq!(text(&sol(&env, so)[0], "state").unwrap(), "cancel");
        // a cancelled order cannot be confirmed
        assert_eq!(err(call(&env, "sale.order", "action_confirm", &[so], &[])), "Some orders are not in a state requiring confirmation.");
        call(&env, "sale.order", "action_draft", &[so], &[])?;
        assert_eq!(text(&rd(&env, "sale.order", so)?, "state").unwrap(), "draft");
        assert_eq!(text(&sol(&env, so)[0], "state").unwrap(), "draft");

        // sent -> cancel needs the wizard
        call(&env, "sale.order", "action_quotation_sent", &[so], &[])?;
        assert_eq!(text(&rd(&env, "sale.order", so)?, "state").unwrap(), "sent");
        assert_eq!(err(call(&env, "sale.order", "action_quotation_sent", &[so], &[])), "Only draft orders can be marked as sent directly.");
        let Value::Map(act) = call(&env, "sale.order", "action_cancel", &[so], &[])? else { panic!("wizard action expected") };
        assert_eq!(act["res_model"], Value::Text("sale.order.cancel".into()));
        assert_eq!(text(&rd(&env, "sale.order", so)?, "state").unwrap(), "sent");
        assert_eq!(call(&env, "sale.order", "_show_cancel_wizard", &[so], &[])?, Value::Bool(true));
        let w = orm::create(&env, "sale.order.cancel", row(&[("order_id", so.into())]))?;
        call(&env, "sale.order.cancel", "action_cancel", &[w], &[])?;
        assert_eq!(text(&rd(&env, "sale.order", so)?, "state").unwrap(), "cancel");
        // an order missing a product cannot be confirmed
        let bad = orm::create(&env, "sale.order", row(&[("partner_id", p.into()), ("order_line", lines(vec![row(&[("name", "free text".into()), ("price_unit", 5.0.into())])]))]))?;
        assert_eq!(err(call(&env, "sale.order", "action_confirm", &[bad], &[])), "A line on these orders missing a product, you cannot confirm it.");
        assert_eq!(call(&env, "sale.order", "_confirmation_error_message", &[bad], &[])?, Value::Text("A line on these orders missing a product, you cannot confirm it.".into()));

        // confirm -> lock: protects cancel + the sensitive line fields
        call(&env, "sale.order", "action_draft", &[so], &[])?;
        call(&env, "sale.order", "action_confirm", &[so], &[])?;
        let o = rd(&env, "sale.order", so)?;
        assert_eq!(text(&o, "state").unwrap(), "sale");
        assert_eq!(text(&sol(&env, so)[0], "state").unwrap(), "sale");
        assert_eq!(text(&o, "type_name").unwrap(), "Sales Order");
        call(&env, "sale.order", "action_lock", &[so], &[])?;
        assert!(flag(&rd(&env, "sale.order", so)?, "locked"));
        assert_eq!(err(call(&env, "sale.order", "action_cancel", &[so], &[])), "You cannot cancel a locked order. Please unlock it first.");
        let l0 = sol(&env, so)[0]["id"].as_i64().unwrap();
        let e = orm::write(&env, "sale.order.line", &[l0], row(&[("price_unit", 1.0.into())])).unwrap_err().to_string();
        assert_eq!(e, "It is forbidden to modify the following fields in a locked order:\nUnit Price");
        call(&env, "sale.order", "action_unlock", &[so], &[])?;
        // confirmed lines can't be deleted, confirmed orders can't be deleted
        assert!(orm::unlink(&env, "sale.order.line", &[l0]).unwrap_err().to_string().starts_with("Once a sales order is confirmed, you can't remove one of its lines"));
        assert_eq!(orm::unlink(&env, "sale.order", &[so]).unwrap_err().to_string(), "You can not delete a sent quotation or a confirmed sales order. You must first cancel it.");

        // cancel with the wizard skipped: lines follow, invoice status resets
        call(&env.with_ctx("disable_cancel_warning", Value::Bool(true)), "sale.order", "action_cancel", &[so], &[])?;
        assert_eq!(text(&rd(&env, "sale.order", so)?, "state").unwrap(), "cancel");
        assert_eq!(text(&sol(&env, so)[0], "state").unwrap(), "cancel");
        assert_eq!(text(&rd(&env, "sale.order", so)?, "invoice_status").unwrap(), "no");
        orm::unlink(&env, "sale.order", &[so])?;
        assert!(orm::search(&env, "sale.order.line", &Domain::Term("order_id".into(), "=".into(), so.into()), None, None, 0)?.is_empty());
        Ok(())
    }).unwrap();
}

#[test]
fn invoicing_quantities_status_and_refund_sign() {
    let (reg, st) = setup(); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = env_of(&reg, c, &rules);
        let p = partner(&env, "Azure"); let t15 = tax(&env, "VAT 15%", 15.0);
        let a = product(&env, "Desk", 100.0, "consu", "order");
        let b = product(&env, "Support", 40.0, "service", "delivery");
        let tx = Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(vec![t15.into()])])]);
        let so = order(&env, p, vec![
            row(&[("display_type", "line_section".into()), ("name", "Hardware".into()), ("sequence", 1.into())]),
            { let mut l = line(a, 2.0); l.insert("tax_id".into(), tx.clone()); l.insert("sequence".into(), 2.into()); l },
            row(&[("display_type", "line_note".into()), ("name", "Delivered in 3 days".into()), ("sequence", 3.into())]),
            { let mut l = line(b, 5.0); l.insert("sequence".into(), 4.into()); l },
        ]);
        // before confirmation nothing is invoiceable
        assert_eq!(text(&rd(&env, "sale.order", so)?, "invoice_status").unwrap(), "no");
        assert_eq!(err(call(&env, "sale.order", "_create_invoices", &[so], &[])), odoo_modules::sale_methods::NOTHING_TO_INVOICE);
        assert!(ids_of(call(&env.with_ctx("raise_if_nothing_to_invoice", Value::Bool(false)), "sale.order", "_create_invoices", &[so], &[])?).is_empty());
        call(&env, "sale.order", "action_confirm", &[so], &[])?;
        let ls = sol(&env, so);
        let (sec, la, note, lb) = (&ls[0], &ls[1], &ls[2], &ls[3]);
        // ordered-quantity policy: invoiceable at once; delivered policy: nothing delivered yet
        assert_eq!((num(la, "qty_to_invoice"), num(lb, "qty_to_invoice")), (2.0, 0.0));
        assert_eq!((text(la, "invoice_status").unwrap(), text(lb, "invoice_status").unwrap()), ("to invoice".into(), "no".into()));
        assert_eq!(text(sec, "invoice_status").unwrap(), "invoiced");   // like Odoo: 0 >= 0 for display lines
        assert_eq!(text(&rd(&env, "sale.order", so)?, "invoice_status").unwrap(), "to invoice");
        // _get_invoiceable_lines: the section is only kept when one of its lines is invoiceable; notes always
        let inv_lines = ids_of(call(&env, "sale.order", "_get_invoiceable_lines", &[so], &[("final", false.into())])?);
        assert_eq!(inv_lines, vec![rid(sec), rid(la), rid(note)]);

        // invoice #1
        let inv = ids_of(call(&env, "sale.order", "_create_invoices", &[so], &[])?);
        assert_eq!(inv.len(), 1);
        let m1 = rd(&env, "account.move", inv[0])?;
        assert_eq!((text(&m1, "move_type").unwrap(), text(&m1, "invoice_origin").unwrap(), text(&m1, "state").unwrap()), ("out_invoice".into(), "S00001".into(), "draft".into()));
        assert_eq!(id_of(&m1, "partner_id"), Some(p));
        let il = children(&env, "account.move.line", "move_id", inv[0])?;
        assert_eq!(il.len(), 3);
        let kinds: Vec<String> = il.iter().map(|l| text(l, "display_type").unwrap_or_default()).collect();
        assert_eq!(kinds, vec!["line_section", "product", "line_note"]);
        let prod_line = &il[1];
        assert_eq!((num(prod_line, "quantity"), num(prod_line, "price_unit"), id_of(prod_line, "product_id")), (2.0, 100.0, Some(a)));
        assert!(text(prod_line, "name").unwrap().contains("Desk"));
        assert_eq!(ids(prod_line, "tax_ids"), vec![t15]);
        assert_eq!(ids(prod_line, "sale_line_ids"), vec![rid(la)]);
        assert_eq!(num(&m1, "amount_total"), 230.0);
        // line + order statuses follow the (draft) invoice
        let la2 = rd(&env, "sale.order.line", rid(la))?;
        assert_eq!((num(&la2, "qty_invoiced"), num(&la2, "qty_to_invoice")), (2.0, 0.0));
        assert_eq!(text(&la2, "invoice_status").unwrap(), "invoiced");
        assert_eq!(num(&la2, "qty_invoiced_posted"), 0.0);          // draft: not posted yet
        assert_eq!(num(&la2, "amount_invoiced"), 0.0);
        let o = rd(&env, "sale.order", so)?;
        assert_eq!(num(&o, "invoice_count"), 1.0);
        assert_eq!(text(&o, "invoice_status").unwrap(), "no");       // support line still 'no'

        // posting makes the *_posted / amount_invoiced figures count
        call(&env, "account.move", "action_post", &[inv[0]], &[])?;
        let la3 = rd(&env, "sale.order.line", rid(la))?;
        assert_eq!((num(&la3, "qty_invoiced_posted"), num(&la3, "untaxed_amount_invoiced"), num(&la3, "amount_invoiced")), (2.0, 200.0, 230.0));
        assert_eq!((num(&la3, "amount_to_invoice"), num(&la3, "untaxed_amount_to_invoice")), (0.0, 0.0));
        assert_eq!(num(&rd(&env, "sale.order", so)?, "amount_invoiced"), 230.0);
        assert_eq!(num(&la3, "price_reduce_taxexcl"), 100.0);
        assert_eq!(num(&la3, "price_reduce_taxinc"), 115.0);

        // delivering the service makes it invoiceable
        orm::write(&env, "sale.order.line", &[rid(lb)], row(&[("qty_delivered", 3.0.into())]))?;
        let lb2 = rd(&env, "sale.order.line", rid(lb))?;
        assert_eq!((num(&lb2, "qty_to_invoice"), text(&lb2, "invoice_status").unwrap()), (3.0, "to invoice".into()));
        assert_eq!(num(&lb2, "untaxed_amount_to_invoice"), 120.0);
        assert_eq!(text(&rd(&env, "sale.order", so)?, "invoice_status").unwrap(), "to invoice");
        let inv2 = ids_of(call(&env, "sale.order", "_create_invoices", &[so], &[])?);
        let il2 = children(&env, "account.move.line", "move_id", inv2[0])?;
        // like Odoo: the pending section and the (always invoiced) note come along with the support line
        assert_eq!(il2.iter().map(|l| text(l, "display_type").unwrap_or_default()).collect::<Vec<_>>(), vec!["line_section", "line_note", "product"]);
        assert_eq!((num(&il2[2], "quantity"), id_of(&il2[2], "product_id")), (3.0, Some(b)));
        assert_eq!(num(&rd(&env, "sale.order", so)?, "invoice_count"), 2.0);
        // over-delivery above the ordered qty on an 'order' policy line = upselling
        orm::write(&env, "sale.order.line", &[rid(la)], row(&[("qty_delivered", 3.0.into())]))?;
        assert_eq!(text(&rd(&env, "sale.order.line", rid(la))?, "invoice_status").unwrap(), "upselling");

        // a credit note linked to the sale line reduces qty_invoiced (and re-opens qty_to_invoice)
        let rn = orm::create(&env, "account.move", row(&[("move_type", "out_refund".into()), ("partner_id", p.into()), ("invoice_origin", "S00001".into())]))?;
        orm::create(&env, "account.move.line", row(&[("move_id", rn.into()), ("display_type", "product".into()), ("product_id", a.into()), ("name", "Desk".into()), ("quantity", 1.0.into()), ("price_unit", 100.0.into()), ("sale_line_ids", Value::List(vec![Value::List(vec![4.into(), rid(la).into()])]))]))?;
        let la4 = rd(&env, "sale.order.line", rid(la))?;
        assert_eq!((num(&la4, "qty_invoiced"), num(&la4, "qty_to_invoice")), (1.0, 1.0));
        // cancelled invoices are ignored
        call(&env, "account.move", "button_cancel", &[rn], &[])?;
        assert_eq!(num(&rd(&env, "sale.order.line", rid(la))?, "qty_invoiced"), 2.0);

        // cancel the order: draft invoices are cancelled with it
        call(&env, "sale.order", "_action_cancel", &[so], &[])?;
        assert_eq!(text(&rd(&env, "account.move", inv2[0])?, "state").unwrap(), "cancel");
        assert_eq!(text(&rd(&env, "account.move", inv[0])?, "state").unwrap(), "posted");
        assert_eq!(text(&rd(&env, "sale.order.line", rid(lb))?, "invoice_status").unwrap(), "no");
        Ok(())
    }).unwrap();
}
fn rd(env: &Env, model: &str, id: i64) -> odoo_core::Result<Row> { Ok(orm::read(env, model, &[id], &[])?.remove(0)) }
fn rid(r: &Row) -> i64 { r["id"].as_i64().unwrap() }

#[test]
fn grouping_and_final_refund() {
    let (reg, st) = setup(); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = env_of(&reg, c, &rules);
        let p = partner(&env, "Azure"); let q = partner(&env, "Other");
        let a = product(&env, "Desk", 100.0, "consu", "order");
        let s1 = order(&env, p, vec![line(a, 1.0)]); let s2 = order(&env, p, vec![line(a, 2.0)]); let s3 = order(&env, q, vec![line(a, 3.0)]);
        call(&env, "sale.order", "action_confirm", &[s1, s2, s3], &[])?;
        // same partner/currency/fiscal position share one invoice; the origins are joined, lines resequenced
        let inv = ids_of(call(&env, "sale.order", "_create_invoices", &[s1, s2, s3], &[])?);
        assert_eq!(inv.len(), 2);
        let mut by_partner: Vec<(i64, String, usize)> = vec![];
        for m in &inv { let r = rd(&env, "account.move", *m)?; by_partner.push((id_of(&r, "partner_id").unwrap(), text(&r, "invoice_origin").unwrap(), children(&env, "account.move.line", "move_id", *m)?.len())); }
        by_partner.sort();
        assert_eq!(by_partner, vec![(p, "S00001, S00002".into(), 2), (q, "S00003".into(), 1)]);
        // grouped=True: one invoice per order
        let (s4, s5) = (order(&env, p, vec![line(a, 1.0)]), order(&env, p, vec![line(a, 1.0)]));
        call(&env, "sale.order", "action_confirm", &[s4, s5], &[])?;
        assert_eq!(ids_of(call(&env, "sale.order", "_create_invoices", &[s4, s5], &[("grouped", true.into())])?).len(), 2);

        // lowering the ordered qty below the invoiced one gives a negative qty_to_invoice: only `final` bills it, as a refund
        orm::write(&env, "sale.order.line", &[rid(&sol(&env, s1)[0])], row(&[("product_uom_qty", 0.0.into())]))?;
        let l = rd(&env, "sale.order.line", rid(&sol(&env, s1)[0]))?;
        assert_eq!(num(&l, "qty_to_invoice"), -1.0);
        assert_eq!(err(call(&env, "sale.order", "_create_invoices", &[s1], &[])), odoo_modules::sale_methods::NOTHING_TO_INVOICE);
        let rf = ids_of(call(&env, "sale.order", "_create_invoices", &[s1], &[("final", true.into())])?);
        let r = rd(&env, "account.move", rf[0])?;
        assert_eq!(text(&r, "move_type").unwrap(), "out_refund");
        assert_eq!(num(&r, "amount_total"), 100.0);
        let rl = children(&env, "account.move.line", "move_id", rf[0])?;
        assert_eq!(num(&rl[0], "quantity"), 1.0);
        // credit note + original invoice net to zero
        assert_eq!(num(&rd(&env, "sale.order.line", rid(&sol(&env, s1)[0]))?, "qty_invoiced"), 0.0);
        Ok(())
    }).unwrap();
}

#[test]
fn down_payments_percentage_fixed_and_deduction() {
    let (reg, st) = setup(); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = env_of(&reg, c, &rules);
        let p = partner(&env, "Azure"); let t15 = tax(&env, "VAT 15%", 15.0);
        let a = product(&env, "Desk", 100.0, "consu", "order");
        let tx = Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(vec![t15.into()])])]);
        let so = order(&env, p, vec![{ let mut l = line(a, 2.0); l.insert("tax_id".into(), tx.clone()); l }]);
        call(&env, "sale.order", "action_confirm", &[so], &[])?;
        assert_eq!(num(&rd(&env, "sale.order", so)?, "amount_total"), 230.0);

        // positive amount required
        let w0 = orm::create(&env, "sale.advance.payment.inv", row(&[("advance_payment_method", "percentage".into()), ("amount", 0.0.into()), ("sale_order_ids", m2m(&[so]))]))?;
        assert_eq!(err(call(&env, "sale.advance.payment.inv", "create_invoices", &[w0], &[])), "The value of the down payment amount must be positive.");

        // 50% down payment => section + one dp line per tax combination
        let w = orm::create(&env, "sale.advance.payment.inv", row(&[("advance_payment_method", "percentage".into()), ("amount", 50.0.into()), ("sale_order_ids", m2m(&[so]))]))?;
        let Value::Map(act) = call(&env, "sale.advance.payment.inv", "create_invoices", &[w], &[])? else { panic!() };
        let dp_inv = act["res_id"].as_i64().unwrap();
        let ls = sol(&env, so);
        assert_eq!(ls.len(), 3);
        let (sec, dp) = (&ls[1], &ls[2]);
        assert!(flag(sec, "is_downpayment") && text(sec, "display_type").unwrap() == "line_section");
        assert!(flag(dp, "is_downpayment") && id_of(dp, "product_id").is_none());
        assert_eq!((num(dp, "price_unit"), num(dp, "product_uom_qty")), (100.0, 0.0));
        assert_eq!(ids(dp, "tax_id"), vec![t15]);
        assert!(text(dp, "name").unwrap().starts_with("Down Payment: ") && text(dp, "name").unwrap().ends_with("(Draft)"));
        let m = rd(&env, "account.move", dp_inv)?;
        assert_eq!((text(&m, "move_type").unwrap(), num(&m, "amount_total")), ("out_invoice".into(), 115.0));
        let il = children(&env, "account.move.line", "move_id", dp_inv)?;
        assert_eq!((num(&il[0], "quantity"), text(&il[0], "name").unwrap()), (1.0, "Down payment of 50.00%".into()));
        assert!(flag(&il[0], "is_downpayment"));
        // dp lines never drive the order invoice status; the regular line is still invoiceable
        assert_eq!(text(&rd(&env, "sale.order", so)?, "invoice_status").unwrap(), "to invoice");
        let dp1 = rd(&env, "sale.order.line", rid(dp))?;
        assert_eq!((num(&dp1, "qty_invoiced"), num(&dp1, "qty_to_invoice")), (1.0, -1.0));
        // posting renames the dp line
        call(&env, "account.move", "action_post", &[dp_inv], &[])?;
        assert_eq!(text(&rd(&env, "sale.order.line", rid(dp))?, "name").unwrap(), "Down Payment");

        // final invoice deducts the posted down payment
        let w2 = orm::create(&env, "sale.advance.payment.inv", row(&[("advance_payment_method", "delivered".into()), ("sale_order_ids", m2m(&[so]))]))?;
        let Value::Map(act) = call(&env, "sale.advance.payment.inv", "create_invoices", &[w2], &[])? else { panic!() };
        let fin = act["res_id"].as_i64().unwrap();
        let fl = children(&env, "account.move.line", "move_id", fin)?;
        let kinds: Vec<String> = fl.iter().map(|l| text(l, "display_type").unwrap_or_default()).collect();
        assert_eq!(kinds, vec!["product", "line_section", "product"]);
        assert_eq!((num(&fl[0], "quantity"), num(&fl[0], "price_unit")), (2.0, 100.0));
        assert_eq!((num(&fl[2], "quantity"), num(&fl[2], "price_unit"), flag(&fl[2], "is_downpayment")), (-1.0, 100.0, true));
        assert_eq!(num(&rd(&env, "account.move", fin)?, "amount_total"), 115.0);          // 230 - 115
        let dp2 = rd(&env, "sale.order.line", rid(dp))?;
        assert_eq!((num(&dp2, "qty_invoiced"), num(&dp2, "qty_to_invoice"), text(&dp2, "invoice_status").unwrap()), (0.0, 0.0, "invoiced".into()));
        assert_eq!(text(&rd(&env, "sale.order", so)?, "invoice_status").unwrap(), "invoiced");
        // nothing left
        assert_eq!(err(call(&env, "sale.order", "_create_invoices", &[so], &[])), odoo_modules::sale_methods::NOTHING_TO_INVOICE);

        // fixed amount down payment (tax-included amount, split back through the ratio)
        let so2 = order(&env, p, vec![{ let mut l = line(a, 2.0); l.insert("tax_id".into(), tx.clone()); l }]);
        call(&env, "sale.order", "action_confirm", &[so2], &[])?;
        let wf = orm::create(&env, "sale.advance.payment.inv", row(&[("advance_payment_method", "fixed".into()), ("fixed_amount", 115.0.into()), ("sale_order_ids", m2m(&[so2]))]))?;
        let Value::Map(act) = call(&env, "sale.advance.payment.inv", "create_invoices", &[wf], &[])? else { panic!() };
        let fi = rd(&env, "account.move", act["res_id"].as_i64().unwrap())?;
        assert_eq!(num(&fi, "amount_total"), 115.0);
        let dpl = sol(&env, so2).into_iter().find(|l| flag(l, "is_downpayment") && text(l, "display_type").is_none()).unwrap();
        assert_eq!(num(&dpl, "price_unit"), 100.0);
        // deleting the draft dp invoice removes its dp lines again
        orm::unlink(&env, "account.move", &[act["res_id"].as_i64().unwrap()])?;
        assert!(!sol(&env, so2).iter().any(|l| flag(l, "is_downpayment") && text(l, "display_type").is_none()));
        Ok(())
    }).unwrap();
}

#[test]
fn pricelist_rules_and_order_pricing() {
    let (reg, st) = setup(); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = env_of(&reg, c, &rules);
        let usd = find_one(&env, "res.currency", Domain::True)?.unwrap();
        let categ = orm::create(&env, "product.category", row(&[("name", "Furniture".into())]))?;
        let a = orm::create(&env, "product.product", row(&[("name", "Desk".into()), ("list_price", 100.0.into()), ("standard_price", 60.0.into()), ("type", "consu".into()), ("categ_id", categ.into())]))?;
        let b = orm::create(&env, "product.product", row(&[("name", "Lamp".into()), ("list_price", 30.0.into()), ("type", "consu".into())]))?;
        let tmpl_a = id_of(&rd(&env, "product.product", a)?, "product_tmpl_id").unwrap();
        let pl = orm::create(&env, "product.pricelist", row(&[("name", "Public".into()), ("currency_id", usd.into())]))?;
        let item = |v: &[(&str, Value)]| { let mut r = row(v); r.insert("pricelist_id".into(), pl.into()); orm::create(&env, "product.pricelist.item", r).unwrap() };
        let global = item(&[("compute_price", "formula".into()), ("base", "list_price".into()), ("price_discount", 5.0.into()), ("price_surcharge", 2.0.into())]);
        let cat = item(&[("applied_on", "2_product_category".into()), ("categ_id", categ.into()), ("compute_price", "percentage".into()), ("percent_price", 10.0.into())]);
        let fixed = item(&[("product_tmpl_id", tmpl_a.into()), ("compute_price", "fixed".into()), ("fixed_price", 80.0.into()), ("min_quantity", 5.0.into())]);
        let expired = item(&[("product_id", a.into()), ("compute_price", "fixed".into()), ("fixed_price", 1.0.into()), ("date_end", "2000-01-01 00:00:00".into())]);
        // create normalises the scope + name from the product/category
        let it = rd(&env, "product.pricelist.item", fixed)?;
        assert_eq!((text(&it, "applied_on").unwrap(), text(&it, "name").unwrap()), ("1_product".into(), "Desk".into()));
        assert_eq!(text(&rd(&env, "product.pricelist.item", cat)?, "name").unwrap(), "Category: Furniture");
        assert_eq!(text(&rd(&env, "product.pricelist.item", global)?, "applied_on").unwrap(), "3_global");
        assert_eq!(text(&rd(&env, "product.pricelist.item", expired)?, "applied_on").unwrap(), "0_product_variant");

        let price = |prod: i64, qty: f64| -> f64 { call(&env, "product.pricelist", "_get_product_price", &[pl], &[("product_id", prod.into()), ("quantity", qty.into()), ("date", "2026-06-01 10:00:00".into())]).unwrap().as_f64().unwrap() };
        assert_eq!(price(b, 1.0), 30.5);          // 30 - 5% + 2 (global formula)
        assert_eq!(price(a, 1.0), 90.0);          // category 10% beats the global formula (applied_on order); expired variant rule ignored
        assert_eq!(price(a, 4.0), 90.0);
        assert_eq!(price(a, 5.0), 80.0);          // product rule needs min_quantity 5
        let Value::List(r) = call(&env, "product.pricelist", "_get_product_price_rule", &[pl], &[("product_id", a.into()), ("quantity", 5.0.into()), ("date", "2026-06-01 10:00:00".into())])? else { panic!() };
        assert_eq!((r[0].as_f64().unwrap(), r[1].as_i64().unwrap()), (80.0, fixed));
        assert_eq!(call(&env, "product.pricelist", "_get_product_rule", &[pl], &[("product_id", b.into()), ("date", "2026-06-01 10:00:00".into())])?, Value::Int(global));
        // date window: the 2000 rule applies to a date before its end
        assert_eq!(call(&env, "product.pricelist", "_get_product_price", &[pl], &[("product_id", a.into()), ("date", "1999-01-01 00:00:00".into())])?.as_f64(), Some(1.0));

        // a second pricelist based on the first (formula 10% off) and one based on cost with markup
        let pl2 = orm::create(&env, "product.pricelist", row(&[("name", "VIP".into()), ("currency_id", usd.into())]))?;
        orm::create(&env, "product.pricelist.item", row(&[("pricelist_id", pl2.into()), ("compute_price", "formula".into()), ("base", "pricelist".into()), ("base_pricelist_id", pl.into()), ("price_discount", 10.0.into())]))?;
        assert!((call(&env, "product.pricelist", "_get_product_price", &[pl2], &[("product_id", b.into())])?.as_f64().unwrap() - (30.0 * 0.95 + 2.0) * 0.9).abs() < 1e-9);
        let pl3 = orm::create(&env, "product.pricelist", row(&[("name", "Cost+".into()), ("currency_id", usd.into())]))?;
        let mk = orm::create(&env, "product.pricelist.item", row(&[("pricelist_id", pl3.into()), ("compute_price", "formula".into()), ("base", "standard_price".into()), ("price_markup", 50.0.into())]))?;
        assert_eq!(num(&rd(&env, "product.pricelist.item", mk)?, "price_discount"), -50.0);   // inverse of the markup
        assert_eq!(call(&env, "product.pricelist", "_get_product_price", &[pl3], &[("product_id", a.into())])?.as_f64(), Some(90.0));  // 60 * 1.5
        // pricelists used as a base cannot be deleted; recursion is rejected
        assert!(orm::unlink(&env, "product.pricelist", &[pl]).unwrap_err().to_string().starts_with("You cannot delete pricelist(s):\n(Public (USD))"));
        assert!(store::nested(c, || orm::create(&env, "product.pricelist.item", row(&[("pricelist_id", pl.into()), ("compute_price", "formula".into()), ("base", "pricelist".into()), ("base_pricelist_id", pl.into())])).map(|_| ())).unwrap_err().to_string().contains("You cannot assign the Main Pricelist"));
        assert!(store::nested(c, || orm::create(&env, "product.pricelist.item", row(&[("pricelist_id", pl.into()), ("applied_on", "2_product_category".into())])).map(|_| ())).unwrap_err().to_string().contains("Please specify the category"));

        // orders pick the partner's pricelist and price lines from it
        let cust = orm::create(&env, "res.partner", row(&[("name", "VIP customer".into()), ("specific_property_product_pricelist", pl.into())]))?;
        assert_eq!(call(&env, "product.pricelist", "_get_partner_pricelist_multi", &[], &[("partner_ids", Value::List(vec![cust.into()]))])?, Value::Map([(cust.to_string(), Value::Int(pl))].into_iter().collect()));
        let so = order(&env, cust, vec![line(a, 1.0), line(a, 5.0), line(b, 1.0)]);
        let o = rd(&env, "sale.order", so)?;
        assert_eq!((id_of(&o, "pricelist_id"), id_of(&o, "currency_id")), (Some(pl), Some(usd)));
        let ls = sol(&env, so);
        assert_eq!(ls.iter().map(|l| num(l, "price_unit")).collect::<Vec<_>>(), vec![90.0, 80.0, 30.5]);
        assert_eq!(ls.iter().map(|l| num(l, "discount")).collect::<Vec<_>>(), vec![0.0, 0.0, 0.0]);   // discount feature (group) is off
        assert_eq!(num(&o, "amount_untaxed"), 90.0 + 400.0 + 30.5);
        // without a pricelist rule match the list price is used; an explicit price_unit is kept
        let so_b = order(&env, partner(&env, "No list"), vec![line(b, 2.0), { let mut l = line(b, 1.0); l.insert("price_unit".into(), 12.0.into()); l }]);
        let lb = sol(&env, so_b);
        assert_eq!((num(&lb[0], "price_unit"), num(&lb[1], "price_unit")), (30.5, 12.0));   // single pricelist in the DB applies to everybody (fallback)
        // action_update_prices re-prices from the pricelist; confirmed orders can't change pricelist
        orm::write(&env, "product.pricelist.item", &[global], row(&[("price_surcharge", 0.0.into())]))?;
        call(&env, "sale.order", "action_update_prices", &[so], &[])?;
        assert_eq!(num(&sol(&env, so)[2], "price_unit"), 28.5);
        call(&env, "sale.order", "action_confirm", &[so], &[])?;
        assert_eq!(orm::write(&env, "sale.order", &[so], row(&[("pricelist_id", pl2.into())])).unwrap_err().to_string(), "You cannot change the pricelist of a confirmed order !");
        Ok(())
    }).unwrap();
}

#[test]
fn pricelist_discount_feature_and_fiscal_position_taxes() {
    let (reg, st) = setup(); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = env_of(&reg, c, &rules);
        let usd = find_one(&env, "res.currency", Domain::True)?.unwrap();
        let a = product(&env, "Desk", 100.0, "consu", "order");
        let pl = orm::create(&env, "product.pricelist", row(&[("name", "Public".into()), ("currency_id", usd.into())]))?;
        orm::create(&env, "product.pricelist.item", row(&[("pricelist_id", pl.into()), ("compute_price", "percentage".into()), ("percent_price", 20.0.into())]))?;
        let cust = partner(&env, "Azure");
        let so = order(&env, cust, vec![line(a, 1.0)]);
        assert_eq!((num(&sol(&env, so)[0], "price_unit"), num(&sol(&env, so)[0], "discount")), (80.0, 0.0));
        // enable `sale.group_discount_per_so_line` for the superuser: the pricelist discount becomes visible on the line
        let g = orm::create(&env, "res.groups", row(&[("name", "Discount on lines".into())]))?;
        orm::create(&env, "ir.model.data", row(&[("module", "sale".into()), ("name", "group_discount_per_so_line".into()), ("model", "res.groups".into()), ("res_id", g.into())]))?;
        orm::write(&env, "res.users", &[1], row(&[("groups_id", Value::List(vec![Value::List(vec![4.into(), g.into()])]))]))?;
        let so2 = order(&env, cust, vec![line(a, 1.0)]);
        let l = &sol(&env, so2)[0];
        assert_eq!((num(l, "price_unit"), num(l, "discount"), num(l, "price_subtotal")), (100.0, 20.0, 80.0));
        assert_eq!(num(&rd(&env, "sale.order", so2)?, "amount_undiscounted"), 100.0);

        orm::write(&env, "product.pricelist", &[pl], row(&[("active", false.into())]))?;   // plain list prices from here on
        // fiscal position taxes: product taxes are mapped on the line; tax-included prices are re-expressed
        let t_inc = orm::create(&env, "account.tax", row(&[("name", "VAT 20% incl".into()), ("amount", 20.0.into()), ("amount_type", "percent".into()), ("type_tax_use", "sale".into()), ("price_include_override", "tax_included".into())]))?;
        let t_export = tax(&env, "Export 0%", 0.0);
        let fp = orm::create(&env, "account.fiscal.position", row(&[("name", "Export".into())]))?;
        orm::create(&env, "account.fiscal.position.tax", row(&[("position_id", fp.into()), ("tax_src_id", t_inc.into()), ("tax_dest_id", t_export.into())]))?;
        let c = orm::create(&env, "res.partner", row(&[("name", "Exporter".into()), ("property_account_position_id", fp.into())]))?;
        let pi = orm::create(&env, "product.product", row(&[("name", "Gadget".into()), ("list_price", 120.0.into()), ("type", "consu".into())]))?;
        orm::write(&env, "product.template", &[id_of(&rec(&env, "product.product", pi)?, "product_tmpl_id").unwrap()], row(&[("taxes_id", Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(vec![t_inc.into()])])]))]))?;
        let so3 = order(&env, c, vec![line(pi, 1.0)]);
        assert_eq!(id_of(&rd(&env, "sale.order", so3)?, "fiscal_position_id"), Some(fp));
        let l = &sol(&env, so3)[0];
        assert_eq!(ids(l, "tax_id"), vec![t_export]);
        assert!((num(l, "price_unit") - 100.0).abs() < 1e-9);       // 120 incl. 20% -> 100 excl. under the export position
        // without the position the included tax is kept and the price unchanged
        let so4 = order(&env, partner(&env, "Local"), vec![line(pi, 1.0)]);
        let l = &sol(&env, so4)[0];
        assert_eq!((ids(l, "tax_id"), num(l, "price_unit"), num(l, "price_subtotal")), (vec![t_inc], 120.0, 100.0));
        Ok(())
    }).unwrap();
}
