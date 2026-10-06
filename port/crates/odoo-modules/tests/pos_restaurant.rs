//! pos_restaurant back-office behaviour (pos_restaurant_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{rules_for, util::*};
use pos_common::*;

fn sync(env: &odoo_core::orm::Env, orders: Vec<Value>) -> odoo_core::Result<Value> { call(env, "pos.order", "sync_from_ui", &[], row(&[("orders", Value::List(orders))])) }
fn draft_dict(f: &Fx, sid: i64, uuid: &str, table: i64) -> Value {
    Value::Map(row(&[("session_id", sid.into()), ("name", format!("Table order {uuid}").into()), ("uuid", uuid.into()), ("state", "draft".into()), ("table_id", table.into()), ("amount_total", 11.0.into()), ("amount_tax", 1.0.into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()),
        ("lines", Value::List(vec![Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("uuid", format!("{uuid}-l").into()), ("product_id", f.pen.into()), ("qty", 1.0.into()), ("price_unit", 10.0.into()), ("price_subtotal", 10.0.into()), ("price_subtotal_incl", 11.0.into()), ("tax_ids", cmd6(&[f.tax]))]))])]))]))
}

#[test]
fn restaurant_register_defaults_floors_and_tables() {
    let (reg, st) = setup(&["point_of_sale", "pos_restaurant", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        let pt = rec(&env, "pos.config", f.cfg)?["picking_type_id"].clone();
        // a restaurant register gets bill splitting, no tip-after-payment without tips, and a default floor with one table
        let rest = mk(&env, "pos.config", &[("name", "Restaurant".into()), ("picking_type_id", pt.clone()), ("payment_method_ids", cmd6(&[f.bank])), ("module_pos_restaurant", true.into()), ("set_tip_after_payment", true.into())]);
        let r = rec(&env, "pos.config", rest)?;
        assert_eq!((flag(&r, "iface_splitbill"), flag(&r, "set_tip_after_payment")), (true, false));
        assert_eq!(ids(&r, "floor_ids").len(), 1);
        let floor = ids(&r, "floor_ids")[0];
        let tables = children(&env, "restaurant.table", "floor_id", floor)?;
        assert_eq!((tables.len(), num(&tables[0], "table_number"), num(&tables[0], "width")), (1, 1.0, 130.0));
        // the default floor is only created when the register has none
        call(&env, "pos.config", "_setup_default_floor", &[rest], Row::new())?;
        assert_eq!(ids(&rec(&env, "pos.config", rest)?, "floor_ids").len(), 1);
        // a plain register stays plain; switching restaurant mode off drops the floors
        assert!(ids(&rec(&env, "pos.config", f.cfg)?, "floor_ids").is_empty());
        orm::write(&env, "pos.config", &[rest], row(&[("module_pos_restaurant", false.into())]))?;
        assert!(ids(&rec(&env, "pos.config", rest)?, "floor_ids").is_empty());
        orm::write(&env, "pos.config", &[rest], row(&[("module_pos_restaurant", true.into())]))?;
        assert_eq!(ids(&rec(&env, "pos.config", rest)?, "floor_ids").len(), 1, "enabling it again recreates the default floor");

        // floors: created from the terminal, renamed, linked to the register
        let Value::Map(nf) = call(&env, "restaurant.floor", "sync_from_ui", &[], row(&[("name", "Terrace".into()), ("background_color", "rgb(1,2,3)".into()), ("config_id", rest.into())]))? else { panic!() };
        let tf = nf["id"].as_i64().unwrap();
        assert_eq!(nf["name"], Value::Text("Terrace".into()));
        assert!(ids(&rec(&env, "pos.config", rest)?, "floor_ids").contains(&tf));
        call(&env, "restaurant.floor", "rename_floor", &[tf], row(&[("new_name", "Patio".into())]))?;
        assert_eq!(text(&rec(&env, "restaurant.floor", tf)?, "name").unwrap(), "Patio");
        let t2 = mk(&env, "restaurant.table", &[("table_number", 2.into()), ("floor_id", tf.into())]);

        // with a session open the floors are frozen
        let s = opened_session_for(&env, rest);
        assert!(err_of(c, || orm::unlink(&env, "restaurant.floor", &[tf])).contains("You cannot remove a floor that is used in a PoS session"));
        let e = err_of(c, || orm::unlink(&env, "restaurant.floor", &[tf]));
        assert!(e.contains("Floor: Patio - PoS Config: Restaurant"), "{e}");
        assert!(err_of(c, || orm::unlink(&env, "restaurant.table", &[t2])).contains("You cannot remove a table that is used in a PoS session"));
        assert!(err_of(c, || orm::write(&env, "restaurant.floor", &[tf], row(&[("active", true.into())]))).contains("Please close and validate the following open PoS Session"));
        assert!(err_of(c, || orm::write(&env, "pos.config", &[rest], row(&[("floor_ids", cmd6(&[]))]))).contains("Floor"), "floor_ids is frozen with the session");
        // draft orders on a table block deactivating its floor or deleting the table
        let o = mk(&env, "pos.order", &[("session_id", s.into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()), ("amount_tax", 0.0.into()), ("amount_total", 0.0.into()), ("table_id", t2.into())]);
        assert!(err_of(c, || call(&env, "restaurant.table", "are_orders_still_in_draft", &[t2], Row::new())).contains("orders are still in draft for this table"));
        assert!(err_of(c, || call(&env, "restaurant.floor", "deactivate_floor", &[tf], row(&[("session_id", s.into())]))).contains("orders are still in draft for this floor"));
        call(&env, "pos.order", "action_pos_order_cancel", &[o], Row::new())?;
        call(&env, "restaurant.table", "are_orders_still_in_draft", &[t2], Row::new())?;
        orm::write(&env, "pos.session", &[s], row(&[("state", "closed".into())]))?;
        call(&env, "restaurant.floor", "deactivate_floor", &[tf], row(&[("session_id", s.into())]))?;
        let fr = rec(&env, "restaurant.floor", tf)?; assert!(!flag(&fr, "active"));
        assert!(!flag(&rec(&env, "restaurant.table", t2)?, "active"), "its tables are deactivated with it");
        Ok(())
    }).unwrap();
}

