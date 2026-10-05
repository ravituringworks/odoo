//! Smaller point_of_sale extensions of other models, ported from `addons/point_of_sale/models`: res.partner, account.journal,
//! account.tax, pos.category, pos.bill, product.template/product, stock.picking(.type), digest and the settings screen.
//!
//! `get_product_info_pos` evaluates pricelists with fixed / percentage / formula rules only (no per-company or UoM variants).
use crate::pos_methods::{flag, has_model, id_list, invalid, user_err};
use crate::{stock, tax, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};
use std::collections::BTreeSet;

pub fn rules() -> Rules {
    partner_rules().merge(journal_rules()).merge(tax_rules()).merge(category_rules()).merge(product_rules()).merge(stock_rules()).merge(settings_rules())
}
fn first(ids_: &[i64], what: &str) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User(format!("Select a {what}"))) }
fn act(name: &str, model: &str, mode: &str, domain: Value) -> Value { Value::Map(row(&[("name", name.into()), ("res_model", model.into()), ("view_mode", mode.into()), ("type", "ir.actions.act_window".into()), ("domain", domain)])) }

// ---------------------------------------------------------------------------------------------------------------------
// res.partner
// ---------------------------------------------------------------------------------------------------------------------
/// The partner and everything below it in the contact hierarchy (`child_of`).
fn partner_family(env: &Env, root: i64) -> Result<Vec<i64>> {
    let e = env.sudo().with_ctx("active_test", Value::Bool(false));
    let mut all = vec![root]; let mut frontier = vec![root];
    while !frontier.is_empty() {
        let kids = orm::search(&e, "res.partner", &term("parent_id", "in", id_list(&frontier)), None, None, 0)?;
        frontier = kids.into_iter().filter(|k| !all.contains(k)).collect(); all.extend(frontier.iter().copied());
    }
    Ok(all)
}
fn partner_rules() -> Rules {
    Rules::default()
        // orders of the partner and of all its contacts
        .compute("res.partner", "pos_order_count", |env, r| Ok((orm::search_count(&env.sudo(), "pos.order", &term("partner_id", "in", id_list(&partner_family(env, r["id"].as_i64().unwrap())?)))?).into()))
        .action("res.partner", "action_view_pos_order", |env, ids_, _| {
            let p = rec(env, "res.partner", first(ids_, "partner")?)?;
            let dom = if flag(&p, "is_company") { Value::List(vec![Value::List(vec!["partner_id.commercial_partner_id".into(), "=".into(), p["id"].clone()])]) } else { Value::List(vec![Value::List(vec!["partner_id".into(), "=".into(), p["id"].clone()])]) };
            Ok(act("Orders", "pos.order", "list,form", dom))
        })
        .on_unlink("res.partner", |env, ids_| {
            if !orm::search(&env.sudo().with_ctx("active_test", Value::Bool(false)), "pos.order", &term("partner_id", "in", id_list(ids_)), None, Some(1), 0)?.is_empty() {
                return invalid("You cannot delete a customer that has point of sales orders. You can archive it instead.");
            }
            Ok(vec![])
        })
}

