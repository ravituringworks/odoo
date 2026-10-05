//! pos.order.sync_from_ui and the helpers behind it (pos_order_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{rules_for, util::*};
use pos_common::*;

fn line(f: &Fx, uuid: &str, qty: f64) -> Value {
    Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("uuid", uuid.into()), ("product_id", f.pen.into()), ("qty", qty.into()), ("price_unit", 10.0.into()), ("price_subtotal", (qty * 10.0).into()), ("price_subtotal_incl", (qty * 11.0).into()), ("tax_ids", cmd6(&[f.tax]))]))])
}
fn payment(method: i64, amount: f64) -> Value { Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("payment_method_id", method.into()), ("amount", amount.into()), ("name", "pay".into())]))]) }
fn order_dict(f: &Fx, sid: i64, uuid: &str, qty: f64, pays: Vec<Value>, extra: &[(&str, Value)]) -> Value {
    let mut m = row(&[("session_id", sid.into()), ("name", format!("Order {uuid}").into()), ("uuid", uuid.into()), ("state", "paid".into()), ("amount_total", (qty * 11.0).into()), ("amount_tax", qty.into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()), ("lines", Value::List(vec![line(f, &format!("{uuid}-l1"), qty)])), ("payment_ids", Value::List(pays)), ("access_token", "x".into())]);
    for (k, v) in extra { m.insert((*k).into(), v.clone()); }
    Value::Map(m)
}
fn sync(env: &odoo_core::orm::Env, orders: Vec<Value>) -> odoo_core::Result<Value> { call(env, "pos.order", "sync_from_ui", &[], row(&[("orders", Value::List(orders))])) }

