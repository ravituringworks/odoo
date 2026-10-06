//! pos_restaurant back-office behaviour (`addons/pos_restaurant/models`): floor/table guards and management, the restaurant
//! defaults of a register, shared open table orders and the "last sent to kitchen" snapshot.
use crate::pos_methods::{flag, id_list, many, user_err};
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};

fn has_restaurant(env: &Env) -> bool { env.reg.model("restaurant.floor").is_ok() && env.reg.field("pos.config", "floor_ids").is_ok() }
fn first(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Select a record".into())) }

/// `pos.order._get_open_order`: restaurant registers also reuse the draft order of a table.
pub fn open_order(env: &Env, order: &Row) -> Result<Option<i64>> {
    let e = env.sudo(); let uuid = text(order, "uuid").unwrap_or_default();
    let by_uuid = || find_one(&e, "pos.order", term("uuid", "=", uuid.as_str()));
    let restaurant = has_restaurant(&e) && order.get("session_id").and_then(|s| s.as_i64()).and_then(|s| rec(&e, "pos.session", s).ok()).and_then(|s| id_of(&s, "config_id")).and_then(|c| rec(&e, "pos.config", c).ok()).map_or(false, |c| flag(&c, "module_pos_restaurant"));
    if !restaurant { return by_uuid(); }
    match (order.get("table_id").and_then(|t| t.as_i64()), text(order, "state").as_deref()) {
        (Some(t), Some("draft")) => Ok(orm::search(&e, "pos.order", &Domain::Or(vec![term("uuid", "=", uuid.as_str()), Domain::And(vec![term("table_id", "=", t), term("state", "=", "draft")])]), None, Some(1), 0)?.first().copied()),
        _ => by_uuid(),
    }
}
/// Draft orders of `tables` that the synced orders do not already cover (the terminal refreshes those tables too).
pub fn table_draft_orders(env: &Env, tables: &[i64], skip: &[i64]) -> Result<Vec<i64>> {
    Ok(orm::search(&env.sudo(), "pos.order", &Domain::And(vec![term("table_id", "in", id_list(tables)), term("state", "=", "draft")]), None, None, 0)?.into_iter().filter(|o| !skip.contains(o)).collect())
}

fn setup_default_floor(env: &Env, cfg: i64) -> Result<()> {
    let e = env.sudo(); let c = rec(&e, "pos.config", cfg)?;
    if !ids(&c, "floor_ids").is_empty() { return Ok(()); }
    let name = id_of(&c, "company_id").and_then(|x| rec(&e, "res.company", x).ok()).and_then(|x| text(&x, "name")).unwrap_or_default();
    let floor = orm::create(&e, "restaurant.floor", row(&[("name", name.into()), ("pos_config_ids", Value::List(vec![Value::List(vec![4.into(), cfg.into()])]))]))?;
    orm::create(&e, "restaurant.table", row(&[("table_number", 1.into()), ("floor_id", floor.into()), ("seats", 1.into()), ("position_h", 100.0.into()), ("position_v", 100.0.into()), ("width", 130.0.into()), ("height", 130.0.into())]))?;
    Ok(())
}

pub fn rules() -> Rules {
    Rules::default()
        // ---- register defaults ----
        .before_create("pos.config", |env, mut v| {
            if !has_restaurant(env) { return Ok(v); }
            let restaurant = flag(&v, "module_pos_restaurant");
            if restaurant && !v.contains_key("iface_splitbill") { v.insert("iface_splitbill".into(), true.into()); }
            // the static default is applied before hooks, so only an explicit true for the tip product keeps tips-after-payment
            if !restaurant || !flag(&v, "iface_tipproduct") { v.insert("set_tip_after_payment".into(), false.into()); }
            Ok(v)
        })
        .after_create("pos.config", |env, ids_, vals| { if has_restaurant(env) && flag(vals, "module_pos_restaurant") { for i in ids_ { setup_default_floor(env, *i)?; } } Ok(()) })
        .before_write("pos.config", |env, mut v| {
            if !has_restaurant(env) { return Ok(v); }
            if v.get("module_pos_restaurant").map_or(false, |x| !x.truthy()) && v.contains_key("module_pos_restaurant") { v.insert("floor_ids".into(), Value::List(vec![Value::List(vec![5.into()])])); }
            if (v.contains_key("module_pos_restaurant") && !flag(&v, "module_pos_restaurant")) || (v.contains_key("iface_tipproduct") && !flag(&v, "iface_tipproduct")) { v.insert("set_tip_after_payment".into(), false.into()); }
            Ok(v)
        })
        .after_write("pos.config", |env, ids_, vals| { if has_restaurant(env) && flag(vals, "module_pos_restaurant") { for i in ids_ { setup_default_floor(env, *i)?; } } Ok(()) })
        .action("pos.config", "_setup_default_floor", |env, ids_, _| { for i in ids_ { setup_default_floor(env, *i)?; } Ok(Value::Null) })
        // ---- floors ----
        .on_unlink("restaurant.floor", |env, ids_| {
            let e = env.sudo(); let mut msg = String::new();
            let mut confs: Vec<i64> = vec![];
            for f in ids_ { for c in ids(&rec(&e, "restaurant.floor", *f)?, "pos_config_ids") { if rec(&e, "pos.config", c).map_or(false, |r| flag(&r, "module_pos_restaurant")) && !confs.contains(&c) { confs.push(c); } } }
            let open = if confs.is_empty() { vec![] } else { orm::search(&e, "pos.session", &Domain::And(vec![term("config_id", "in", id_list(&confs)), term("state", "!=", "closed")]), None, None, 0)? };
            if !open.is_empty() {
                msg.push_str("You cannot remove a floor that is used in a PoS session, close the session(s) first: \n");
                for f in ids_ { for s in &open {
                    let cfg = rec(&e, "pos.config", id_of(&rec(&e, "pos.session", *s)?, "config_id").unwrap_or(0))?;
                    if ids(&cfg, "floor_ids").contains(f) { msg.push_str(&format!("Floor: {} - PoS Config: {} \n", text(&rec(&e, "restaurant.floor", *f)?, "name").unwrap_or_default(), text(&cfg, "name").unwrap_or_default())); }
                } }
                return user_err(msg);
            }
            Ok(vec![])
        })
        .after_write("restaurant.floor", |env, ids_, vals| {
            if !(flag(vals, "pos_config_ids") || flag(vals, "active")) { return Ok(()); }
            let e = env.sudo();
            for f in ids_ { for c in ids(&rec(&e, "restaurant.floor", *f)?, "pos_config_ids") {
                if !orm::search(&e, "pos.session", &Domain::And(vec![term("config_id", "=", c), term("state", "!=", "closed")]), None, Some(1), 0)?.is_empty() {
                    return user_err(format!("Please close and validate the following open PoS Session before modifying this floor.\nOpen session: {}", text(&rec(&e, "pos.config", c)?, "name").unwrap_or_default()));
                }
            } }
            Ok(())
        })
        .action("restaurant.floor", "rename_floor", |env, ids_, kw| { orm::write(&env.sudo(), "restaurant.floor", ids_, row(&[("name", kw.get("new_name").cloned().unwrap_or(Value::Null))]))?; Ok(Value::Null) })
        .action("restaurant.floor", "sync_from_ui", |env, _, kw| {
            let e = env.sudo(); let name = kw.get("name").cloned().unwrap_or(Value::Null);
            let mut v = row(&[("name", name)]); if let Some(c) = kw.get("background_color") { v.insert("background_color".into(), c.clone()); }
            let fid = orm::create(&e, "restaurant.floor", v)?;
            orm::write(&e, "restaurant.floor", &[fid], row(&[("pos_config_ids", Value::List(vec![Value::List(vec![4.into(), kw.get("config_id").cloned().unwrap_or(Value::Null)])]))]))?;
            let f = rec(&e, "restaurant.floor", fid)?;
            Ok(Value::Map(row(&[("id", fid.into()), ("name", f.get("name").cloned().unwrap_or(Value::Null)), ("background_color", f.get("background_color").cloned().unwrap_or(Value::Null)), ("table_ids", Value::List(vec![])), ("sequence", f.get("sequence").cloned().unwrap_or(Value::Null)), ("tables", Value::List(vec![]))])))
        })
        .action("restaurant.floor", "deactivate_floor", |env, ids_, kw| {
            let e = env.sudo(); let fid = first(ids_)?; let session = kw.get("session_id").and_then(|v| v.as_i64()).unwrap_or(0);
            if !orm::search(&e, "pos.order", &Domain::And(vec![term("session_id", "=", session), term("state", "=", "draft"), term("table_id.floor_id", "=", fid)]), None, Some(1), 0)?.is_empty() { return user_err("You cannot delete a floor when orders are still in draft for this floor."); }
            let tables = children(&e, "restaurant.table", "floor_id", fid)?.into_iter().filter_map(|t| t["id"].as_i64()).collect::<Vec<_>>();
            if !tables.is_empty() { orm::write(&e, "restaurant.table", &tables, row(&[("active", false.into())]))?; }
            orm::write(&e, "restaurant.floor", &[fid], row(&[("active", false.into())]))?;
            Ok(Value::Bool(true))
        })
        // ---- tables ----
        .action("restaurant.table", "are_orders_still_in_draft", |env, ids_, _| {
            if orm::search_count(&env.sudo(), "pos.order", &Domain::And(vec![term("table_id", "in", id_list(ids_)), term("state", "=", "draft")]))? > 0 { return user_err("You cannot delete a table when orders are still in draft for this table."); }
            Ok(Value::Bool(true))
        })
        .on_unlink("restaurant.table", |env, ids_| {
            let e = env.sudo(); let mut confs: Vec<i64> = vec![];
            for t in ids_ { if let Some(f) = id_of(&rec(&e, "restaurant.table", *t)?, "floor_id") { for c in ids(&rec(&e, "restaurant.floor", f)?, "pos_config_ids") { if rec(&e, "pos.config", c).map_or(false, |r| flag(&r, "module_pos_restaurant")) && !confs.contains(&c) { confs.push(c); } } } }
            if !confs.is_empty() && !orm::search(&e, "pos.session", &Domain::And(vec![term("config_id", "in", id_list(&confs)), term("state", "!=", "closed")]), None, Some(1), 0)?.is_empty() {
                return user_err("You cannot remove a table that is used in a PoS session, close the session(s) first.");
            }
            Ok(vec![])
        })
        // ---- orders / sessions ----
        .action("pos.order", "_get_open_order_restaurant", |env, _, kw| { let o = match kw.get("order") { Some(Value::Map(m)) => m.clone(), _ => Row::new() }; Ok(open_order(env, &o)?.map_or(Value::Bool(false), Value::Int)) })
        .action("pos.session", "_set_last_order_preparation_change", |env, _, kw| {
            let e = env.sudo(); let orders: Vec<i64> = match kw.get("order_ids") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] };
            for o in orders {
                let mut lines = serde_json::Map::new();
                for l in many(&e, "pos.order.line", &ids(&rec(&e, "pos.order", o)?, "lines"))? {
                    let uuid = text(&l, "uuid").unwrap_or_default();
                    lines.insert(format!("{uuid} - "), serde_json::json!({"uuid": uuid, "name": text(&l, "full_product_name"), "note": "", "product_id": id_of(&l, "product_id"), "quantity": num(&l, "qty"), "attribute_value_ids": ids(&l, "attribute_value_ids")}));
                }
                orm::write(&e, "pos.order", &[o], row(&[("last_order_preparation_change", serde_json::json!({"lines": lines, "generalNote": ""}).to_string().into())]))?;
            }
            Ok(Value::Null)
        })
}