// ---------------------------------------------------------------------------------------------------------------------
// account.journal
// ---------------------------------------------------------------------------------------------------------------------
fn check_no_active_payments(env: &Env, journals: &[i64]) -> Result<()> {
    let e = env.sudo();
    let methods = orm::search(&e, "pos.payment.method", &term("journal_id", "in", id_list(journals)), None, None, 0)?;
    if methods.is_empty() { return Ok(()); }
    let pays = orm::search_read(&e, "pos.payment", &term("payment_method_id", "in", id_list(&methods)), &["id".into(), "payment_method_id".into(), "pos_order_id".into(), "session_id".into()], Some("id desc"), None, 0)?;
    for p in pays {
        let Some(sess) = id_of(&p, "pos_order_id").and_then(|o| rec(&e, "pos.order", o).ok()).and_then(|o| id_of(&o, "session_id")).and_then(|s| rec(&e, "pos.session", s).ok()) else { continue };
        if text(&sess, "state").as_deref() == Some("opened") {
            let pm = id_of(&p, "payment_method_id").and_then(|m| rec(&e, "pos.payment.method", m).ok()).and_then(|m| text(&m, "name")).unwrap_or_default();
            let order = id_of(&p, "pos_order_id").and_then(|o| rec(&e, "pos.order", o).ok()).and_then(|o| text(&o, "name")).unwrap_or_default();
            return invalid(format!("This journal is associated with payment method {pm} that is being used by order {order} in the active pos session {}", text(&sess, "name").unwrap_or_default()));
        }
    }
    Ok(())
}
fn journal_rules() -> Rules {
    Rules::default()
        .after_write("account.journal", |env, ids_, vals| {
            if vals.contains_key("type") && !orm::search(&env.sudo(), "pos.payment.method", &term("journal_id", "in", id_list(ids_)), None, Some(1), 0)?.is_empty() {
                return invalid("This journal is associated with a payment method. You cannot modify its type");
            }
            Ok(())
        })
        .on_unlink("account.journal", |env, ids_| { for i in ids_ { check_no_active_payments(env, &[*i])?; } Ok(vec![]) })
        .action("account.journal", "_check_no_active_payments", |env, ids_, _| { check_no_active_payments(env, ids_)?; Ok(Value::Bool(true)) })
        .action("account.journal", "_check_type", |env, ids_, _| {
            if !orm::search(&env.sudo(), "pos.payment.method", &term("journal_id", "in", id_list(ids_)), None, Some(1), 0)?.is_empty() { return invalid("This journal is associated with a payment method. You cannot modify its type"); }
            Ok(Value::Bool(true))
        })
        .action("account.journal", "action_archive", |env, ids_, _| {
            check_no_active_payments(env, ids_)?;
            orm::write(&env.with_ctx("active_test", Value::Bool(false)), "account.journal", ids_, row(&[("active", false.into())]))?; Ok(Value::Bool(true))
        })
        // the company's "Point of Sale" miscellaneous journal, created on first use
        .action("account.journal", "_ensure_company_account_journal", |env, _, kw| {
            let e = env.sudo(); let company = kw.get("company_id").and_then(|v| v.as_i64()).or_else(|| find_one(&e, "res.company", Domain::True).ok().flatten()).unwrap_or(1);
            Ok(Value::Int(find_or_create(&e, "account.journal", Domain::And(vec![term("code", "=", "POSS"), term("company_id", "=", company)]), row(&[("name", "Point of Sale".into()), ("code", "POSS".into()), ("type", "general".into()), ("company_id", company.into())]))?))
        })
}

// ---------------------------------------------------------------------------------------------------------------------
// account.tax: taxes used by unposted POS orders are frozen
// ---------------------------------------------------------------------------------------------------------------------
fn tax_rules() -> Rules {
    Rules::default().after_write("account.tax", |env, ids_, vals| {
        const FORBIDDEN: [&str; 8] = ["amount_type", "amount", "type_tax_use", "tax_group_id", "price_include", "price_include_override", "include_base_amount", "is_base_affected"];
        if !FORBIDDEN.iter().any(|k| vals.contains_key(*k)) || !has_model(env, "pos.order.line") { return Ok(()); }
        let e = env.sudo();
        let lines = orm::search(&e, "pos.order.line", &term("tax_ids", "in", id_list(ids_)), None, None, 0)?;
        for l in lines {
            let sess = id_of(&rec(&e, "pos.order.line", l)?, "order_id").and_then(|o| rec(&e, "pos.order", o).ok()).and_then(|o| id_of(&o, "session_id")).and_then(|s| rec(&e, "pos.session", s).ok());
            if sess.map_or(false, |s| text(&s, "state").as_deref() != Some("closed")) { return user_err("It is forbidden to modify a tax used in a POS order not posted. You must close the POS sessions before modifying the tax."); }
        }
        Ok(())
    })
}

