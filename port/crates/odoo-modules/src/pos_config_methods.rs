//! More `pos.config` behaviour (loading limits, settings-view preprocessing, trusted registers, access checks) plus
//! pos.note, account.cash.rounding and the warehouse's POS operation type (`addons/point_of_sale/models`).
//!
//! Not ported: bus notifications (`notify_synchronisation`, `update_customer_display` only validates the token) and the
//! automatic POS operation type at warehouse creation (call `_create_missing_pos_picking_types` explicitly).
use crate::pos_methods::{flag, has_model, id_list, invalid, many, user_err, xmlid};
use crate::{stock, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};
use std::collections::BTreeSet;

fn first(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Select a point of sale".into())) }
fn param_int(env: &Env, key: &str, default: i64) -> i64 {
    let e = env.sudo();
    find_one(&e, "ir.config_parameter", term("key", "=", key)).ok().flatten().and_then(|p| rec(&e, "ir.config_parameter", p).ok()).and_then(|p| text(&p, "value")).and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(default)
}
/// `env.is_admin()` or a member of the POS managers group.
pub(crate) fn is_pos_manager(env: &Env) -> bool {
    if env.uid <= 1 { return true; }
    let Ok(u) = rec(env, "res.users", env.uid) else { return false };
    let groups = ids(&u, "groups_id");
    ["group_pos_manager"].iter().filter_map(|g| xmlid(env, "point_of_sale", g)).any(|g| groups.contains(&g)) || xmlid(env, "base", "group_system").map_or(false, |g| groups.contains(&g))
}
pub(crate) fn check_manager_access(env: &Env) -> Result<()> {
    if is_pos_manager(env) { Ok(()) } else { user_err("Only Point of Sale managers can modify a Point of Sale configuration.") }
}

/// Partners the terminal preloads: the busiest customers of the company first.
fn limited_partners(env: &Env, cfg: &Row) -> Result<Vec<i64>> {
    let e = env.sudo(); let company = id_of(cfg, "company_id");
    let mut counts: std::collections::BTreeMap<i64, i64> = Default::default();
    for o in orm::search_read(&e, "pos.order", &Domain::True, &["partner_id".into()], None, None, 0)? { if let Some(p) = id_of(&o, "partner_id") { *counts.entry(p).or_default() += 1; } }
    let mut partners: Vec<Row> = vec![];
    for p in orm::search(&e, "res.partner", &Domain::True, None, None, 0)? {
        let r = rec(&e, "res.partner", p)?;
        if id_of(&r, "company_id").is_none() || id_of(&r, "company_id") == company { partners.push(r); }
    }
    partners.sort_by(|a, b| counts.get(&b["id"].as_i64().unwrap_or(0)).unwrap_or(&0).cmp(counts.get(&a["id"].as_i64().unwrap_or(0)).unwrap_or(&0)).then(text(a, "name").cmp(&text(b, "name"))));
    Ok(partners.into_iter().take(param_int(&e, "point_of_sale.limited_customer_count", 100).max(0) as usize).filter_map(|p| p["id"].as_i64()).collect())
}
/// The products a register preloads: favourites, services, recently moved, recently written; plus special and combo products.
fn limited_products(env: &Env, cfg_id: i64, fields: &[String]) -> Result<Vec<Row>> {
    let e = env.sudo();
    let dom_terms = match orm::call(&e, "pos.config", "_get_available_product_domain", &[cfg_id], &Row::new())? { Value::List(l) => l, _ => vec![] };
    let mut dom = vec![];
    for t in dom_terms { if let Value::List(x) = t { if x.len() == 3 { if let (Some(f), Some(op)) = (x[0].as_str(), x[1].as_str()) { dom.push(Domain::Term(f.into(), op.into(), x[2].clone())); } } } }
    let candidates = orm::search(&e, "product.product", &Domain::And(dom), None, None, 0)?;
    // last stock movement per product
    let mut last_move: std::collections::BTreeMap<i64, String> = Default::default();
    if has_model(&e, "stock.move") {
        for m in orm::search_read(&e, "stock.move", &Domain::True, &["product_id".into(), "write_date".into()], None, None, 0)? { if let (Some(p), Some(d)) = (id_of(&m, "product_id"), text(&m, "write_date")) { let en = last_move.entry(p).or_default(); if d > *en { *en = d; } } }
    }
    let mut rows: Vec<(bool, bool, Option<String>, String, i64)> = vec![];
    for p in candidates {
        let pr = rec(&e, "product.product", p)?; let t = id_of(&pr, "product_tmpl_id").and_then(|t| rec(&e, "product.template", t).ok()).unwrap_or_default();
        rows.push((flag(&t, "is_favorite"), text(&t, "type").as_deref() == Some("service"), last_move.get(&p).cloned(), text(&pr, "write_date").unwrap_or_default(), p));
    }
    // favourites, then services, then the most recent movement (products without one last), then the latest edit
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then_with(|| match (&a.2, &b.2) { (Some(x), Some(y)) => y.cmp(x), (Some(_), None) => std::cmp::Ordering::Less, (None, Some(_)) => std::cmp::Ordering::Greater, _ => std::cmp::Ordering::Equal }).then(b.3.cmp(&a.3)));
    let limit = param_int(&e, "point_of_sale.limited_product_count", 20000).max(0) as usize;
    let mut product_ids: Vec<i64> = rows.into_iter().take(limit).map(|r| r.4).collect();
    if let Value::List(sp) = orm::call(&e, "pos.config", "_get_special_products", &[cfg_id], &Row::new())? { for s in sp.iter().filter_map(|v| v.as_i64()) { if !product_ids.contains(&s) { product_ids.push(s); } } }
    // combo products bring the products of their items
    let mut extra: Vec<i64> = vec![];
    for p in &product_ids {
        let pr = rec(&e, "product.product", *p)?; let t = id_of(&pr, "product_tmpl_id").and_then(|t| rec(&e, "product.template", t).ok()).unwrap_or_default();
        if text(&t, "type").as_deref() != Some("combo") || !has_model(&e, "product.combo.item") { continue; }
        for c in ids(&t, "combo_ids") { for it in children(&e, "product.combo.item", "combo_id", c)? { if let Some(x) = id_of(&it, "product_id") { if rec(&e, "product.product", x).map_or(false, |r| r.get("active").map_or(true, |a| a.truthy())) && !product_ids.contains(&x) && !extra.contains(&x) { extra.push(x); } } } }
    }
    product_ids.extend(extra);
    let want: Vec<String> = fields.to_vec();
    orm::read(&e, "product.product", &product_ids, &want)
}