#[test]
fn sync_from_ui_pays_gives_change_ships_and_invoices() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        // 2 pens = 22, tendered 30 in cash, 8 change: the order is paid, the change is booked as a negative cash payment
        let res = sync(&env, vec![order_dict(&f, s, "u1", 2.0, vec![payment(f.cash, 30.0)], &[("amount_return", 8.0.into())])])?;
        let Value::Map(res) = res else { panic!() }; let Value::List(os) = &res["pos.order"] else { panic!() };
        let Value::Map(o1) = &os[0] else { panic!() }; let o1id = o1["id"].as_i64().unwrap();
        let o = rec(&env, "pos.order", o1id)?;
        assert_eq!((text(&o, "state").unwrap(), text(&o, "pos_reference").unwrap(), num(&o, "amount_paid"), num(&o, "amount_return")), ("paid".to_string(), "Order u1".to_string(), 22.0, 8.0));
        assert_eq!(text(&o, "name").unwrap(), "Shop/0001");
        let pays = children(&env, "pos.payment", "pos_order_id", o1id)?;
        assert_eq!(pays.len(), 2);
        let change = pays.iter().find(|p| num(p, "amount") == -8.0).expect("change payment");
        assert_eq!((flag(change, "is_change"), id_of(change, "payment_method_id")), (true, Some(f.cash)));
        // the goods left the stock: a picking linked to the order and session
        assert_eq!(ids(&o, "picking_ids").len(), 1);
        assert_eq!(rd(&env, "pos.order", o1id, "picking_count"), Value::Int(1));
        let pk = rec(&env, "stock.picking", ids(&o, "picking_ids")[0])?;
        assert_eq!((id_of(&pk, "pos_session_id"), text(&pk, "state").unwrap()), (Some(s), "done".to_string()));
        assert_eq!(rd(&env, "pos.session", s, "picking_count"), Value::Int(1));
        // syncing the same uuid again is ignored once the order is settled
        let again = sync(&env, vec![order_dict(&f, s, "u1", 5.0, vec![payment(f.cash, 55.0)], &[])])?;
        let Value::Map(again) = again else { panic!() }; let Value::List(a) = &again["pos.order"] else { panic!() };
        assert_eq!(a.len(), 1); assert_eq!(children(&env, "pos.order", "session_id", s)?.len(), 1);

        // an underpaid order stays a draft (the payment error is only logged, like Odoo)
        let res = sync(&env, vec![order_dict(&f, s, "u2", 2.0, vec![payment(f.cash, 10.0)], &[])])?;
        let Value::Map(res) = res else { panic!() }; let Value::List(os) = &res["pos.order"] else { panic!() };
        let Value::Map(o2) = &os[0] else { panic!() };
        assert_eq!(text(&rec(&env, "pos.order", o2["id"].as_i64().unwrap())?, "state").unwrap(), "draft");
        // a later sync of that draft completes it (existing order path: payments added, amounts recomputed)
        let upd = sync(&env, vec![order_dict(&f, s, "u2", 2.0, vec![payment(f.bank, 12.0)], &[("name", "Order u2".into()), ("lines", Value::List(vec![]))])])?;
        let Value::Map(upd) = upd else { panic!() }; let Value::List(os) = &upd["pos.order"] else { panic!() };
        let Value::Map(o2b) = &os[0] else { panic!() }; assert_eq!(o2b["id"], o2["id"]);
        let o2r = rec(&env, "pos.order", o2["id"].as_i64().unwrap())?;
        assert_eq!((text(&o2r, "state").unwrap(), num(&o2r, "amount_paid")), ("paid".to_string(), 22.0));

        // to_invoice: invoiced and the payment is booked against the invoice
        let res = sync(&env, vec![order_dict(&f, s, "u3", 1.0, vec![payment(f.bank, 11.0)], &[("partner_id", f.partner.into()), ("to_invoice", true.into())])])?;
        let Value::Map(res) = res else { panic!() }; let Value::List(os) = &res["pos.order"] else { panic!() };
        let Value::Map(o3) = &os[0] else { panic!() };
        let o3r = rec(&env, "pos.order", o3["id"].as_i64().unwrap())?;
        assert_eq!(text(&o3r, "state").unwrap(), "invoiced");
        let inv = rec(&env, "account.move", id_of(&o3r, "account_move").unwrap())?;
        assert_eq!((text(&inv, "state").unwrap(), num(&inv, "amount_total"), num(&inv, "amount_residual")), ("posted".to_string(), 11.0, 0.0));

        // a payment method outside the register is refused
        let foreign = mk(&env, "pos.payment.method", &[("name", "Foreign".into())]);
        assert!(err_of(c, || sync(&env, vec![order_dict(&f, s, "u4", 1.0, vec![payment(foreign, 11.0)], &[])])).contains("not allowed in the config"));
        // refunding lines from two different orders in one go is refused
        let l1 = children(&env, "pos.order.line", "order_id", o1id)?[0]["id"].as_i64().unwrap();
        let l3 = children(&env, "pos.order.line", "order_id", o3["id"].as_i64().unwrap())?[0]["id"].as_i64().unwrap();
        let mut bad = match order_dict(&f, s, "u5", 1.0, vec![], &[]) { Value::Map(m) => m, _ => unreachable!() };
        bad.insert("lines".into(), Value::List([l1, l3].iter().map(|l| Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("product_id", f.pen.into()), ("qty", (-1.0).into()), ("refunded_orderline_id", (*l).into())]))])).collect()));
        assert!(err_of(c, || sync(&env, vec![Value::Map(bad.clone())])).contains("only refund products from the same order"));
        let refunded = call(&env, "pos.order", "_get_refunded_orders", &[], row(&[("order", Value::Map(bad))]))?;
        assert_eq!(refunded, Value::List(vec![Value::Int(o1id), Value::Int(o3["id"].as_i64().unwrap())]));
        Ok(())
    }).unwrap();
}

