//! pos_sale: sale orders settled at the till (pos_sale_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{rules_for, util::*};
use pos_common::*;

fn sync(env: &odoo_core::orm::Env, order: Row) -> odoo_core::Result<Value> { call(env, "pos.order", "sync_from_ui", &[], row(&[("orders", Value::List(vec![Value::Map(order)]))])) }

#[test]
fn sale_orders_down_payments_team_and_deliveries() {
    let (reg, st) = setup(&["point_of_sale", "pos_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        let team = mk(&env, "crm.team", &[("name", "Shop team".into())]);
        let dp = mk(&env, "product.product", &[("name", "Down payment".into()), ("list_price", 0.0.into()), ("type", "service".into()), ("available_in_pos", true.into())]);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("crm_team_id", team.into()), ("down_payment_product_id", dp.into())]))?;
        let s = opened_session(&env, &f, 0.0);
        // team computes follow the register; special products include the down payment product
        assert_eq!(rd(&env, "crm.team", team, "pos_sessions_open_count"), Value::Int(1));
        assert_eq!(rd(&env, "crm.team", team, "pos_order_amount_total").as_f64(), Some(0.0));
        assert_eq!(call(&env, "pos.config", "_get_special_products", &[f.cfg], Row::new())?, Value::List(vec![Value::Int(dp)]));
        // new orders take the register's team; an emptied team falls back to it
        let o0 = draft_order(&env, &f, s, 2.0);
        assert_eq!(id_of(&rec(&env, "pos.order", o0)?, "crm_team_id"), Some(team));
        orm::write(&env, "pos.order", &[o0], row(&[("crm_team_id", Value::Bool(false))]))?;
        assert_eq!(id_of(&rec(&env, "pos.order", o0)?, "crm_team_id"), Some(team));
        assert_eq!(rd(&env, "crm.team", team, "pos_order_amount_total").as_f64(), Some(22.0));

        // a quotation of 3 pens (33) gets a 10 down payment at the till
        let so = mk(&env, "sale.order", &[("partner_id", f.partner.into()), ("order_line", Value::List(vec![Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("product_id", f.pen.into()), ("product_uom_qty", 3.0.into()), ("price_unit", 10.0.into()), ("tax_id", cmd6(&[f.tax]))]))])]))]);
        assert_eq!(num(&rec(&env, "sale.order", so)?, "amount_total"), 33.0);
        assert_eq!(rd(&env, "sale.order", so, "amount_unpaid").as_f64(), Some(33.0));
        let dp_line = Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("uuid", "dp1".into()), ("product_id", dp.into()), ("qty", 1.0.into()), ("price_unit", 10.0.into()), ("price_subtotal", 10.0.into()), ("price_subtotal_incl", 10.0.into()), ("sale_order_origin_id", so.into())]))]);
        let pay_line = Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("payment_method_id", f.cash.into()), ("amount", 10.0.into())]))]);
        let res = sync(&env, row(&[("session_id", s.into()), ("name", "Order dp".into()), ("uuid", "dp-order".into()), ("state", "paid".into()), ("partner_id", f.partner.into()), ("amount_total", 10.0.into()), ("amount_tax", 0.0.into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()), ("lines", Value::List(vec![dp_line])), ("payment_ids", Value::List(vec![pay_line]))]))?;
        let Value::Map(res) = res else { panic!() }; let Value::List(os) = &res["pos.order"] else { panic!() };
        let Value::Map(o) = &os[0] else { panic!() }; let oid = o["id"].as_i64().unwrap();
        assert_eq!(text(&rec(&env, "pos.order", oid)?, "state").unwrap(), "paid");
        // the sale order shows the down payment as a section and a zero-quantity line carrying the amount; the quotation is confirmed
        let sale_lines = children(&env, "sale.order.line", "order_id", so)?;
        assert!(sale_lines.iter().any(|l| text(l, "display_type").as_deref() == Some("line_section") && flag(l, "is_downpayment")));
        let dpl = sale_lines.iter().find(|l| flag(l, "is_downpayment") && text(l, "display_type").as_deref() != Some("line_section")).expect("down payment line");
        assert_eq!((num(dpl, "price_unit"), num(dpl, "product_uom_qty")), (10.0, 0.0));
        assert!(text(dpl, "name").unwrap().starts_with("Down payment (ref: "));
        let pol = children(&env, "pos.order.line", "order_id", oid)?;
        assert_eq!(id_of(&pol[0], "sale_order_line_id"), dpl["id"].as_i64());
        assert_eq!(text(&rec(&env, "sale.order", so)?, "state").unwrap(), "sale");
        // links and amounts
        assert_eq!(rd(&env, "sale.order", so, "pos_order_count"), Value::Int(1));
        assert_eq!(rd(&env, "pos.order", oid, "sale_order_count"), Value::Int(1));
        assert_eq!(rd(&env, "sale.order", so, "amount_unpaid").as_f64(), Some(23.0));
        let Value::Map(a) = call(&env, "pos.order", "action_view_sale_order", &[oid], Row::new())? else { panic!() };
        assert_eq!(a["res_model"], Value::Text("sale.order".into()));
        let Value::Map(b) = call(&env, "sale.order", "action_view_pos_order", &[so], Row::new())? else { panic!() };
        let Value::List(d) = &b["domain"] else { panic!() }; assert!(matches!(&d[0], Value::List(t) if t[2] == Value::List(vec![Value::Int(oid)])));
        // refunding the down payment lowers the sale line it created
        let rline = Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("uuid", "dp-r".into()), ("product_id", dp.into()), ("qty", (-1.0).into()), ("price_unit", 10.0.into()), ("price_subtotal", (-10.0).into()), ("price_subtotal_incl", (-10.0).into()), ("refunded_orderline_id", pol[0]["id"].clone())]))]);
        let rpay = Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("payment_method_id", f.cash.into()), ("amount", (-10.0).into())]))]);
        sync(&env, row(&[("session_id", s.into()), ("name", "Order dp refund".into()), ("uuid", "dp-refund".into()), ("state", "paid".into()), ("amount_total", (-10.0).into()), ("amount_tax", 0.0.into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()), ("lines", Value::List(vec![rline])), ("payment_ids", Value::List(vec![rpay]))]))?;
        assert_eq!(num(&rec(&env, "sale.order.line", dpl["id"].as_i64().unwrap())?, "price_unit"), 0.0);
        Ok(())
    }).unwrap();
}

