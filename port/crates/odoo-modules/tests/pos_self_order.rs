//! pos_self_order and pos_discount rules (pos_self_order_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{rules_for, util::*};
use pos_common::*;

const MODS: [&str; 5] = ["point_of_sale", "pos_self_order", "pos_restaurant", "pos_discount", "account"];

#[test]
fn self_order_urls_modes_tokens_and_links() {
    let (reg, st) = setup(&MODS); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        let txt = |v: Value| match v { Value::Text(t) => t, other => panic!("{other:?}") };
        let token = text(&rec(&env, "pos.config", f.cfg)?, "access_token").unwrap();
        // mode "nothing": still addressable; consultation has no token; mobile with a table carries the table's identifier
        assert_eq!(rd(&env, "pos.config", f.cfg, "status"), Value::Text("inactive".into()));
        assert_eq!(txt(rd(&env, "pos.config", f.cfg, "self_ordering_url")), format!("http://localhost:8069/pos-self/{}?access_token={token}", f.cfg));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("self_ordering_mode", "consultation".into())]))?;
        assert_eq!(txt(call(&env, "pos.config", "_get_self_order_route", &[f.cfg], Row::new())?), format!("/pos-self/{}", f.cfg));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("module_pos_restaurant", true.into())]))?;
        let floor = ids(&rec(&env, "pos.config", f.cfg)?, "floor_ids")[0];
        let table = children(&env, "restaurant.table", "floor_id", floor)?[0].clone(); let tid = table["id"].as_i64().unwrap();
        let ident = text(&table, "identifier").expect("tables get a security token"); assert!(!ident.is_empty());
        orm::write(&env, "pos.config", &[f.cfg], row(&[("self_ordering_mode", "mobile".into()), ("self_ordering_service_mode", "table".into()), ("self_ordering_pay_after", "meal".into())]))?;
        let route = txt(call(&env, "pos.config", "_get_self_order_route", &[f.cfg], row(&[("table_id", tid.into())]))?);
        assert_eq!(route, format!("/pos-self/{}?access_token={token}&table_identifier={ident}", f.cfg));
        let url = txt(call(&env, "pos.config", "_get_self_order_url", &[f.cfg], row(&[("table_id", tid.into())]))?);
        assert_eq!(url, format!("http://localhost:8069/pos-self/{}%3Faccess_token%3D{token}%26table_identifier%3D{ident}", f.cfg), "werkzeug-style quoting keeps only / and :");
        let Value::Map(p) = call(&env, "pos.config", "preview_self_order_app", &[f.cfg], Row::new())? else { panic!() }; assert_eq!(p["type"], Value::Text("ir.actions.act_url".into()));
        // the "Order Now" link exists once
        let links = children(&env, "pos_self_order.custom_link", "url", 0).unwrap_or_default(); let _ = links;
        let l = orm::search(&env, "pos_self_order.custom_link", &term("url", "=", format!("/pos-self/{}/products", f.cfg).as_str()), None, None, 0)?;
        assert_eq!(l.len(), 1);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("name", "Shop".into()), ("self_ordering_mode", "mobile".into())]))?;
        assert_eq!(orm::search(&env, "pos_self_order.custom_link", &term("url", "=", format!("/pos-self/{}/products", f.cfg).as_str()), None, None, 0)?.len(), 1);
        assert_eq!(text(&rec(&env, "pos_self_order.custom_link", l[0])?, "link_html").unwrap(), "<a class=\"btn btn-primary w-100\">Order Now</a>");
        let ln = mk(&env, "pos_self_order.custom_link", &[("name", "A & <b>".into()), ("url", "/x".into()), ("style", "danger".into())]);
        assert_eq!(text(&rec(&env, "pos_self_order.custom_link", ln)?, "link_html").unwrap(), "<a class=\"btn btn-danger w-100\">A &amp; &lt;b&gt;</a>");

        // mode consistency: with meal-payment a mobile menu is table service; counter service or kiosk pays each order
        let r = rec(&env, "pos.config", f.cfg)?; assert_eq!((text(&r, "self_ordering_pay_after").unwrap(), text(&r, "self_ordering_service_mode").unwrap()), ("meal".to_string(), "table".to_string()));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("self_ordering_service_mode", "counter".into()), ("self_ordering_mode", "mobile".into())]))?;
        assert_eq!(text(&rec(&env, "pos.config", f.cfg)?, "self_ordering_pay_after").unwrap(), "each");
        orm::write(&env, "pos.config", &[f.cfg], row(&[("self_ordering_pay_after", "meal".into()), ("self_ordering_mode", "mobile".into()), ("self_ordering_service_mode", "table".into())]))?;
        assert!(err_of(c, || orm::write(&env, "pos.config", &[f.cfg], row(&[("self_ordering_mode", "kiosk".into())]))).contains("cash payment methods in kiosk mode"), "the register still has a cash method");
        orm::write(&env, "pos.config", &[f.cfg], row(&[("payment_method_ids", cmd6(&[f.bank, f.later]))]))?;
        orm::write(&env, "pos.config", &[f.cfg], row(&[("self_ordering_mode", "kiosk".into())]))?;
        assert_eq!(text(&rec(&env, "pos.config", f.cfg)?, "self_ordering_pay_after").unwrap(), "each");
        // a kiosk cannot take cash
        assert!(err_of(c, || orm::write(&env, "pos.config", &[f.cfg], row(&[("payment_method_ids", cmd6(&[f.cash, f.bank]))]))).contains("You cannot add cash payment methods in kiosk mode"));
        assert!(err_of(c, || call(&env, "pos.config", "_check_default_user", &[f.cfg], Row::new())).contains("default user must be a POS user"));

        // helpers: kiosk URL, custom button (recreated when removed), session sequence
        assert_eq!(txt(call(&env, "pos.config", "get_kiosk_url", &[f.cfg], Row::new())?), format!("http://localhost:8069/pos-self/{}?access_token={}", f.cfg, text(&rec(&env, "pos.config", f.cfg)?, "access_token").unwrap()));
        let link = orm::search(&env, "pos_self_order.custom_link", &term("url", "=", format!("/pos-self/{}/products", f.cfg).as_str()), None, None, 0)?;
        orm::unlink(&env, "pos_self_order.custom_link", &link)?;
        call(&env, "pos.config", "_prepare_self_order_custom_btn", &[f.cfg], Row::new())?;
        assert_eq!(orm::search(&env, "pos_self_order.custom_link", &term("url", "=", format!("/pos-self/{}/products", f.cfg).as_str()), None, None, 0)?.len(), 1);
        let tmp = mk(&env, "pos.session", &[("config_id", f.cfg.into())]);
        orm::unlink(&env, "ir.sequence", &orm::search(&env, "ir.sequence", &term("code", "=", format!("pos.order_{tmp}").as_str()), None, None, 0)?)?;
        call(&env, "pos.session", "_create_pos_self_sessions_sequence", &[tmp], Row::new())?;
        assert!(find_one(&env, "ir.sequence", term("code", "=", format!("pos.order_{tmp}").as_str()))?.is_some());
        orm::write(&env, "pos.session", &[tmp], row(&[("state", "closed".into())]))?;
        call(&env, "restaurant.table", "_update_identifier", &[], Row::new())?;

        // QR code data: per-table codes for table service, six generic ones otherwise; split into rows
        orm::write(&env, "pos.config", &[f.cfg], row(&[("self_ordering_mode", "mobile".into()), ("self_ordering_service_mode", "table".into()), ("self_ordering_pay_after", "meal".into())]))?;
        let Value::List(qr) = call(&env, "pos.config", "_get_qr_code_data", &[f.cfg], Row::new())? else { panic!() };
        assert!(matches!(&qr[0], Value::Map(m) if m["type"] == Value::Text("table".into()) && matches!(&m["tables"], Value::List(t) if t.len() == 1)));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("self_ordering_service_mode", "counter".into())]))?;
        let Value::List(qr) = call(&env, "pos.config", "_get_qr_code_data", &[f.cfg], Row::new())? else { panic!() };
        assert!(matches!(&qr[0], Value::Map(m) if m["type"] == Value::Text("default".into()) && matches!(&m["tables"], Value::List(t) if t.len() == 6)));
        let Value::List(rows) = call(&env, "pos.config", "_split_qr_codes_list", &[f.cfg], row(&[("floors", Value::List(qr)), ("cols", 4.into())]))? else { panic!() };
        assert!(matches!(&rows[0], Value::Map(m) if matches!(&m["rows_of_tables"], Value::List(r) if r.len() == 2 && matches!(&r[0], Value::List(x) if x.len() == 4))));
        // new tokens invalidate old QR codes
        call(&env, "pos.config", "_update_access_token", &[f.cfg], Row::new())?;
        let new_token = text(&rec(&env, "pos.config", f.cfg)?, "access_token").unwrap(); assert!(new_token != token && new_token.len() == 16);
        assert!(text(&rec(&env, "restaurant.table", tid)?, "identifier").unwrap() != ident);

        // a session creates its own order sequence; the kiosk wizard opens one and closing drops unpaid orders
        let company = rec(&env, "pos.config", f.cfg)?["company_id"].as_i64().unwrap();
        orm::write(&env, "res.company", &[company], row(&[("chart_template", "generic_coa".into())]))?;
        orm::write(&env, "pos.config", &[f.cfg], row(&[("self_ordering_mode", "kiosk".into())]))?;
        let seqs = |env: &odoo_core::orm::Env| orm::search(env, "ir.sequence", &term("code", "ilike", "pos.order_"), None, None, 0).unwrap().len();
        let before = seqs(&env);
        let Value::Map(w) = call(&env, "pos.config", "action_open_wizard", &[f.cfg], Row::new())? else { panic!() };
        assert_eq!(w["tag"], Value::Text("install_kiosk_pwa".into()));
        assert_eq!(seqs(&env), before + 1);
        assert_eq!(rd(&env, "pos.config", f.cfg, "status"), Value::Text("active".into()));
        let sid = *ids(&rec(&env, "pos.config", f.cfg)?, "session_ids").last().unwrap();
        let sess = rec(&env, "pos.session", sid)?; assert_eq!(text(&sess, "state").unwrap(), "opened");
        assert!(find_one(&env, "ir.sequence", term("code", "=", format!("pos.order_{sid}").as_str()))?.is_some());
        // a second call reuses the session; an unpaid order is dropped on close, a paid one stays
        call(&env, "pos.config", "action_open_wizard", &[f.cfg], Row::new())?; assert_eq!(seqs(&env), before + 1);
        let unpaid = draft_order(&env, &f, sid, 1.0); let paid = draft_order(&env, &f, sid, 1.0);
        pay(&env, paid, f.bank, 11.0); call(&env, "pos.order", "action_pos_order_paid", &[paid], Row::new())?;
        call(&env, "pos.config", "action_close_kiosk_session", &[f.cfg], Row::new())?;
        assert!(rec(&env, "pos.order", unpaid).is_err() && rec(&env, "pos.order", paid).is_ok());
        assert_eq!(text(&rec(&env, "pos.session", sid)?, "state").unwrap(), "closed");
        Ok(())
    }).unwrap();
}