#[test]
fn closed_session_redirects_orders_and_helpers_work() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        // the session is closing: with no other open session the order is refused
        orm::write(&env, "pos.session", &[s], row(&[("state", "closing_control".into())]))?;
        assert!(err_of(c, || sync(&env, vec![order_dict(&f, s, "late", 1.0, vec![payment(f.cash, 11.0)], &[])])).contains("No open session available"));
        let v = call(&env, "pos.order", "_get_valid_session", &[], row(&[("order", Value::Map(row(&[("session_id", s.into())])))]));
        assert!(v.is_err());
        // once the old session is closed and a new one is open, the order lands in the new one
        orm::write(&env, "pos.session", &[s], row(&[("state", "closed".into())]))?;
        let s2 = opened_session(&env, &f, 0.0);
        assert_eq!(call(&env, "pos.order", "_get_valid_session", &[], row(&[("order", Value::Map(row(&[("session_id", s.into())])))]))?, Value::Int(s2));
        let res = sync(&env, vec![order_dict(&f, s, "late", 1.0, vec![payment(f.cash, 11.0)], &[])])?;
        let Value::Map(res) = res else { panic!() }; let Value::List(os) = &res["pos.order"] else { panic!() };
        let Value::Map(o) = &os[0] else { panic!() }; assert_eq!(id_of(o, "session_id"), Some(s2));
        // a vanished partner is dropped together with the invoicing request
        let ghost = mk(&env, "res.partner", &[("name", "Ghost".into())]);
        orm::unlink(&env, "res.partner", &[ghost])?;
        let res = sync(&env, vec![order_dict(&f, s2, "g1", 1.0, vec![payment(f.cash, 11.0)], &[("partner_id", ghost.into()), ("to_invoice", true.into())])])?;
        let Value::Map(res) = res else { panic!() }; let Value::List(os) = &res["pos.order"] else { panic!() };
        let Value::Map(g) = &os[0] else { panic!() };
        let gr = rec(&env, "pos.order", g["id"].as_i64().unwrap())?;
        assert_eq!((text(&gr, "state").unwrap(), id_of(&gr, "partner_id"), flag(&gr, "to_invoice")), ("paid".to_string(), None, false));
        // _get_open_order finds a draft by uuid; helper actions
        assert_eq!(call(&env, "pos.order", "_get_open_order", &[], row(&[("order", Value::Map(row(&[("uuid", "g1".into())])))]))?, Value::Int(g["id"].as_i64().unwrap()));
        let rv = call(&env, "pos.order", "_prepare_refund_values", &[g["id"].as_i64().unwrap()], row(&[("current_session", s2.into())]))?;
        let Value::Map(rv) = rv else { panic!() }; assert_eq!((rv["amount_total"].as_f64(), rv["session_id"].clone()), (Some(-11.0), Value::Int(s2)));
        assert!(matches!(&rv["uuid"], Value::Text(u) if u.len() == 36));
        Ok(())
    }).unwrap();
}

#[test]
fn combo_children_must_belong_to_the_combo() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        let side = mk(&env, "product.product", &[("name", "Side".into()), ("list_price", 2.0.into()), ("type", "consu".into())]);
        let other = mk(&env, "product.product", &[("name", "Other".into()), ("list_price", 2.0.into()), ("type", "consu".into())]);
        let combo = mk(&env, "product.combo", &[("name", "Menu part".into())]);
        mk(&env, "product.combo.item", &[("combo_id", combo.into()), ("product_id", side.into())]);
        let menu = mk(&env, "product.product", &[("name", "Menu".into()), ("list_price", 8.0.into()), ("type", "combo".into())]);
        orm::write(&env, "product.template", &[id_of(&rec(&env, "product.product", menu)?, "product_tmpl_id").unwrap()], row(&[("combo_ids", cmd6(&[combo]))]))?;
        let mk_line = |id: i64, uuid: &str, pid: i64, kids: Option<Vec<i64>>| {
            let mut v = row(&[("id", id.into()), ("uuid", uuid.into()), ("product_id", pid.into()), ("qty", 1.0.into()), ("price_unit", 1.0.into()), ("price_subtotal", 1.0.into()), ("price_subtotal_incl", 1.0.into())]);
            if let Some(k) = kids { v.insert("combo_line_ids".into(), Value::List(k.into_iter().map(Value::Int).collect())); }
            Value::List(vec![0.into(), 0.into(), Value::Map(v)])
        };
        let dict = |child: i64| Value::Map(row(&[("session_id", s.into()), ("name", "m".into()), ("uuid", "m1".into()), ("state", "draft".into()), ("amount_total", 2.0.into()), ("amount_tax", 0.0.into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()),
            ("lines", Value::List(vec![mk_line(-1, "parent", menu, Some(vec![-2])), mk_line(-2, "child", child, None)]))]));
        assert!(err_of(c, || sync(&env, vec![dict(other)])).contains("combo choice 'Other' is no longer available"));
        let res = sync(&env, vec![dict(side)])?;
        let Value::Map(res) = res else { panic!() }; let Value::List(os) = &res["pos.order"] else { panic!() };
        let Value::Map(o) = &os[0] else { panic!() };
        let lines = children(&env, "pos.order.line", "order_id", o["id"].as_i64().unwrap())?;
        let parent = lines.iter().find(|l| text(l, "uuid").as_deref() == Some("parent")).unwrap(); let child = lines.iter().find(|l| text(l, "uuid").as_deref() == Some("child")).unwrap();
        assert_eq!(id_of(child, "combo_parent_id"), parent["id"].as_i64(), "the child is linked to its parent line");
        Ok(())
    }).unwrap();
}