fn opened_session_for(env: &odoo_core::orm::Env, cfg: i64) -> i64 {
    let s = mk(env, "pos.session", &[("config_id", cfg.into())]);
    call(env, "pos.session", "set_opening_control", &[s], row(&[("cashbox_value", 0.0.into()), ("notes", "".into())])).unwrap();
    s
}

#[test]
fn table_orders_are_shared_between_terminals() {
    let (reg, st) = setup(&["point_of_sale", "pos_restaurant", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("module_pos_restaurant", true.into())]))?;
        let floor = ids(&rec(&env, "pos.config", f.cfg)?, "floor_ids")[0];
        let t1 = children(&env, "restaurant.table", "floor_id", floor)?[0]["id"].as_i64().unwrap();
        let t2 = mk(&env, "restaurant.table", &[("table_number", 2.into()), ("floor_id", floor.into())]);
        let s = opened_session_for(&env, f.cfg);
        // a draft saved by terminal A for table 1 ...
        let Value::Map(r1) = sync(&env, vec![draft_dict(&f, s, "A-1", t1)])? else { panic!() };
        let Value::List(os) = &r1["pos.order"] else { panic!() }; let Value::Map(o) = &os[0] else { panic!() }; let oid = o["id"].as_i64().unwrap();
        // ... is the same order when terminal B sends its draft of that table under another uuid
        let mut b1 = match draft_dict(&f, s, "B-1", t1) { Value::Map(m) => m, _ => unreachable!() }; b1.insert("lines".into(), Value::List(vec![]));   // B only re-sends what changed
        let Value::Map(r2) = sync(&env, vec![Value::Map(b1)])? else { panic!() };
        let Value::List(os) = &r2["pos.order"] else { panic!() }; let Value::Map(o2) = &os[0] else { panic!() };
        assert_eq!(o2["id"], Value::Int(oid));
        assert_eq!(children(&env, "pos.order", "table_id", t1)?.len(), 1);
        // another table is another order; the open orders of the tables a terminal works on come back with the sync
        let Value::Map(_) = sync(&env, vec![draft_dict(&f, s, "A-2", t2)])? else { panic!() };
        let working = env.with_ctx("table_ids", Value::List(vec![t1.into(), t2.into()]));
        let Value::Map(r3) = sync(&working, vec![draft_dict(&f, s, "A-3", t2)])? else { panic!() };
        let Value::List(os) = &r3["pos.order"] else { panic!() };
        assert_eq!(os.len(), 2, "the synced order of table 2 plus table 1's open order");
        let open = call(&env, "pos.order", "_get_open_order_restaurant", &[], row(&[("order", Value::Map(row(&[("session_id", s.into()), ("uuid", "zzz".into()), ("table_id", t1.into()), ("state", "draft".into())])))]))?;
        assert_eq!(open, Value::Int(oid));
        // not a restaurant register: the table is irrelevant, only the uuid counts
        // last sent snapshot for the kitchen
        call(&env, "pos.session", "_set_last_order_preparation_change", &[], row(&[("order_ids", Value::List(vec![oid.into()]))]))?;
        let snap = text(&rec(&env, "pos.order", oid)?, "last_order_preparation_change").unwrap();
        let j: serde_json::Value = serde_json::from_str(&snap).unwrap();
        let lines = j["lines"].as_object().unwrap(); assert_eq!(lines.len(), 1);
        let (k, v) = lines.iter().next().unwrap(); assert!(k.ends_with(" - ")); assert_eq!((v["quantity"].as_f64(), v["uuid"].as_str(), v["note"].as_str()), (Some(1.0), Some("A-1-l"), Some("")));   // the line has no full_product_name here, so name is null
        Ok(())
    }).unwrap();
}
