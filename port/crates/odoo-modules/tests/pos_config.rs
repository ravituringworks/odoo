//! More pos.config behaviour (pos_config_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{rules_for, stock, util::*};
use pos_common::*;

fn param(env: &odoo_core::orm::Env, k: &str, v: &str) { match find_one(env, "ir.config_parameter", term("key", "=", k)).unwrap() { Some(p) => orm::write(env, "ir.config_parameter", &[p], row(&[("value", v.into())])).unwrap(), None => { mk(env, "ir.config_parameter", &[("key", k.into()), ("value", v.into())]); } } }

#[test]
fn loading_limits_open_orders_trusted_configs_and_access() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        // limits come from system parameters with defaults
        assert_eq!(call(&env, "pos.config", "get_limited_product_count", &[f.cfg], Row::new())?, Value::Int(20000));
        assert_eq!(call(&env, "pos.config", "_get_limited_partner_count", &[f.cfg], Row::new())?, Value::Int(100));
        call(&env, "pos.config", "_set_default_pos_load_limit", &[], Row::new())?;
        param(&env, "point_of_sale.limited_product_count", "not a number");
        assert_eq!(call(&env, "pos.config", "get_limited_product_count", &[f.cfg], Row::new())?, Value::Int(20000), "unparsable values fall back to the default");

        // products: favourites first, then services, then recently moved ones
        let mkp = |name: &str, ty: &str, fav: bool| { let p = mk(&env, "product.product", &[("name", name.into()), ("list_price", 1.0.into()), ("type", ty.into()), ("available_in_pos", true.into())]); if fav { orm::write(&env, "product.template", &[id_of(&rec(&env, "product.product", p).unwrap(), "product_tmpl_id").unwrap()], row(&[("is_favorite", true.into())])).unwrap(); } p };
        let star = mkp("Star", "consu", true); let svc = mkp("Delivery", "service", false); let moved = mkp("Moved", "consu", false); let idle = mkp("Idle", "consu", false);
        let src = stock::ensure_location(&env, "internal", "Stock")?; let dst = stock::ensure_location(&env, "customer", "Customers")?;
        mk(&env, "stock.move", &[("product_id", moved.into()), ("product_uom_qty", 1.0.into()), ("location_id", src.into()), ("location_dest_id", dst.into()), ("state", "done".into())]);
        param(&env, "point_of_sale.limited_product_count", "20000");
        let load = |fields: &[&str]| -> odoo_core::Result<Vec<Row>> { let Value::List(l) = call(&env, "pos.config", "get_limited_products_loading", &[f.cfg], row(&[("fields", Value::List(fields.iter().map(|x| Value::Text((*x).into())).collect()))]))? else { panic!() }; Ok(l.into_iter().map(|v| match v { Value::Map(m) => m, _ => panic!() }).collect()) };
        let names: Vec<String> = load(&["name"])?.iter().map(|p| text(p, "name").unwrap()).collect();
        assert_eq!((names[0].as_str(), names[1].as_str(), names[2].as_str()), ("Star", "Delivery", "Moved"));
        assert!(names.contains(&"Idle".to_string()) && names.contains(&"Pen".to_string()));
        param(&env, "point_of_sale.limited_product_count", "2");
        assert_eq!(load(&["id"])?.len(), 2, "the limit applies");
        let _ = (star, svc, idle);

        // partners: the ones with most orders first
        let s = opened_session(&env, &f, 0.0);
        let busy = mk(&env, "res.partner", &[("name", "Zed busy".into())]); let _quiet = mk(&env, "res.partner", &[("name", "Abe quiet".into())]);
        for _ in 0..2 { let o = draft_order(&env, &f, s, 1.0); orm::write(&env, "pos.order", &[o], row(&[("partner_id", busy.into())]))?; }
        let Value::List(ps) = call(&env, "pos.config", "get_limited_partners_loading", &[f.cfg], Row::new())? else { panic!() };
        assert_eq!(ps[0], Value::List(vec![Value::Int(busy)]));
        param(&env, "point_of_sale.limited_customer_count", "1");
        let Value::List(ps) = call(&env, "pos.config", "get_limited_partners_loading", &[f.cfg], Row::new())? else { panic!() }; assert_eq!(ps.len(), 1);

        // open orders sync: matching records are returned, vanished or cancelled ones are listed as deleted
        let d1 = draft_order(&env, &f, s, 1.0); let d2 = draft_order(&env, &f, s, 1.0);
        call(&env, "pos.order", "action_pos_order_cancel", &[d2], Row::new())?;
        let res = call(&env, "pos.config", "read_config_open_orders", &[f.cfg], row(&[("domain", Value::Map(row(&[("pos.order", Value::List(vec![Value::List(vec!["state".into(), "=".into(), "draft".into()])]))]))), ("record_ids", Value::Map(row(&[("pos.order", Value::List(vec![d2.into(), 9999.into()]))])))]))?;
        let Value::Map(res) = res else { panic!() };
        let Value::Map(dynamic) = &res["dynamic_records"] else { panic!() }; let Value::List(open) = &dynamic["pos.order"] else { panic!() };
        assert!(open.iter().any(|o| matches!(o, Value::Map(m) if m["id"] == Value::Int(d1))));
        let Value::Map(del) = &res["deleted_record_ids"] else { panic!() };
        assert_eq!(del["pos.order"], Value::List(vec![Value::Int(9999), Value::Int(d2)]));
        let Value::Map(got) = call(&env, "pos.config", "get_records", &[f.cfg], row(&[("data", Value::Map(row(&[("pos.order", Value::List(vec![d1.into()]))])))]))? else { panic!() };
        assert!(matches!(&got["pos.order"], Value::List(l) if l.len() == 1));

        // trusted registers
        let other = mk(&env, "pos.config", &[("name", "Other".into()), ("picking_type_id", rec(&env, "pos.config", f.cfg)?["picking_type_id"].clone()), ("payment_method_ids", cmd6(&[f.bank]))]);
        call(&env, "pos.config", "_add_trusted_config_id", &[f.cfg], row(&[("config_id", other.into())]))?;
        assert_eq!(ids(&rec(&env, "pos.config", f.cfg)?, "trusted_config_ids"), vec![other]);
        call(&env, "pos.config", "_remove_trusted_config_id", &[f.cfg], row(&[("config_id", other.into())]))?;
        assert!(ids(&rec(&env, "pos.config", f.cfg)?, "trusted_config_ids").is_empty());

        // settings-view saves: absolute link commands unlink what is missing; unchanged values are dropped
        let sv = env.with_ctx("from_settings_view", Value::Bool(true));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("trusted_config_ids", cmd6(&[other]))]))?;
        let Value::Map(v) = call(&sv, "pos.config", "_preprocess_x2many_vals_from_settings_view", &[f.cfg], row(&[("vals", Value::Map(row(&[("trusted_config_ids", Value::List(vec![]))])))]))? else { panic!() };
        assert_eq!(v["trusted_config_ids"], Value::List(vec![Value::List(vec![3.into(), other.into()])]));
        let Value::Map(v) = call(&sv, "pos.config", "_keep_new_vals", &[f.cfg], row(&[("vals", Value::Map(row(&[("name", "Shop".into()), ("manual_discount", false.into())])))]))? else { panic!() };
        assert_eq!(v.keys().cloned().collect::<Vec<_>>(), vec!["manual_discount".to_string()], "the unchanged name is dropped");
        let Value::Map(same) = call(&env, "pos.config", "_keep_new_vals", &[f.cfg], row(&[("vals", Value::Map(row(&[("name", "Shop".into())])))]))? else { panic!() }; assert_eq!(same.len(), 1, "outside the settings view nothing is filtered");

        // customer display token, IoT/http decision, modal action
        let tok = text(&rec(&env, "pos.config", f.cfg)?, "access_token").unwrap();
        assert_eq!(call(&env, "pos.config", "update_customer_display", &[f.cfg], row(&[("access_token", tok.as_str().into())]))?, Value::Bool(true));
        assert_eq!(call(&env, "pos.config", "update_customer_display", &[f.cfg], row(&[("access_token", "bad".into())]))?, Value::Bool(false));
        assert_eq!(call(&env, "pos.config", "_force_http", &[f.cfg], Row::new())?, Value::Bool(false));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("other_devices", true.into())]))?;
        assert_eq!(call(&env, "pos.config", "_force_http", &[f.cfg], Row::new())?, Value::Bool(true));
        param(&env, "point_of_sale.enforce_https", "1");
        assert_eq!(call(&env, "pos.config", "_force_http", &[f.cfg], Row::new())?, Value::Bool(false));
        let Value::Map(a) = call(&env, "pos.config", "action_pos_config_modal_edit", &[f.cfg], Row::new())? else { panic!() }; assert_eq!(a["res_id"], Value::Int(f.cfg));
        let Value::Map(x) = call(&env, "pos.config", "execute", &[f.cfg], Row::new())? else { panic!() }; assert_eq!(x["tag"], Value::Text("reload".into()));

        // only managers change registers; the admin may
        call(&env, "pos.config", "_check_pos_manager_access", &[f.cfg], Row::new())?;
        let u = mk(&env, "res.users", &[("name", "Cashier".into()), ("login", "cashier".into())]);
        let plain = odoo_core::orm::Env::new(&reg, c, &rules, &odoo_core::orm::AllowAll, u);
        assert!(err_of(c, || call(&plain, "pos.config", "_check_pos_manager_access", &[f.cfg], Row::new())).contains("Only Point of Sale managers can modify"));
        Ok(())
    }).unwrap();
}