// ---------------------------------------------------------------------------------------------------------------------
// pos.category / pos.bill
// ---------------------------------------------------------------------------------------------------------------------
fn category_descendants(env: &Env, roots: &[i64]) -> Result<Vec<i64>> {
    let mut all: Vec<i64> = roots.to_vec(); let mut frontier: Vec<i64> = roots.to_vec();
    while !frontier.is_empty() {
        let kids = orm::search(&env.sudo(), "pos.category", &term("parent_id", "in", id_list(&frontier)), Some("sequence, name"), None, 0)?;
        frontier = kids.into_iter().filter(|k| !all.contains(k)).collect(); all.extend(frontier.iter().copied());
    }
    Ok(all)
}
fn hierarchy(env: &Env, id: i64) -> Result<Vec<String>> {
    let mut chain = vec![]; let mut cur = Some(id);
    while let Some(c) = cur { if chain.len() > 64 { break; } let r = rec(env, "pos.category", c)?; chain.push(text(&r, "name").unwrap_or_default()); cur = id_of(&r, "parent_id"); }
    chain.reverse(); Ok(chain)
}
fn category_rules() -> Rules {
    Rules::default()
        .compute("pos.category", "has_image", |_, r| Ok(flag(r, "image_128").into()))
        // a sub-category takes its parent's colour
        .before_create("pos.category", |env, mut v| {
            if let Some(p) = v.get("parent_id").and_then(|p| p.as_i64()) { if let Some(c) = rec(env, "pos.category", p)?.get("color") { v.insert("color".into(), c.clone()); } }
            Ok(v)
        })
        .before_write("pos.category", |env, mut v| {
            if let (Some(p), false) = (v.get("parent_id").and_then(|p| p.as_i64()), v.contains_key("color")) { if let Some(c) = rec(env, "pos.category", p)?.get("color") { v.insert("color".into(), c.clone()); } }
            Ok(v)
        })
        .after_write("pos.category", |env, ids_, vals| {
            if !vals.contains_key("parent_id") { return Ok(()); }
            for i in ids_ {
                let mut seen = vec![*i]; let mut cur = id_of(&rec(env, "pos.category", *i)?, "parent_id");
                while let Some(c) = cur { if seen.contains(&c) { return invalid("Error! You cannot create recursive categories."); } seen.push(c); cur = id_of(&rec(env, "pos.category", c)?, "parent_id"); }
            }
            Ok(())
        })
        .on_unlink("pos.category", |env, _| {
            if !orm::search(&env.sudo(), "pos.session", &term("state", "!=", "closed"), None, Some(1), 0)?.is_empty() { return user_err("You cannot delete a point of sale category while a session is still opened."); }
            Ok(vec![])
        })
        .action("pos.category", "_get_hierarchy", |env, ids_, _| Ok(Value::List(hierarchy(env, first(ids_, "category")?)?.into_iter().map(Value::Text).collect())))
        .action("pos.category", "_get_descendants", |env, ids_, _| Ok(id_list(&category_descendants(env, ids_)?)))
        .action("pos.category", "_check_category_recursion", |env, ids_, _| {
            for i in ids_ { let mut seen = vec![*i]; let mut cur = id_of(&rec(env, "pos.category", *i)?, "parent_id"); while let Some(c) = cur { if seen.contains(&c) { return invalid("Error! You cannot create recursive categories."); } seen.push(c); cur = id_of(&rec(env, "pos.category", c)?, "parent_id"); } }
            Ok(Value::Bool(true))
        })
        // coins/bills are named after their value
        .action("pos.bill", "name_create", |env, _, kw| {
            let name = text(kw, "name").unwrap_or_default();
            let Ok(value) = name.trim().parse::<f64>() else { return user_err("The name of the Coins/Bills must be a number.") };
            let id = orm::create(env, "pos.bill", row(&[("name", name.as_str().into()), ("value", value.into())]))?;
            Ok(Value::List(vec![Value::Int(id), Value::Text(name)]))
        })
}

// ---------------------------------------------------------------------------------------------------------------------
// product.template / product.product
// ---------------------------------------------------------------------------------------------------------------------
fn open_sessions_exist(env: &Env) -> Result<bool> { Ok(!orm::search(&env.sudo(), "pos.session", &term("state", "!=", "closed"), None, Some(1), 0)?.is_empty()) }
const DELETE_MSG: &str = "To delete a product, make sure all point of sale sessions are closed.\n\nDeleting a product available in a session would be like attempting to snatch a hamburger from a customer’s hand mid-bite; chaos will ensue as ketchup and mayo go flying everywhere!";

