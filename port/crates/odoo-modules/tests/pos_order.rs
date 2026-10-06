//! pos.order / pos.order.line methods (pos_order_methods).
mod pos_common;
use odoo_core::{orm, store, Domain, Row, Value};
use odoo_modules::{rules_for, util::*};
use pos_common::*;

fn state(env: &odoo_core::orm::Env, o: i64) -> String { text(&rec(env, "pos.order", o).unwrap(), "state").unwrap() }

#[test]
fn order_pricing_paying_cancelling_and_name() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        let o = draft_order(&env, &f, s, 3.0);
        let r = rec(&env, "pos.order", o)?;
        // company, pricelist defaults come from the session's config; amounts from the lines
        assert!(id_of(&r, "company_id").is_some());
        assert_eq!((num(&r, "amount_total"), num(&r, "amount_tax"), num(&r, "amount_paid"), num(&r, "amount_difference")), (33.0, 3.0, 0.0, -33.0));
        let l = ids(&r, "lines")[0]; let lr = rec(&env, "pos.order.line", l)?;
        assert_eq!((num(&lr, "price_subtotal"), num(&lr, "price_subtotal_incl")), (30.0, 33.0));
        assert!(text(&lr, "name").unwrap().starts_with("Shop/"), "line named by the config's line sequence");
        // computes
        assert_eq!(rd(&env, "pos.order", o, "tracking_number"), Value::Text(format!("{:03}", (s % 10) * 100 + 1)));
        assert_eq!(rd(&env, "pos.order", o, "is_invoiced"), Value::Bool(false));
        assert_eq!(rd(&env, "pos.order", o, "has_refundable_lines"), Value::Bool(true));
        assert_eq!(rd(&env, "pos.order.line", l, "refunded_qty").as_f64(), Some(0.0));
        assert_eq!(rd(&env, "pos.order", o, "refund_orders_count"), Value::Int(0));
        assert_eq!(rd(&env, "pos.order", o, "picking_count"), Value::Int(0));
        assert_eq!(rd(&env, "pos.order", o, "currency_rate").as_f64(), Some(1.0));
        assert_eq!(rd(&env, "pos.order", o, "is_edited"), Value::Bool(false));
        assert_eq!(rd(&env, "pos.order", o, "is_total_cost_computed"), Value::Bool(false));
        assert_eq!(rd(&env, "pos.order", o, "margin").as_f64(), Some(0.0));
        assert!(rid(&rd(&env, "pos.order", o, "currency_id")).is_some(), "order currency reads through the config");
        // not fully paid -> refused; underpaid payments are refused after the fact
        pay(&env, o, f.cash, 20.0);
        assert_eq!(num(&rec(&env, "pos.order", o)?, "amount_paid"), 20.0);
        assert_eq!(call(&env, "pos.order", "_is_pos_order_paid", &[o], Row::new())?, Value::Bool(false));
        assert!(err_of(c, || call(&env, "pos.order", "action_pos_order_paid", &[o], Row::new())).contains("is not fully paid"));
        pay(&env, o, f.bank, 13.0);
        assert_eq!(call(&env, "pos.order", "_is_pos_order_paid", &[o], Row::new())?, Value::Bool(true));
        assert_eq!(text(&rec(&env, "pos.order", o)?, "name").unwrap(), "/");
        call(&env, "pos.order", "action_pos_order_paid", &[o], Row::new())?;
        assert_eq!(state(&env, o), "paid");
        assert_eq!(text(&rec(&env, "pos.order", o)?, "name").unwrap(), "Shop/0001", "the paid order takes the next number of the config sequence");
        // a paid order cannot be deleted and its payments cannot be edited into an underpayment
        assert!(err_of(c, || orm::unlink(&env, "pos.order", &[o])).contains("must be new or cancelled"));
        assert!(err_of(c, || orm::unlink(&env, "pos.order.line", &[l])).contains("new or cancelled state"));
        let pay_id = children(&env, "pos.payment", "pos_order_id", o)?[0]["id"].as_i64().unwrap();
        assert!(err_of(c, || orm::write(&env, "pos.order", &[o], row(&[("payment_ids", Value::List(vec![Value::List(vec![1.into(), pay_id.into(), Value::Map(row(&[("amount", 1.0.into())]))])]))]))).contains("paid amount is different"));
        // payment methods outside the config are refused on write
        let foreign = mk(&env, "pos.payment.method", &[("name", "Foreign".into())]);
        assert!(err_of(c, || orm::write(&env, "pos.payment", &[pay_id], row(&[("payment_method_id", foreign.into())]))).contains("not allowed in the config"));

        // draft orders can be cancelled or removed from the terminal; paid ones are untouched
        let d1 = draft_order(&env, &f, s, 1.0); let d2 = draft_order(&env, &f, s, 1.0);
        pay(&env, d2, f.cash, 5.0);
        let Value::Map(res) = call(&env, "pos.order", "action_pos_order_cancel", &[d1, o], Row::new())? else { panic!() };
        assert!(matches!(&res["pos.order"], Value::List(l) if l.len() == 1));
        assert_eq!((state(&env, d1), state(&env, o)), ("cancel".to_string(), "paid".to_string()));
        let gone = call(&env, "pos.order", "remove_from_ui", &[], row(&[("server_ids", Value::List(vec![d2.into(), o.into()]))]))?;
        assert_eq!(gone, Value::List(vec![Value::Int(d2)]));
        assert!(rec(&env, "pos.order", d2).is_err());
        assert!(children(&env, "pos.payment", "pos_order_id", d2)?.is_empty());
        orm::unlink(&env, "pos.order", &[d1])?;   // cancelled orders may be deleted
        Ok(())
    }).unwrap();
}