#[test]
fn delivered_quantity_of_pos_lines_follows_the_picking() {
    let (reg, st) = setup(&["point_of_sale", "pos_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        let o = draft_order(&env, &f, s, 2.0);
        let l = ids(&rec(&env, "pos.order", o)?, "lines")[0];
        assert_eq!(num(&rec(&env, "pos.order.line", l)?, "qty_delivered"), 0.0);
        pay(&env, o, f.cash, 22.0);
        call(&env, "pos.order", "action_pos_order_paid", &[o], Row::new())?;
        assert_eq!(num(&rec(&env, "pos.order.line", l)?, "qty_delivered"), 0.0, "no outgoing picking yet");
        call(&env, "pos.order", "_create_order_picking", &[o], Row::new())?;
        assert_eq!(num(&rec(&env, "pos.order.line", l)?, "qty_delivered"), 2.0, "delivered in full once the picking is done");
        // optional products are offered with the product info
        let t = id_of(&rec(&env, "product.product", f.pen)?, "product_tmpl_id").unwrap();
        let acc = mk(&env, "product.template", &[("name", "Pen cap".into()), ("list_price", 1.5.into()), ("sale_ok", true.into()), ("available_in_pos", true.into())]);
        orm::write(&env, "product.template", &[t], row(&[("optional_product_ids", cmd6(&[acc]))]))?;
        assert_eq!(call(&env, "product.product", "has_optional_product_in_pos", &[f.pen], Row::new())?, Value::Bool(true));
        let Value::Map(i) = call(&env, "product.product", "get_product_info_pos", &[f.pen], row(&[("price", 10.0.into()), ("quantity", 1.0.into()), ("pos_config_id", f.cfg.into())]))? else { panic!() };
        let Value::List(opt) = &i["optional_products"] else { panic!() };
        assert!(matches!(&opt[0], Value::Map(m) if m["name"] == Value::Text("Pen cap".into()) && m["price"].as_f64() == Some(1.5)));
        Ok(())
    }).unwrap();
}