/// The price a pricelist gives `product` for `qty` (first matching rule; list price when none applies).
fn pricelist_price(env: &Env, pl: i64, product: &Row, tmpl: &Row, qty: f64, depth: u8) -> Result<f64> {
    let list = num(tmpl, "list_price");
    if !has_model(env, "product.pricelist.item") || depth > 4 { return Ok(list); }
    let now = orm::now();
    let mut items = children(env, "product.pricelist.item", "pricelist_id", pl)?;
    items.sort_by(|a, b| text(a, "applied_on").cmp(&text(b, "applied_on")).then(num(b, "min_quantity").partial_cmp(&num(a, "min_quantity")).unwrap_or(std::cmp::Ordering::Equal)).then(b["id"].as_i64().cmp(&a["id"].as_i64())));
    let categ = id_of(tmpl, "categ_id");
    let in_categ = |c: i64| -> bool { let mut cur = categ; let mut n = 0; while let Some(x) = cur { if x == c { return true; } n += 1; if n > 32 { break; } cur = rec(env, "product.category", x).ok().and_then(|r| id_of(&r, "parent_id")); } false };
    for it in items {
        if num(&it, "min_quantity") > qty { continue; }
        if text(&it, "date_start").map_or(false, |d| d > now) || text(&it, "date_end").map_or(false, |d| d < now) { continue; }
        let hit = match text(&it, "applied_on").as_deref() {
            Some("0_product_variant") => id_of(&it, "product_id") == product["id"].as_i64(),
            Some("1_product") => id_of(&it, "product_tmpl_id") == tmpl["id"].as_i64(),
            Some("2_product_category") => id_of(&it, "categ_id").map_or(false, in_categ),
            _ => true,
        };
        if !hit { continue; }
        let base = match text(&it, "base").as_deref() {
            Some("standard_price") => num(product, "standard_price"),
            Some("pricelist") => match id_of(&it, "base_pricelist_id") { Some(b) => pricelist_price(env, b, product, tmpl, qty, depth + 1)?, None => list },
            _ => list,
        };
        return Ok(match text(&it, "compute_price").as_deref() {
            Some("fixed") => num(&it, "fixed_price"),
            Some("percentage") => base - base * num(&it, "percent_price") / 100.0,
            _ => {
                let mut price = base - base * num(&it, "price_discount") / 100.0;
                if num(&it, "price_round") != 0.0 { price = (price / num(&it, "price_round")).round() * num(&it, "price_round"); }
                if num(&it, "price_surcharge") != 0.0 { price += num(&it, "price_surcharge"); }
                if num(&it, "price_min_margin") != 0.0 { price = price.max(base + num(&it, "price_min_margin")); }
                if num(&it, "price_max_margin") != 0.0 { price = price.min(base + num(&it, "price_max_margin")); }
                price
            }
        });
    }
    Ok(list)
}