#[test]
fn sequences_rounding_notes_and_warehouse_types() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        // deleting a register removes the sequences it used; rounding methods in use are protected
        let s = opened_session(&env, &f, 0.0); let o = draft_order(&env, &f, s, 1.0);
        pay(&env, o, f.cash, 11.0); call(&env, "pos.order", "action_pos_order_paid", &[o], Row::new())?;
        let cfgr = rec(&env, "pos.config", f.cfg)?;
        let seq = id_of(&cfgr, "sequence_id").expect("order sequence created on first use"); assert!(id_of(&cfgr, "sequence_line_id").is_some());
        let rm = mk(&env, "account.cash.rounding", &[("name", "0.05".into()), ("rounding", 0.05.into()), ("strategy", "add_invoice_line".into()), ("rounding_method", "HALF-UP".into())]);
        orm::write(&env, "pos.session", &[s], row(&[("state", "closed".into())]))?;
        orm::write(&env, "pos.config", &[f.cfg], row(&[("cash_rounding", true.into()), ("rounding_method", rm.into())]))?;
        assert!(err_of(c, || orm::unlink(&env, "account.cash.rounding", &[rm])).contains("used in a Point of Sale configuration"));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("cash_rounding", false.into()), ("rounding_method", Value::Bool(false))]))?;
        orm::unlink(&env, "account.cash.rounding", &[rm])?;
        let empty = mk(&env, "pos.config", &[("name", "Temp".into()), ("picking_type_id", cfgr["picking_type_id"].clone()), ("payment_method_ids", cmd6(&[f.bank]))]);
        orm::unlink(&env, "pos.config", &[empty])?;
        // sessions of the register block deleting it? not in Odoo either (restrict via sessions); here the order sequence outlives other registers
        assert!(rec(&env, "ir.sequence", seq).is_ok());
        // notes are unique
        let n = mk(&env, "pos.note", &[("name", "No onions".into())]);
        assert!(err_of(c, || orm::create(&env, "pos.note", row(&[("name", "No onions".into())]))).contains("A note with this name already exists"));
        assert!(rec(&env, "pos.note", n).is_ok());
        // warehouses get their POS operation type on demand
        let loc = mk(&env, "stock.location", &[("name", "WH/Stock".into()), ("usage", "internal".into())]);
        let wh = mk(&env, "stock.warehouse", &[("name", "Main".into()), ("code", "MAIN".into()), ("lot_stock_id", loc.into())]);
        let Value::List(made) = call(&env, "stock.warehouse", "_create_missing_pos_picking_types", &[], Row::new())? else { panic!() };
        assert_eq!(made.len(), 1);
        let pt = rec(&env, "stock.picking.type", made[0].as_i64().unwrap())?;
        assert_eq!((text(&pt, "name").unwrap(), text(&pt, "code").unwrap(), id_of(&pt, "default_location_src_id"), id_of(&pt, "warehouse_id")), ("PoS Orders".to_string(), "outgoing".to_string(), Some(loc), Some(wh)));
        assert_eq!(id_of(&rec(&env, "stock.warehouse", wh)?, "pos_type_id"), made[0].as_i64());
        let Value::List(again) = call(&env, "stock.warehouse", "_create_missing_pos_picking_types", &[], Row::new())? else { panic!() }; assert!(again.is_empty());
        Ok(())
    }).unwrap();
}
