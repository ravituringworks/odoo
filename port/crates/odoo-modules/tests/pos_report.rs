//! report.point_of_sale.report_saledetails (pos_report_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{rules_for, util::*};
use pos_common::*;

const REPORT: &str = "report.point_of_sale.report_saledetails";
fn m(v: &Value) -> &Row { match v { Value::Map(m) => m, _ => panic!("{v:?}") } }
fn l(v: &Value) -> &Vec<Value> { match v { Value::List(l) => l, _ => panic!("{v:?}") } }

#[test]
fn session_sales_details_report() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 50.0);
        let cat = mk(&env, "pos.category", &[("name", "Stationery".into())]);
        orm::write(&env, "product.template", &[id_of(&rec(&env, "product.product", f.pen)?, "product_tmpl_id").unwrap()], row(&[("pos_categ_ids", cmd6(&[cat]))]))?;
        // 22 in cash, 22 on a card invoice, a return of one pen (-11 cash), and a cash-out of 5
        let a = draft_order(&env, &f, s, 2.0); pay(&env, a, f.cash, 22.0); call(&env, "pos.order", "action_pos_order_paid", &[a], Row::new())?;
        let b = draft_order(&env, &f, s, 2.0); orm::write(&env, "pos.order", &[b], row(&[("partner_id", f.partner.into())]))?; pay(&env, b, f.bank, 22.0);
        call(&env, "pos.order", "action_pos_order_invoice", &[b], Row::new())?;
        let r = draft_order(&env, &f, s, -1.0); pay(&env, r, f.cash, -11.0); call(&env, "pos.order", "action_pos_order_paid", &[r], Row::new())?;
        call(&env, "pos.session", "try_cash_in_out", &[s], row(&[("_type", "out".into()), ("amount", 5.0.into()), ("reason", "milk".into()), ("extras", Value::Map(row(&[("translatedType", "out".into())])))]))?;
        // a 20% discounted pen
        let d = draft_order(&env, &f, s, 1.0);
        let dl = ids(&rec(&env, "pos.order", d)?, "lines")[0];
        orm::write(&env, "pos.order.line", &[dl], row(&[("discount", 20.0.into())]))?;
        call(&env, "pos.order.line", "_onchange_amount_line_all", &[dl], Row::new())?; call(&env, "pos.order", "_compute_prices", &[d], Row::new())?;
        assert_eq!(num(&rec(&env, "pos.order", d)?, "amount_total"), 8.8);
        pay(&env, d, f.cash, 8.8); call(&env, "pos.order", "action_pos_order_paid", &[d], Row::new())?;
        // cash: 50 float + 22 - 11 + 8.8 - 5 = 64.8
        call(&env, "pos.session", "post_closing_cash_details", &[s], row(&[("counted_cash", 64.8.into())]))?;
        call(&env, "pos.session", "close_session_from_ui", &[s], Row::new())?;
        assert_eq!(text(&rec(&env, "pos.session", s)?, "state").unwrap(), "closed");

        let rep = call(&env, REPORT, "get_sale_details", &[], row(&[("session_ids", Value::List(vec![s.into()]))]))?;
        let r = m(&rep);
        assert_eq!((r["nbr_orders"].as_i64(), text(r, "state").as_deref(), r["show_payment_per_method"].clone()), (Some(4), Some("closed"), Value::Bool(false)));
        assert_eq!(r["session_name"], rec(&env, "pos.session", s)?["name"]);
        assert_eq!(m(&r["currency"])["total_paid"].as_f64(), Some(41.8));   // 22 + 22 - 11 + 8.8
        assert_eq!(l(&r["config_names"]), &vec![Value::Text("Shop".into())]);
        // sold products: 2 + 2 pens at 10 and one at 10 with 20% off; the return is separate
        let prods = l(&r["products"]); assert_eq!(prods.len(), 1);
        assert_eq!(m(&prods[0])["name"], Value::Text("Stationery".into()));
        let lines = l(&m(&prods[0])["products"]); assert_eq!(lines.len(), 2, "full price and discounted price are separate rows");
        let full = lines.iter().map(m).find(|x| x["discount"].as_f64() == Some(0.0)).unwrap();
        assert_eq!((full["quantity"].as_f64(), full["base_amount"].as_f64(), full["total_paid"].as_f64(), full["product_name"].clone()), (Some(4.0), Some(40.0), Some(40.0), Value::Text("Pen".into())));
        let disc = lines.iter().map(m).find(|x| x["discount"].as_f64() == Some(20.0)).unwrap();
        assert_eq!((disc["quantity"].as_f64(), disc["base_amount"].as_f64()), (Some(1.0), Some(8.0)));
        assert_eq!((m(&r["products_info"])["qty"].as_f64(), m(&r["products_info"])["total"].as_f64()), (Some(5.0), Some(48.0)));
        let rprods = l(&r["refund_products"]); assert_eq!(m(&l(&m(&rprods[0])["products"])[0])["quantity"].as_f64(), Some(-1.0));
        assert_eq!(m(&r["refund_info"])["qty"].as_f64(), Some(-1.0));
        // taxes
        assert_eq!(m(&r["taxes_info"])["tax_amount"].as_f64(), Some(4.8)); assert_eq!(m(&r["taxes_info"])["base_amount"].as_f64(), Some(48.0));
        assert_eq!(m(&l(&r["taxes"])[0])["name"], Value::Text("VAT 10%".into()));
        assert_eq!(m(&r["refund_taxes_info"])["tax_amount"].as_f64(), Some(-1.0));
        // discounts: one order carries a discount worth 2.2 (tax included)
        assert_eq!(r["discount_number"].as_i64(), Some(1)); assert_eq!(r["discount_amount"].as_f64(), Some(2.2));
        // payments: the cash row carries the drawer reconciliation, the card row a plain total
        let pays = l(&r["payments"]);
        let cash = pays.iter().map(m).find(|p| p["cash"] == Value::Bool(true)).unwrap();
        assert_eq!((cash["total"].as_f64(), cash["final_count"].as_f64(), cash["money_counted"].as_f64(), cash["money_difference"].as_f64()), (Some(19.8), Some(64.8), Some(64.8), Some(0.0)));
        let names: Vec<String> = l(&cash["cash_moves"]).iter().map(|x| text(m(x), "name").unwrap()).collect();
        assert_eq!(names[0], "Cash Opening"); assert_eq!(l(&cash["cash_moves"]).len(), 2);
        assert!(text(cash, "name").unwrap().starts_with("Cash POS/"));
        let card = pays.iter().map(m).find(|p| p["cash"] == Value::Bool(false)).unwrap(); assert_eq!(card["total"].as_f64(), Some(22.0));
        // invoices and totals per session
        let inv = l(&r["invoiceList"]); assert_eq!(l(&m(&inv[0])["invoices"]).len(), 1);
        assert_eq!((r["invoiceTotal"].as_f64(), r["total_paid"].as_f64()), (Some(22.0), Some(41.8)));
        assert_eq!(r["cash_rounding_total"].as_f64(), Some(0.0));
        assert_eq!(l(&r["payments_per_method"]).len(), 2);

        // by period instead of by session: nothing yesterday, everything today; per method totals are shown
        let all = call(&env, REPORT, "get_sale_details", &[], row(&[("config_ids", Value::List(vec![f.cfg.into()]))]))?;
        assert_eq!(m(&all)["nbr_orders"].as_i64(), Some(4)); assert_eq!(m(&all)["show_payment_per_method"], Value::Bool(true));
        let none = call(&env, REPORT, "get_sale_details", &[], row(&[("config_ids", Value::List(vec![f.cfg.into()])), ("date_start", "2000-01-01 00:00:00".into()), ("date_stop", "2000-01-01 23:59:59".into())]))?;
        assert_eq!(m(&none)["nbr_orders"].as_i64(), Some(0));
        // helpers
        let dr = call(&env, REPORT, "_get_date_start_and_date_stop", &[], row(&[("date_start", "2026-03-01 08:00:00".into()), ("date_stop", "2026-02-01 00:00:00".into())]))?;
        assert_eq!(l(&dr), &vec![Value::Text("2026-03-01 08:00:00".into()), Value::Text("2026-03-02 07:59:59".into())], "a stop before the start is replaced by start + 1 day - 1 s");
        let vals = call(&env, REPORT, "_get_report_values", &[s], row(&[("data", Value::Map(Row::new()))]))?;
        assert_eq!(m(&vals)["nbr_orders"].as_i64(), Some(4), "called with a session id from the POS");
        let line = ids(&rec(&env, "pos.order", a)?, "lines")[0];
        assert_eq!(call(&env, REPORT, "_get_product_total_amount", &[line], Row::new())?.as_f64(), Some(20.0));
        Ok(())
    }).unwrap();
}