fn product_info_pos(env: &Env, pid: i64, price: f64, quantity: f64, config: i64) -> Result<Value> {
    let e = env.sudo(); let p = rec(&e, "product.product", pid)?;
    let t = rec(&e, "product.template", id_of(&p, "product_tmpl_id").ok_or_else(|| OdooError::User("Product without template".into()))?)?;
    let cfg = rec(&e, "pos.config", config)?; let company = id_of(&cfg, "company_id");
    // taxes of the config's company
    let mut tax_ids: Vec<i64> = vec![];
    for x in ids(&t, "taxes_id") { if id_of(&rec(&e, "account.tax", x)?, "company_id") == company { tax_ids.push(x); } }
    let all = tax::load(&e, &tax_ids)?;
    let (excl, _, incl) = tax::compute(quantity, price, 0.0, &all);
    let mut details: Vec<Value> = vec![];
    for (i, x) in tax_ids.iter().enumerate() {
        let one = tax::compute(quantity, price, 0.0, &all[i..=i]);
        details.push(Value::Map(row(&[("name", text(&rec(&e, "account.tax", *x)?, "name").unwrap_or_default().into()), ("amount", (if quantity != 0.0 { (one.1) / quantity } else { 0.0 }).into())])));
    }
    let per = |x: f64| if quantity != 0.0 { x / quantity } else { 0.0 };
    let all_prices = Value::Map(row(&[("price_without_tax", per(excl).into()), ("price_with_tax", per(incl).into()), ("tax_details", Value::List(details))]));
    // pricelists
    let pls = if flag(&cfg, "use_pricelist") { ids(&cfg, "available_pricelist_ids") } else { id_of(&cfg, "pricelist_id").into_iter().collect() };
    let mut pricelists = vec![];
    for pl in pls { let r = rec(&e, "product.pricelist", pl)?; pricelists.push(Value::Map(row(&[("id", pl.into()), ("name", r.get("name").cloned().unwrap_or(Value::Null)), ("price", pricelist_price(&e, pl, &p, &t, quantity, 0)?.into())]))); }
    // warehouses: on hand in the stock location, forecast adds the moves still to come
    let uom = id_of(&t, "uom_id").and_then(|u| rec(&e, "uom.uom", u).ok()).and_then(|u| text(&u, "name"));
    let mut warehouses: Vec<(bool, Value)> = vec![];
    if has_model(&e, "stock.warehouse") {
        let own = id_of(&cfg, "picking_type_id").and_then(|pt| rec(&e, "stock.picking.type", pt).ok()).and_then(|pt| id_of(&pt, "warehouse_id"));
        for w in orm::search(&e, "stock.warehouse", &company.map_or(Domain::True, |c| term("company_id", "=", c)), None, None, 0)? {
            let wr = rec(&e, "stock.warehouse", w)?; let stock_loc = id_of(&wr, "lot_stock_id");
            let (avail, forecast) = match stock_loc {
                Some(l) => {
                    let on_hand = stock::on_hand(&e, pid, l)?;
                    let (mut inc, mut out) = (0.0, 0.0);
                    for m in children(&e, "stock.move", "product_id", pid)? {
                        if !matches!(text(&m, "state").as_deref(), Some("confirmed" | "assigned" | "waiting" | "partially_available")) { continue; }
                        if id_of(&m, "location_dest_id") == Some(l) && id_of(&m, "location_id") != Some(l) { inc += num(&m, "product_uom_qty"); }
                        if id_of(&m, "location_id") == Some(l) && id_of(&m, "location_dest_id") != Some(l) { out += num(&m, "product_uom_qty"); }
                    }
                    (on_hand, on_hand + inc - out)
                }
                None => (0.0, 0.0),
            };
            warehouses.push((own == Some(w), Value::Map(row(&[("id", w.into()), ("name", wr.get("name").cloned().unwrap_or(Value::Null)), ("available_quantity", avail.into()), ("forecasted_quantity", forecast.into()), ("uom", uom.clone().map_or(Value::Bool(false), Value::Text))]))));
        }
        warehouses.sort_by_key(|(first, _)| !*first);   // the register's own warehouse comes first
    }
    // suppliers: first valid price line per partner
    let mut suppliers = vec![];
    if has_model(&e, "product.supplierinfo") {
        let today = orm::today();
        let mut lines = children(&e, "product.supplierinfo", "product_tmpl_id", t["id"].as_i64().unwrap())?; lines.sort_by_key(|l| id_of(l, "partner_id"));
        let mut seen: BTreeSet<Option<i64>> = BTreeSet::new();
        for s in lines {
            let pr = id_of(&s, "partner_id"); if seen.contains(&pr) { continue; }
            if text(&s, "date_start").map_or(false, |d| d > today) || text(&s, "date_end").map_or(false, |d| d < today) || num(&s, "min_qty") > quantity { continue; }
            seen.insert(pr);
            suppliers.push(Value::Map(row(&[("id", s["id"].clone()), ("name", pr.and_then(|p| rec(&e, "res.partner", p).ok()).and_then(|p| text(&p, "name")).map_or(Value::Bool(false), Value::Text)), ("delay", s.get("delay").cloned().unwrap_or(Value::Null)), ("price", s.get("price").cloned().unwrap_or(Value::Null))])));
        }
    }
    // variants: attribute lines with their values
    let mut variants = vec![];
    if has_model(&e, "product.template.attribute.line") {
        for l in children(&e, "product.template.attribute.line", "product_tmpl_id", t["id"].as_i64().unwrap())? {
            let attr = id_of(&l, "attribute_id").and_then(|a| rec(&e, "product.attribute", a).ok()).and_then(|a| text(&a, "name")).unwrap_or_default();
            let tname = text(&t, "name").unwrap_or_default();
            let values: Vec<Value> = ids(&l, "value_ids").into_iter().filter_map(|v| rec(&e, "product.attribute.value", v).ok()).filter_map(|v| text(&v, "name")).map(|n| Value::Map(row(&[("name", n.as_str().into()), ("search", format!("{tname} {n}").into())]))).collect();
            variants.push(Value::Map(row(&[("name", attr.into()), ("values", Value::List(values))])));
        }
    }
    Ok(Value::Map(row(&[("all_prices", all_prices), ("pricelists", Value::List(pricelists)), ("warehouses", Value::List(warehouses.into_iter().map(|x| x.1).collect())), ("suppliers", Value::List(suppliers)), ("variants", Value::List(variants))])))
}