#[test]
fn cash_rounding_tolerance_and_rounded_amounts() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        let rm = mk(&env, "account.cash.rounding", &[("name", "0.05".into()), ("rounding", 0.05.into()), ("strategy", "add_invoice_line".into()), ("rounding_method", "HALF-UP".into())]);
        // a wrong strategy is refused
        let bad = mk(&env, "account.cash.rounding", &[("name", "bad".into()), ("rounding", 0.05.into()), ("strategy", "biggest_tax".into()), ("rounding_method", "HALF-UP".into())]);
        assert!(err_of(c, || orm::write(&env, "pos.config", &[f.cfg], row(&[("cash_rounding", true.into()), ("rounding_method", bad.into())]))).contains("cash rounding strategy of the point of sale Shop must be"));
        orm::write(&env, "pos.session", &[s], row(&[("state", "closed".into())]))?;   // allow the config to change
        orm::write(&env, "pos.config", &[f.cfg], row(&[("cash_rounding", true.into()), ("rounding_method", rm.into())]))?;
        let s = mk(&env, "pos.session", &[("config_id", f.cfg.into())]);
        // one pen at 10.00 + 10% = 11.00, a 0.03 line makes the total 11.03 which rounds to 11.05
        let o = draft_order(&env, &f, s, 1.0);
        let extra = row(&[("order_id", o.into()), ("product_id", f.pen.into()), ("qty", 1.0.into()), ("price_unit", 0.03.into()), ("price_subtotal", 0.03.into()), ("price_subtotal_incl", 0.03.into())]);
        mk(&env, "pos.order.line", &extra.iter().map(|(k, v)| (k.as_str(), v.clone())).collect::<Vec<_>>());
        call(&env, "pos.order", "_compute_prices", &[o], Row::new())?;
        assert_eq!(num(&rec(&env, "pos.order", o)?, "amount_total"), 11.05);
        assert_eq!(call(&env, "pos.order", "_get_rounded_amount", &[o], row(&[("amount", 11.03.into())]))?.as_f64(), Some(11.05));
        // 11.03 paid is within the tolerance (half a step, 0.03) of 11.05; 10.90 is not
        pay(&env, o, f.cash, 10.9);
        assert!(err_of(c, || call(&env, "pos.order", "action_pos_order_paid", &[o], Row::new())).contains("is not fully paid"));
        pay(&env, o, f.cash, 0.13);
        call(&env, "pos.order", "action_pos_order_paid", &[o], Row::new())?;
        assert_eq!(state(&env, o), "paid");
        Ok(())
    }).unwrap();
}