fn rules_core() -> Rules {
    Rules::default()
        .action("pos.config", "get_limited_product_count", |env, _, _| Ok(param_int(env, "point_of_sale.limited_product_count", 20000).into()))
        .action("pos.config", "_get_limited_partner_count", |env, _, _| Ok(param_int(env, "point_of_sale.limited_customer_count", 100).into()))
        .action("pos.config", "get_limited_partners_loading", |env, ids_, _| Ok(Value::List(limited_partners(env, &rec(env, "pos.config", first(ids_)?)?)?.into_iter().map(|p| Value::List(vec![Value::Int(p)])).collect())))
        .action("pos.config", "get_limited_products_loading", |env, ids_, kw| {
            let fields: Vec<String> = match kw.get("fields") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_str().map(String::from)).collect(), _ => vec![] };
            Ok(Value::List(limited_products(env, first(ids_)?, &fields)?.into_iter().map(Value::Map).collect()))
        })
        .action("pos.config", "_add_trusted_config_id", |env, ids_, kw| { orm::write(&env.sudo(), "pos.config", ids_, row(&[("trusted_config_ids", Value::List(vec![Value::List(vec![4.into(), kw.get("config_id").cloned().unwrap_or(Value::Null)])]))]))?; Ok(Value::Null) })
        .action("pos.config", "_remove_trusted_config_id", |env, ids_, kw| { orm::write(&env.sudo(), "pos.config", ids_, row(&[("trusted_config_ids", Value::List(vec![Value::List(vec![3.into(), kw.get("config_id").cloned().unwrap_or(Value::Null)])]))]))?; Ok(Value::Null) })
        .action("pos.config", "action_pos_config_modal_edit", |_, ids_, _| Ok(Value::Map(row(&[("view_mode", "form".into()), ("res_model", "pos.config".into()), ("type", "ir.actions.act_window".into()), ("target", "new".into()), ("res_id", ids_.first().map_or(Value::Bool(false), |i| Value::Int(*i))), ("context", Value::Map(row(&[("pos_config_open_modal", true.into())])))]))))
        .action("pos.config", "execute", |_, _, _| Ok(Value::Map(row(&[("type", "ir.actions.client".into()), ("tag", "reload".into())]))))
        .action("pos.config", "_force_http", |env, ids_, _| {
            let e = env.sudo(); let cfg = rec(&e, "pos.config", first(ids_)?)?;
            let enforce = find_one(&e, "ir.config_parameter", term("key", "=", "point_of_sale.enforce_https"))?.and_then(|p| rec(&e, "ir.config_parameter", p).ok()).and_then(|p| text(&p, "value")).map_or(false, |v| !v.is_empty() && v != "False" && v != "0");
            let epson = many(&e, "pos.printer", &ids(&cfg, "printer_ids"))?.iter().any(|p| text(p, "printer_type").as_deref() == Some("epson_epos"));
            Ok((!enforce && (flag(&cfg, "other_devices") || epson)).into())
        })
        .action("pos.config", "_check_pos_manager_access", |env, _, _| { check_manager_access(env)?; Ok(Value::Bool(true)) })
        // settings-view saves send absolute link commands: items that are no longer linked must be unlinked explicitly
        .action("pos.config", "_preprocess_x2many_vals_from_settings_view", |env, ids_, kw| {
            let mut vals = match kw.get("vals") { Some(Value::Map(m)) => m.clone(), _ => Row::new() };
            if !env.ctx.get("from_settings_view").map_or(false, |v| v.truthy()) { return Ok(Value::Map(vals)); }
            let cfg = rec(env, "pos.config", first(ids_)?)?;
            for (name, f) in env.reg.model("pos.config")?.fields.iter() {
                if !f.is_x2many() { continue; }
                let Some(Value::List(cmds)) = vals.get(name).cloned() else { continue };
                let mut linked: BTreeSet<i64> = ids(&cfg, name).into_iter().collect();
                for c in &cmds { if let Value::List(x) = c { if x.first().and_then(|v| v.as_i64()) == Some(4) { if let Some(i) = x.get(1).and_then(|v| v.as_i64()) { linked.remove(&i); } } } }
                let mut out: Vec<Value> = linked.into_iter().map(|i| Value::List(vec![3.into(), i.into()])).collect(); out.extend(cmds);
                vals.insert(name.clone(), Value::List(out));
            }
            Ok(Value::Map(vals))
        })
        // from the settings view only really changed values are written
        .action("pos.config", "_keep_new_vals", |env, ids_, kw| {
            let vals = match kw.get("vals") { Some(Value::Map(m)) => m.clone(), _ => Row::new() };
            if !env.ctx.get("from_settings_view").map_or(false, |v| v.truthy()) { return Ok(Value::Map(vals)); }
            let cfg = orm::read(env, "pos.config", &[first(ids_)?], &[])?.remove(0);
            let norm = |v: &Value| match v { Value::List(l) if l.len() == 2 && matches!(l[1], Value::Text(_)) => l[0].clone(), Value::Null => Value::Bool(false), other => other.clone() };
            let mut out = Row::new();
            for (k, v) in vals { match cfg.get(&k) { Some(cur) if norm(cur) == norm(&v) => {}, Some(_) | None => { if env.reg.field("pos.config", &k).is_ok() { out.insert(k, v); } } } }
            Ok(Value::Map(out))
        })
        .action("pos.config", "update_customer_display", |env, ids_, kw| {
            // the display only reacts to the register's own access token
            let cfg = rec(env, "pos.config", first(ids_)?)?; let token = text(kw, "access_token").unwrap_or_default();
            Ok(Value::Bool(!token.is_empty() && text(&cfg, "access_token").as_deref() == Some(token.as_str())))
        })
        .action("pos.config", "read_config_open_orders", |env, _, kw| {
            let e = env.sudo(); let domains = match kw.get("domain") { Some(Value::Map(m)) => m.clone(), _ => Row::new() }; let record_ids = match kw.get("record_ids") { Some(Value::Map(m)) => m.clone(), _ => Row::new() };
            let (mut dynamic, mut deleted) = (Row::new(), Row::new());
            for (model, dom) in &domains {
                let wanted: Vec<i64> = match record_ids.get(model) { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] };
                let d = Domain::parse(&dom.to_json())?;
                let found = orm::search(&e, model, &d, None, None, 0)?;
                let mut gone: Vec<i64> = wanted.iter().copied().filter(|i| rec(&e, model, *i).is_err()).collect();
                // cancelled orders must disappear from the terminal too
                if model == "pos.order" { for i in &wanted { if let Ok(o) = rec(&e, model, *i) { if text(&o, "state").as_deref() == Some("cancel") { gone.push(*i); } } } }
                deleted.insert(model.clone(), id_list(&gone));
                dynamic.insert(model.clone(), Value::List(orm::read(&e, model, &found, &[])?.into_iter().map(Value::Map).collect()));
            }
            Ok(Value::Map(row(&[("dynamic_records", Value::Map(dynamic)), ("deleted_record_ids", Value::Map(deleted))])))
        })
        .action("pos.config", "get_records", |env, _, kw| {
            let data = match kw.get("data") { Some(Value::Map(m)) => m.clone(), _ => Row::new() }; let mut out = Row::new();
            for (model, v) in data { let wanted: Vec<i64> = match v { Value::List(l) => l.iter().filter_map(|x| x.as_i64()).collect(), _ => vec![] }; out.insert(model.clone(), Value::List(orm::read(&env.sudo(), &model, &wanted, &[])?.into_iter().map(Value::Map).collect())); }
            Ok(Value::Map(out))
        })
        // the register's sequences go with it
        .on_unlink("pos.config", |env, ids_| {
            let e = env.sudo();
            for i in ids_ { let c = rec(&e, "pos.config", *i)?; let seqs: Vec<i64> = ["sequence_id", "sequence_line_id"].iter().filter_map(|f| id_of(&c, f)).collect(); if !seqs.is_empty() { orm::unlink(&e, "ir.sequence", &seqs)?; } }
            Ok(vec![])
        })
        .on_unlink("account.cash.rounding", |env, ids_| {
            if !orm::search(&env.sudo(), "pos.config", &term("rounding_method", "in", id_list(ids_)), None, Some(1), 0)?.is_empty() { return user_err("You cannot delete a rounding method that is used in a Point of Sale configuration."); }
            Ok(vec![])
        })
        .after_create("pos.note", |env, ids_, _| { for i in ids_ { check_note_unique(env, *i)?; } Ok(()) })
        .after_write("pos.note", |env, ids_, vals| { if vals.contains_key("name") { for i in ids_ { check_note_unique(env, *i)?; } } Ok(()) })
        // a POS operation type per warehouse (outgoing from the stock location to customers), created on demand
        .action("stock.warehouse", "_create_missing_pos_picking_types", |env, _, _| {
            let e = env.sudo(); let mut made = vec![];
            if !has_model(&e, "stock.warehouse") || e.reg.field("stock.warehouse", "pos_type_id").is_err() { return Ok(Value::List(vec![])); }
            for w in orm::search(&e, "stock.warehouse", &term("pos_type_id", "=", Value::Bool(false)), None, None, 0)? {
                let wr = rec(&e, "stock.warehouse", w)?; let customers = stock::ensure_location(&e, "customer", "Customers")?;
                let mut v = row(&[("name", "PoS Orders".into()), ("code", "outgoing".into()), ("default_location_dest_id", customers.into()), ("sequence_code", "POS".into()), ("warehouse_id", w.into())]);
                if let Some(l) = id_of(&wr, "lot_stock_id") { v.insert("default_location_src_id".into(), l.into()); }
                if let Some(c) = id_of(&wr, "company_id") { v.insert("company_id".into(), c.into()); }
                let pt = orm::create(&e, "stock.picking.type", v)?;
                orm::write(&e, "stock.warehouse", &[w], row(&[("pos_type_id", pt.into())]))?; made.push(Value::Int(pt));
            }
            Ok(Value::List(made))
        })
}
fn check_note_unique(env: &Env, id: i64) -> Result<()> {
    let n = text(&rec(env, "pos.note", id)?, "name").unwrap_or_default();
    if orm::search(&env.sudo(), "pos.note", &Domain::And(vec![term("name", "=", n.as_str()), term("id", "!=", id)]), None, Some(1), 0)?.is_empty() { Ok(()) } else { invalid("A note with this name already exists") }
}

pub fn rules() -> Rules { rules_core() }