fn product_rules() -> Rules {
    Rules::default()
        // stored, editable: the colour of the first POS category (kept as is without categories)
        .compute("product.template", "color", |env, r| {
            let mut cats = crate::pos_methods::many(env, "pos.category", &ids(r, "pos_categ_ids"))?;
            cats.sort_by(|a, b| num(a, "sequence").partial_cmp(&num(b, "sequence")).unwrap_or(std::cmp::Ordering::Equal).then(text(a, "name").cmp(&text(b, "name"))));
            Ok(cats.first().and_then(|c| c.get("color").cloned()).unwrap_or_else(|| r.get("color").cloned().filter(|v| !v.is_null()).unwrap_or(0.into())))
        })
        .after_write("product.template", |env, ids_, vals| {
            if !vals.contains_key("available_in_pos") { return Ok(()); }
            check_combo_inclusions(env, ids_)
        })
        .on_unlink("product.template", |env, ids_| {
            let e = env.sudo().with_ctx("active_test", Value::Bool(false));
            if !orm::search(&e, "product.template", &Domain::And(vec![term("id", "in", id_list(ids_)), term("available_in_pos", "=", true)]), None, Some(1), 0)?.is_empty() && open_sessions_exist(&e)? { return user_err(DELETE_MSG); }
            Ok(vec![])
        })
        .on_unlink("product.product", |env, ids_| {
            let e = env.sudo().with_ctx("active_test", Value::Bool(false));
            if open_sessions_exist(&e)? && !orm::search(&e, "product.product", &Domain::And(vec![term("id", "in", id_list(ids_)), term("product_tmpl_id.available_in_pos", "=", true)]), None, Some(1), 0)?.is_empty() { return user_err(DELETE_MSG); }
            Ok(vec![])
        })
        .action("product.template", "_check_combo_inclusions", |env, ids_, _| { check_combo_inclusions(env, ids_)?; Ok(Value::Bool(true)) })
        .action("product.product", "get_product_info_pos", |env, ids_, kw| product_info_pos(env, first(ids_, "product")?, num(kw, "price"), num(kw, "quantity"), kw.get("pos_config_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("pos_config_id required".into()))?))
        .compute("digest.digest", "kpi_pos_total_value", |env, _| {
            // sales of orders that are paid but not yet invoiced, in the digest period
            let e = env.sudo(); let mut dom = vec![term("state", "not in", Value::List(vec!["draft".into(), "cancel".into(), "invoiced".into()]))];
            if let Some(Value::Text(s)) = env.ctx.get("start_datetime") { dom.push(term("date_order", ">=", s.as_str())); }
            if let Some(Value::Text(s)) = env.ctx.get("end_datetime") { dom.push(term("date_order", "<", s.as_str())); }
            Ok(orm::search_read(&e, "pos.order", &Domain::And(dom), &["amount_total".into()], None, None, 0)?.iter().map(|o| num(o, "amount_total")).sum::<f64>().into())
        })
}
fn check_combo_inclusions(env: &Env, ids_: &[i64]) -> Result<()> {
    if !has_model(env, "product.combo.item") { return Ok(()); }
    let e = env.sudo();
    for t in ids_ {
        if flag(&rec(&e, "product.template", *t)?, "available_in_pos") { continue; }
        let variants = orm::search(&e.with_ctx("active_test", Value::Bool(false)), "product.product", &term("product_tmpl_id", "=", *t), None, None, 0)?;
        if let Some(item) = find_one(&e, "product.combo.item", term("product_id", "in", id_list(&variants)))? {
            let name = id_of(&rec(&e, "product.combo.item", item)?, "combo_id").and_then(|c| rec(&e, "product.combo", c).ok()).and_then(|c| text(&c, "name")).unwrap_or_default();
            return user_err(format!("You must first remove this product from the {name} combo"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// stock.picking / stock.picking.type
// ---------------------------------------------------------------------------------------------------------------------
/// `_create_picking_from_pos_order_lines`: one outgoing picking for sold quantities and one return picking for refunded ones.
fn pickings_from_lines(env: &Env, dest: i64, line_ids: &[i64], picking_type: i64, partner: Option<i64>) -> Result<Vec<i64>> {
    let e = env.sudo(); let mut out = vec![];
    let mut stockable: Vec<Row> = vec![];
    for l in line_ids {
        let l = rec(&e, "pos.order.line", *l)?;
        let ty = id_of(&l, "product_id").and_then(|p| rec(&e, "product.product", p).ok()).and_then(|p| id_of(&p, "product_tmpl_id")).and_then(|t| rec(&e, "product.template", t).ok()).and_then(|t| text(&t, "type"));
        if ty.as_deref() == Some("consu") && num(&l, "qty").abs() > 1e-9 { stockable.push(l); }
    }
    if stockable.is_empty() { return Ok(out); }
    let pt = rec(&e, "stock.picking.type", picking_type)?;
    for positive in [true, false] {
        let lines: Vec<&Row> = stockable.iter().filter(|l| (num(l, "qty") > 0.0) == positive).collect();
        if lines.is_empty() { continue; }
        let (ptype, src, dst) = if positive { (picking_type, id_of(&pt, "default_location_src_id"), Some(dest)) } else {
            match id_of(&pt, "return_picking_type_id").and_then(|r| rec(&e, "stock.picking.type", r).ok()) {
                Some(rt) => (rt["id"].as_i64().unwrap(), Some(dest), id_of(&rt, "default_location_dest_id")),
                None => (picking_type, Some(dest), id_of(&pt, "default_location_src_id")),
            }
        };
        let mut pv = row(&[("picking_type_id", ptype.into()), ("move_type", "direct".into()), ("state", "draft".into())]);
        if let Some(s) = src { pv.insert("location_id".into(), s.into()); }
        if let Some(d) = dst { pv.insert("location_dest_id".into(), d.into()); }
        if let Some(p) = partner { pv.insert("partner_id".into(), p.into()); }
        let pk = orm::create(&e, "stock.picking", pv)?;
        // one move per product (and attribute combination)
        let mut groups: Vec<((i64, Vec<i64>), Vec<&Row>)> = vec![];
        for l in lines { let mut attrs = ids(l, "attribute_value_ids"); attrs.sort(); let key = (id_of(l, "product_id").unwrap_or(0), attrs); match groups.iter_mut().find(|g| g.0 == key) { Some(g) => g.1.push(l), None => groups.push((key, vec![l])) } }
        for ((pid, _), ls) in groups {
            let qty: f64 = ls.iter().map(|l| num(l, "qty")).sum::<f64>().abs();
            let mut mv = row(&[("name", text(ls[0], "name").unwrap_or_default().into()), ("picking_id", pk.into()), ("picking_type_id", ptype.into()), ("product_id", pid.into()), ("product_uom_qty", qty.into()), ("state", "confirmed".into())]);
            if let Some(s) = src { mv.insert("location_id".into(), s.into()); }
            if let Some(d) = dst { mv.insert("location_dest_id".into(), d.into()); }
            orm::create(&e, "stock.move", mv)?;
        }
        // _action_done is attempted in a savepoint and its failure is tolerated, like Odoo
        let _ = odoo_core::store::nested(e.conn, || stock::validate(&e, pk));
        out.push(pk);
    }
    Ok(out)
}
fn stock_rules() -> Rules {
    Rules::default()
        .action("stock.picking", "_prepare_picking_vals", |_, _, kw| {
            let mut v = row(&[("user_id", false.into()), ("picking_type_id", kw.get("picking_type_id").cloned().unwrap_or(Value::Null)), ("move_type", "direct".into()), ("location_id", kw.get("location_id").cloned().unwrap_or(Value::Null)), ("location_dest_id", kw.get("location_dest_id").cloned().unwrap_or(Value::Null)), ("state", "draft".into())]);
            v.insert("partner_id".into(), kw.get("partner_id").filter(|p| p.truthy()).cloned().unwrap_or(Value::Bool(false)));
            Ok(Value::Map(v))
        })
        .action("stock.picking", "_create_picking_from_pos_order_lines", |env, _, kw| {
            let lines: Vec<i64> = match kw.get("line_ids") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] };
            let pt = kw.get("picking_type_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("picking_type_id required".into()))?;
            Ok(id_list(&pickings_from_lines(env, kw.get("location_dest_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("location_dest_id required".into()))?, &lines, pt, kw.get("partner_id").and_then(|v| v.as_i64()))?))
        })
        .after_write("stock.picking.type", |env, ids_, vals| {
            if !vals.contains_key("active") || flag(vals, "active") { return Ok(()); }
            let e = env.sudo();
            for i in ids_ {
                if let Some(c) = find_one(&e, "pos.config", term("picking_type_id", "=", *i))? {
                    let pt = rec(&e, "stock.picking.type", *i)?;
                    return invalid(format!("You cannot archive '{}' as it is used by POS configuration '{}'.", text(&pt, "name").unwrap_or_default(), text(&rec(&e, "pos.config", c)?, "name").unwrap_or_default()));
                }
            }
            Ok(())
        })
}

// ---------------------------------------------------------------------------------------------------------------------
// res.config.settings (the POS part of the settings screen)
// ---------------------------------------------------------------------------------------------------------------------
fn settings_config(env: &Env, r: &Row) -> Option<Row> { id_of(r, "pos_config_id").and_then(|c| rec(env, "pos.config", c).ok()) }
/// Value copied from the config only while `gate` (a flag on the config) is on; `false` otherwise.
fn gated(env: &Env, r: &Row, gate: &str, field: &str, off: Value) -> Value {
    match settings_config(env, r) { Some(c) if flag(&c, gate) => c.get(field).cloned().filter(|v| !v.is_null()).unwrap_or(off), _ => off }
}
fn settings_rules() -> Rules {
    Rules::default()
        // the settings open on the last modified register of the company
        .default_for("res.config.settings", "pos_config_id", |env| {
            let e = env.sudo(); if !has_model(&e, "pos.config") { return Ok(Value::Null); }
            Ok(orm::search(&e, "pos.config", &Domain::True, Some("write_date desc, id desc"), Some(1), 0)?.first().map_or(Value::Null, |i| Value::Int(*i)))
        })
        .compute("res.config.settings", "pos_is_order_printer", |env, r| Ok(settings_config(env, r).map_or(false, |c| flag(&c, "is_order_printer")).into()))
        .compute("res.config.settings", "pos_iface_cashdrawer", |env, r| {
            // the cash drawer is only offered when printing goes through the IoT box
            let via_proxy = settings_config(env, r).map_or(false, |c| flag(&c, "is_posbox") && flag(&c, "iface_print_via_proxy"));
            Ok(if via_proxy { settings_config(env, r).map_or(false, |c| flag(&c, "iface_cashdrawer")) } else { false }.into())
        })
        .compute("res.config.settings", "pos_iface_print_via_proxy", |env, r| Ok(gated(env, r, "is_posbox", "iface_print_via_proxy", Value::Bool(false))))
        .compute("res.config.settings", "pos_iface_scan_via_proxy", |env, r| Ok(gated(env, r, "is_posbox", "iface_scan_via_proxy", Value::Bool(false))))
        .compute("res.config.settings", "pos_iface_electronic_scale", |env, r| Ok(gated(env, r, "is_posbox", "iface_electronic_scale", Value::Bool(false))))
        .compute("res.config.settings", "pos_receipt_header", |env, r| Ok(gated(env, r, "is_header_or_footer", "receipt_header", Value::Null)))
        .compute("res.config.settings", "pos_receipt_footer", |env, r| Ok(gated(env, r, "is_header_or_footer", "receipt_footer", Value::Null)))
        .compute("res.config.settings", "pos_tip_product_id", |env, r| Ok(gated(env, r, "iface_tipproduct", "tip_product_id", Value::Null)))
        .compute("res.config.settings", "pos_default_fiscal_position_id", |env, r| Ok(gated(env, r, "tax_regime_selection", "default_fiscal_position_id", Value::Null)))
        .compute("res.config.settings", "pos_pricelist_id", |env, r| {
            // pricelists are only used when enabled; a pricelist in another currency than the register falls back to the first one in its currency
            let Some(c) = settings_config(env, r) else { return Ok(Value::Null) };
            if !flag(&c, "use_pricelist") { return Ok(Value::Null); }
            let cur = crate::pos_methods::config_currency_id(env, c["id"].as_i64().unwrap());
            let mismatched = crate::pos_methods::many(env, "product.pricelist", &ids(&c, "available_pricelist_ids"))?.iter().any(|p| id_of(p, "currency_id") != cur);
            if mismatched {
                let found = orm::search(&env.sudo(), "product.pricelist", &cur.map_or(Domain::True, |x| term("currency_id", "=", x)), None, Some(1), 0)?;
                return Ok(found.first().map_or(Value::Null, |p| Value::Int(*p)));
            }
            Ok(id_of(&c, "pricelist_id").map_or(Value::Null, Value::Int))
        })
        .action("res.config.settings", "_is_cashdrawer_displayed", |env, ids_, _| { let r = rec(env, "res.config.settings", first(ids_, "settings")?)?; Ok(settings_config(env, &r).map_or(false, |c| flag(&c, "iface_print_via_proxy")).into()) })
        .action("res.config.settings", "pos_open_ui", |env, _, kw| {
            let cfg = kw.get("pos_config_id").and_then(|v| v.as_i64()).or_else(|| env.ctx.get("pos_config_id").and_then(|v| v.as_i64()));
            match cfg { Some(c) => orm::call(env, "pos.config", "open_ui", &[c], &Row::new()), None => Ok(Value::Null) }
        })
        .action("res.config.settings", "action_pos_config_create_new", |_, _, _| Ok(Value::Map(row(&[("view_mode", "form".into()), ("res_model", "pos.config".into()), ("type", "ir.actions.act_window".into()), ("target", "new".into()), ("res_id", false.into()), ("context", Value::Map(row(&[("pos_config_open_modal", true.into()), ("pos_config_create_mode", true.into())])))]))))
}