#[test]
fn invoice_refund_and_fiscal_position_mapping() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        let o = draft_order(&env, &f, s, 2.0);
        pay(&env, o, f.cash, 22.0);
        // invoicing needs a customer
        assert!(err_of(c, || call(&env, "pos.order", "action_pos_order_invoice", &[o], Row::new())).contains("Please provide a partner"));
        orm::write(&env, "pos.order", &[o], row(&[("partner_id", f.partner.into())]))?;
        let vals = call(&env, "pos.order", "_prepare_invoice_vals", &[o], Row::new())?;
        let Value::Map(v) = vals else { panic!() };
        assert_eq!((v["move_type"].clone(), v["partner_id"].clone()), (Value::Text("out_invoice".into()), Value::Int(f.partner)));
        let Value::Map(act) = call(&env, "pos.order", "action_pos_order_invoice", &[o], Row::new())? else { panic!() };
        let inv = act["res_id"].as_i64().unwrap();
        let order = rec(&env, "pos.order", o)?;
        assert_eq!((text(&order, "state").unwrap(), id_of(&order, "account_move")), ("invoiced".to_string(), Some(inv)));
        assert_eq!(rd(&env, "pos.order", o, "is_invoiced"), Value::Bool(true));
        let m = rec(&env, "account.move", inv)?;
        assert_eq!((text(&m, "state").unwrap(), text(&m, "move_type").unwrap(), num(&m, "amount_total")), ("posted".to_string(), "out_invoice".to_string(), 22.0));
        assert_eq!(num(&m, "amount_residual"), 0.0, "the cash payment settled the invoice");
        assert_eq!(text(&m, "ref").unwrap(), text(&order, "name").unwrap());
        // the payment was booked on the customer's receivable and linked to the payment
        let p = children(&env, "pos.payment", "pos_order_id", o)?;
        assert!(id_of(&p[0], "account_move_id").is_some());
        assert_eq!(rd(&env, "pos.order", o, "has_refundable_lines"), Value::Bool(true));

        // refund: the whole order comes back through a new draft order in the open session
        let Value::List(refs) = call(&env, "pos.order", "_refund", &[o], Row::new())? else { panic!() };
        let ro = refs[0].as_i64().unwrap();
        let rr = rec(&env, "pos.order", ro)?;
        assert_eq!((text(&rr, "state").unwrap(), num(&rr, "amount_total"), num(&rr, "amount_tax")), ("draft".to_string(), -22.0, -2.0));
        assert!(text(&rr, "name").unwrap().ends_with("REFUND"));
        let rl = children(&env, "pos.order.line", "order_id", ro)?;
        assert_eq!((num(&rl[0], "qty"), num(&rl[0], "price_subtotal_incl")), (-2.0, -22.0));
        assert_eq!(rd(&env, "pos.order.line", rl[0]["id"].as_i64().unwrap(), "price_subtotal").as_f64(), Some(-20.0));
        assert_eq!(rid(&rd(&env, "pos.order", ro, "refunded_order_id")), Some(o));
        assert_eq!(rd(&env, "pos.order", o, "refund_orders_count"), Value::Int(1));
        assert_eq!(rd(&env, "pos.order", o, "has_refundable_lines"), Value::Bool(false));
        assert_eq!(rd(&env, "pos.order.line", ids(&order, "lines")[0], "refunded_qty").as_f64(), Some(2.0));
        assert_eq!(call(&env, "pos.order.line", "isRefund", &[rl[0]["id"].as_i64().unwrap()], Row::new())?, Value::Bool(true));
        // paying the refund returns exactly what was paid; then it is invoiced as a credit note reversing the invoice
        assert_eq!(call(&env, "pos.order", "_is_pos_order_paid", &[ro], Row::new())?, Value::Bool(false));
        pay(&env, ro, f.cash, -22.0);
        assert_eq!(call(&env, "pos.order", "_is_pos_order_paid", &[ro], Row::new())?, Value::Bool(true));
        call(&env, "pos.order", "action_pos_order_paid", &[ro], Row::new())?;
        assert_eq!(text(&rec(&env, "pos.order", ro)?, "name").unwrap(), format!("{} REFUND", text(&order, "name").unwrap()));
        let Value::Map(act) = call(&env, "pos.order", "action_pos_order_invoice", &[ro], Row::new())? else { panic!() };
        let cn = rec(&env, "account.move", act["res_id"].as_i64().unwrap())?;
        assert_eq!((text(&cn, "move_type").unwrap(), id_of(&cn, "reversed_entry_id"), num(&cn, "amount_total")), ("out_refund".to_string(), Some(inv), 22.0));
        assert!(text(&cn, "ref").unwrap().starts_with("Reversal of: "));
        // nothing is left to refund and the session must be open to refund at all
        call(&env, "pos.session", "action_pos_session_closing_control", &[s], Row::new())?;
        assert!(err_of(c, || call(&env, "pos.order", "_refund", &[o], Row::new())).contains("you need to open a session in the POS Shop"));

        // fiscal position mapping: 10% VAT -> 5% VAT on lines of orders with that position
        let s2 = opened_session(&env, &f, 0.0);
        let t5 = mk(&env, "account.tax", &[("name", "VAT 5%".into()), ("amount", 5.0.into()), ("amount_type", "percent".into()), ("type_tax_use", "sale".into())]);
        let fp = mk(&env, "account.fiscal.position", &[("name", "Reduced".into())]);
        mk(&env, "account.fiscal.position.tax", &[("position_id", fp.into()), ("tax_src_id", f.tax.into()), ("tax_dest_id", t5.into())]);
        let o2 = draft_order(&env, &f, s2, 2.0);
        orm::write(&env, "pos.order", &[o2], row(&[("fiscal_position_id", fp.into())]))?;
        let line = ids(&rec(&env, "pos.order", o2)?, "lines")[0];
        assert_eq!(call(&env, "pos.order.line", "_get_tax_ids_after_fiscal_position", &[line], Row::new())?, Value::List(vec![Value::Int(t5)]));
        call(&env, "pos.order.line", "_onchange_amount_line_all", &[line], Row::new())?;
        call(&env, "pos.order", "_compute_prices", &[o2], Row::new())?;
        let r2 = rec(&env, "pos.order", o2)?;
        assert_eq!((num(&r2, "amount_total"), num(&r2, "amount_tax")), (21.0, 1.0));
        // a discounted line reports the discount in tax-included terms
        orm::write(&env, "pos.order.line", &[line], row(&[("discount", 50.0.into())]))?;
        call(&env, "pos.order.line", "_onchange_amount_line_all", &[line], Row::new())?;
        assert_eq!(call(&env, "pos.order.line", "_get_discount_amount", &[line], Row::new())?.as_f64(), Some(10.5));
        Ok(())
    }).unwrap();
}