#[test]
fn category_hours_combo_parents_and_discount_product() {
    let (reg, st) = setup(&MODS); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        // availability hours must be sane
        assert!(err_of(c, || orm::create(&env, "pos.category", row(&[("name", "Late".into()), ("hour_until", 25.0.into())]))).contains("between 00:00 and 24:00"));
        assert!(err_of(c, || orm::create(&env, "pos.category", row(&[("name", "Odd".into()), ("hour_until", 8.0.into()), ("hour_after", 10.0.into())]))).contains("must be greater than Availability After"));
        let ok = mk(&env, "pos.category", &[("name", "Lunch".into()), ("hour_after", 11.0.into()), ("hour_until", 15.0.into())]);
        assert!(err_of(c, || orm::write(&env, "pos.category", &[ok], row(&[("hour_after", (-1.0).into())]))).contains("between 00:00 and 24:00"));

        // a combo child can name its parent by uuid
        let s = opened_session(&env, &f, 0.0); let o = draft_order(&env, &f, s, 1.0);
        let pl = mk(&env, "pos.order.line", &[("order_id", o.into()), ("product_id", f.pen.into()), ("qty", 1.0.into()), ("price_subtotal", 1.0.into()), ("price_subtotal_incl", 1.0.into()), ("uuid", "parent-1".into())]);
        let cl = mk(&env, "pos.order.line", &[("order_id", o.into()), ("product_id", f.pen.into()), ("qty", 1.0.into()), ("price_subtotal", 1.0.into()), ("price_subtotal_incl", 1.0.into()), ("combo_parent_uuid", "parent-1".into())]);
        assert_eq!(id_of(&rec(&env, "pos.order.line", cl)?, "combo_parent_id"), Some(pl));
        let other = mk(&env, "pos.order.line", &[("order_id", o.into()), ("product_id", f.pen.into()), ("qty", 1.0.into()), ("price_subtotal", 1.0.into()), ("price_subtotal_incl", 1.0.into())]);
        orm::write(&env, "pos.order.line", &[other], row(&[("combo_parent_uuid", "parent-1".into())]))?;
        assert_eq!(id_of(&rec(&env, "pos.order.line", other)?, "combo_parent_id"), Some(pl));
        orm::write(&env, "pos.session", &[s], row(&[("state", "closed".into())]))?;

        // global discount: the register needs its product before the first session
        let dp = mk(&env, "product.product", &[("name", "Discount".into()), ("list_price", 0.0.into()), ("type", "service".into())]);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("module_pos_discount", true.into())]))?;
        assert!(err_of(c, || call(&env, "pos.config", "open_ui", &[f.cfg], row(&[("opening_cash", 0.0.into())]))).contains("A discount product is needed to use the Global Discount feature"));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("discount_product_id", dp.into())]))?;
        let Value::Map(t) = call(&env, "pos.config", "open_ui", &[f.cfg], row(&[("opening_cash", 0.0.into())]))? else { panic!() };
        assert!(t.contains_key("session"), "the terminal runtime takes over once the product is set");
        assert_eq!(call(&env, "pos.config", "_get_special_products", &[f.cfg], Row::new())?, Value::List(vec![Value::Int(dp)]));
        // the settings screen proposes the register's discount product while the module is on
        let sid = mk(&env, "res.config.settings", &[("pos_config_id", f.cfg.into())]);
        assert_eq!(id_of(&rec(&env, "res.config.settings", sid)?, "pos_discount_product_id"), Some(dp));
        // module installation default: registers without an open session lose a product that cannot be the default one
        orm::write(&env, "pos.session", &children(&env, "pos.session", "config_id", f.cfg)?.iter().filter_map(|s| s["id"].as_i64()).collect::<Vec<_>>(), row(&[("state", "closed".into())]))?;
        call(&env, "pos.config", "_default_discount_value_on_module_install", &[], Row::new())?;
        assert!(id_of(&rec(&env, "pos.config", f.cfg)?, "discount_product_id").is_none(), "no default discount product is installed here");
        Ok(())
    }).unwrap();
}