#[test]
fn invoicing_after_the_session_closed_reverses_the_closing_entry() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        let o = draft_order(&env, &f, s, 2.0); pay(&env, o, f.cash, 22.0); call(&env, "pos.order", "action_pos_order_paid", &[o], Row::new())?;
        call(&env, "pos.session", "post_closing_cash_details", &[s], row(&[("counted_cash", 22.0.into())]))?;
        call(&env, "pos.session", "close_session_from_ui", &[s], Row::new())?;
        assert_eq!(state(&env, o), "done");
        let closing = id_of(&rec(&env, "pos.session", s)?, "move_id").expect("closing entry");
        let before = orm::search(&env, "account.move", &Domain::True, None, None, 0)?.len();
        // the customer asks for an invoice later
        orm::write(&env, "pos.order", &[o], row(&[("partner_id", f.partner.into())]))?;
        let Value::Map(act) = call(&env, "pos.order", "action_pos_order_invoice", &[o], Row::new())? else { panic!() };
        let inv = rec(&env, "account.move", act["res_id"].as_i64().unwrap())?;
        assert_eq!((text(&inv, "state").unwrap(), num(&inv, "amount_total"), num(&inv, "amount_residual")), ("posted".to_string(), 22.0, 0.0));
        assert_eq!(state(&env, o), "invoiced");
        // a reversal entry took the order out of the closing entry: sales and taxes debited, payments credited
        let rev = orm::search(&env, "account.move", &term("ref", "ilike", "Reversal of POS closing entry"), None, None, 0)?;
        assert_eq!(rev.len(), 1);
        let rm = rec(&env, "account.move", rev[0])?;
        assert!(text(&rm, "ref").unwrap().contains(&text(&rec(&env, "account.move", closing)?, "name").unwrap()) && text(&rm, "ref").unwrap().contains(&text(&rec(&env, "pos.order", o)?, "name").unwrap()));
        assert_eq!(text(&rm, "state").unwrap(), "posted");
        assert_eq!(id_of(&rm, "reversed_pos_order_id"), Some(o));
        let ml = children(&env, "account.move.line", "move_id", rev[0])?;
        let (d, cr): (f64, f64) = ml.iter().fold((0.0, 0.0), |a, l| (a.0 + num(l, "debit"), a.1 + num(l, "credit")));
        assert_eq!((d, cr), (22.0, 22.0));
        assert!(ml.iter().any(|l| num(l, "debit") == 20.0) && ml.iter().any(|l| num(l, "debit") == 2.0) && ml.iter().any(|l| num(l, "credit") == 22.0));
        // invoice + payment move + reversal entry were added
        assert!(orm::search(&env, "account.move", &Domain::True, None, None, 0)?.len() >= before + 3);
        // an order whose session is still open is not reversed
        let s2 = opened_session(&env, &f, 0.0); let o2 = draft_order(&env, &f, s2, 1.0); pay(&env, o2, f.cash, 11.0); call(&env, "pos.order", "action_pos_order_paid", &[o2], Row::new())?;
        orm::write(&env, "pos.order", &[o2], row(&[("partner_id", f.partner.into())]))?;
        call(&env, "pos.order", "action_pos_order_invoice", &[o2], Row::new())?;
        assert_eq!(orm::search(&env, "account.move", &term("ref", "ilike", "Reversal of POS closing entry"), None, None, 0)?.len(), 1);
        Ok(())
    }).unwrap();
}
