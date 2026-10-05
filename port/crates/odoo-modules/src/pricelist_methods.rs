//! product.pricelist / product.pricelist.item: rule selection and price computation
//! (ports of addons/product/models/product_pricelist.py and product_pricelist_item.py).
use crate::sale_util::*;
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};

/// `env.company`: the session's active company, else the first one.
pub fn env_company(env: &Env) -> Option<i64> {
    if let Some(Value::Int(c)) = env.ctx.get("company_id") { return Some(*c); }
    find_one(&env.sudo(), "res.company", Domain::True).ok().flatten()
}
pub fn env_company_currency(env: &Env) -> Option<i64> { company_currency(env, env_company(env)) }

/// Product price of `price_type` ("list_price" | "standard_price") in `uom`/`currency` (`product.product._price_compute`).
pub fn price_compute(env: &Env, product: i64, price_type: &str, uom: Option<i64>, currency: Option<i64>, date: &str) -> Result<f64> {
    let p = rec(env, "product.product", product)?;
    let t = tmpl_of(env, product)?;
    let mut price = if price_type == "standard_price" { if p.get("standard_price").map_or(false, |v| !v.is_null()) { num(&p, "standard_price") } else { num(&t, "standard_price") } } else { num(&t, "list_price") };
    if price_type == "list_price" { price += num(&p, "price_extra"); }
    let puom = opt_id(&t, "uom_id");
    if uom.is_some() { price = uom_compute_price(env, price, puom, uom)?; }
    if currency.is_some() { price = convert(env, price, env_company_currency(env), currency, date, true); }
    Ok(price)
}

fn category_chain(env: &Env, categ: Option<i64>) -> Vec<i64> {
    let mut out = vec![]; let mut c = categ;
    while let Some(id) = c { if out.contains(&id) { break; } out.push(id); c = rec(env, "product.category", id).ok().and_then(|r| opt_id(&r, "parent_id")); }
    out
}

/// `product.pricelist.item._is_applicable_for` (product given as a product.product id).
pub fn item_is_applicable(env: &Env, item: &Row, product: i64, qty_in_product_uom: f64) -> Result<bool> {
    let p = rec(env, "product.product", product)?;
    let t = tmpl_of(env, product)?;
    let min = num(item, "min_quantity");
    if min != 0.0 && qty_in_product_uom < min { return Ok(false); }
    Ok(match text(item, "applied_on").as_deref() {
        Some("2_product_category") => { let chain = category_chain(env, opt_id(&t, "categ_id")); opt_id(item, "categ_id").map_or(false, |c| chain.contains(&c)) }
        Some("1_product") => opt_id(&p, "product_tmpl_id") == opt_id(item, "product_tmpl_id"),
        Some("0_product_variant") => Some(product) == opt_id(item, "product_id"),
        _ => true,
    })
}

/// Items of `pricelist` that can apply to `product` on `date`, in the model order
/// `applied_on, min_quantity desc, categ_id desc, id desc` (`_get_applicable_rules`).
pub fn applicable_rules(env: &Env, pricelist: i64, product: i64, date: &str) -> Result<Vec<Row>> {
    let e = env.sudo();
    let t = tmpl_of(env, product)?;
    let p = rec(env, "product.product", product)?;
    let chain = category_chain(env, opt_id(&t, "categ_id"));
    let mut rows: Vec<Row> = children(&e, "product.pricelist.item", "pricelist_id", pricelist)?.into_iter().filter(|r| {
        let ok_categ = opt_id(r, "categ_id").map_or(true, |c| chain.contains(&c));
        let ok_tmpl = opt_id(r, "product_tmpl_id").map_or(true, |x| Some(x) == opt_id(&p, "product_tmpl_id"));
        let ok_prod = opt_id(r, "product_id").map_or(true, |x| x == product);
        let ok_start = text(r, "date_start").filter(|s| !s.is_empty()).map_or(true, |s| s.as_str() <= date);
        let ok_end = text(r, "date_end").filter(|s| !s.is_empty()).map_or(true, |s| s.as_str() >= date);
        ok_categ && ok_tmpl && ok_prod && ok_start && ok_end
    }).collect();
    rows.sort_by(|a, b| {
        text(a, "applied_on").cmp(&text(b, "applied_on"))
            .then(num(b, "min_quantity").partial_cmp(&num(a, "min_quantity")).unwrap_or(std::cmp::Ordering::Equal))
            .then(opt_id(b, "categ_id").cmp(&opt_id(a, "categ_id")))
            .then(b["id"].as_i64().cmp(&a["id"].as_i64()))
    });
    Ok(rows)
}

/// `_compute_price_rule` for one product: (price, applied rule id).
pub fn compute_price_rule(env: &Env, pricelist: Option<i64>, product: i64, quantity: f64, currency: Option<i64>, uom: Option<i64>, date: &str, compute_price: bool) -> Result<(f64, Option<i64>)> {
    let pl = pricelist.map(|i| rec(env, "product.pricelist", i)).transpose()?;
    let currency = currency.or_else(|| pl.as_ref().and_then(|p| opt_id(p, "currency_id"))).or_else(|| env_company_currency(env));
    let date = if date.is_empty() { orm::now() } else { date.to_string() };
    let t = tmpl_of(env, product)?;
    let puom = opt_id(&t, "uom_id");
    let target = uom.or(puom);
    let qty_in_product_uom = if target != puom { uom_compute_quantity(env, quantity, target, puom, true, "UP", false)? } else { quantity };
    let mut rule: Option<Row> = None;
    if let Some(pid) = pricelist {
        for r in applicable_rules(env, pid, product, &date)? { if item_is_applicable(env, &r, product, qty_in_product_uom)? { rule = Some(r); break; } }
    }
    let price = if compute_price { item_compute_price(env, rule.as_ref(), product, quantity, target, &date, currency)? } else { 0.0 };
    Ok((price, rule.and_then(|r| r["id"].as_i64())))
}
pub fn get_product_price(env: &Env, pricelist: Option<i64>, product: i64, quantity: f64, uom: Option<i64>, date: &str) -> Result<f64> { Ok(compute_price_rule(env, pricelist, product, quantity, None, uom, date, true)?.0) }
pub fn get_product_rule(env: &Env, pricelist: i64, product: i64, quantity: f64, uom: Option<i64>, date: &str) -> Result<Option<i64>> { Ok(compute_price_rule(env, Some(pricelist), product, quantity, None, uom, date, false)?.1) }

/// `_compute_base_price` (rule may be empty: list price).
pub fn item_base_price(env: &Env, item: Option<&Row>, product: i64, quantity: f64, uom: Option<i64>, date: &str, currency: Option<i64>) -> Result<f64> {
    let base = item.and_then(|i| text(i, "base")).unwrap_or_else(|| "list_price".into());
    let comp_cur = env_company_currency(env);
    let (price, src_cur) = match (base.as_str(), item.and_then(|i| opt_id(i, "base_pricelist_id"))) {
        ("pricelist", Some(bp)) => {
            let bcur = rec(env, "product.pricelist", bp).ok().and_then(|p| opt_id(&p, "currency_id"));
            (compute_price_rule(env, Some(bp), product, quantity, bcur, uom, date, true)?.0, bcur)
        }
        ("standard_price", _) => (price_compute(env, product, "standard_price", uom, None, date)?, comp_cur),
        _ => (price_compute(env, product, "list_price", uom, None, date)?, comp_cur),
    };
    Ok(if src_cur != currency { convert(env, price, src_cur, currency, date, false) } else { price })
}

/// `product.pricelist.item._compute_price`.
pub fn item_compute_price(env: &Env, item: Option<&Row>, product: i64, quantity: f64, uom: Option<i64>, date: &str, currency: Option<i64>) -> Result<f64> {
    let t = tmpl_of(env, product)?;
    let puom = opt_id(&t, "uom_id");
    let conv = |p: f64| -> Result<f64> { if puom != uom { uom_compute_price(env, p, puom, uom) } else { Ok(p) } };
    let Some(it) = item else { return item_base_price(env, None, product, quantity, uom, date, currency) };
    Ok(match text(it, "compute_price").as_deref() {
        Some("fixed") => conv(num(it, "fixed_price"))?,
        Some("percentage") => { let b = item_base_price(env, Some(it), product, quantity, uom, date, currency)?; b - b * (num(it, "percent_price") / 100.0) }
        Some("formula") => {
            let b = item_base_price(env, Some(it), product, quantity, uom, date, currency)?;
            let limit = b;
            let discount = if text(it, "base").as_deref() != Some("standard_price") { num(it, "price_discount") } else { -num(it, "price_markup") };
            let mut price = b - b * (discount / 100.0);
            if num(it, "price_round") != 0.0 { price = float_round(price, num(it, "price_round"), "HALF-UP"); }
            if num(it, "price_surcharge") != 0.0 { price += conv(num(it, "price_surcharge"))?; }
            if num(it, "price_min_margin") != 0.0 { price = price.max(limit + conv(num(it, "price_min_margin"))?); }
            if num(it, "price_max_margin") != 0.0 { price = price.min(limit + conv(num(it, "price_max_margin"))?); }
            price
        }
        _ => item_base_price(env, Some(it), product, quantity, uom, date, currency)?,
    })
}

/// `_compute_price_before_discount`: follow `base=pricelist` percentage rules down to the lowest base price.
pub fn item_price_before_discount(env: &Env, item: Option<&Row>, product: i64, quantity: f64, uom: Option<i64>, date: &str, currency: Option<i64>) -> Result<f64> {
    let mut cur: Option<Row> = item.cloned();
    while let Some(i) = cur.clone() {
        if text(&i, "base").as_deref() != Some("pricelist") { break; }
        let Some(bp) = opt_id(&i, "base_pricelist_id") else { break };
        let rid = get_product_rule(env, bp, product, quantity, uom, date)?;
        match rid.map(|r| rec(env, "product.pricelist.item", r)).transpose()? {
            Some(r) if text(&r, "compute_price").as_deref() == Some("percentage") => cur = Some(r),
            _ => break,
        }
    }
    item_base_price(env, cur.as_ref(), product, quantity, uom, date, currency)
}

/// `_is_discount_feature_enabled`: the superuser belongs to `sale.group_discount_per_so_line`.
pub fn discount_feature_enabled(env: &Env) -> bool { user_has_group(env, 1, "sale.group_discount_per_so_line") }
/// `_show_discount`.
pub fn item_show_discount(env: &Env, item: Option<&Row>) -> bool { item.map_or(false, |i| discount_feature_enabled(env) && text(i, "compute_price").as_deref() == Some("percentage")) }

/// `product.pricelist._get_partner_pricelist_multi` for one partner: specific property, else country group, else fallback.
pub fn partner_pricelist(env: &Env, partner: i64) -> Result<Option<i64>> {
    if !has_model(env, "product.pricelist") { return Ok(None); }
    let e = env.sudo().with_ctx("active_test", Value::Bool(false));
    let p = rec(&e, "res.partner", partner)?;
    let company = env_company(env);
    let ok = |pl: i64| -> bool { rec(&e, "product.pricelist", pl).map_or(false, |r| flag(&r, "active") && opt_id(&r, "company_id").map_or(true, |c| Some(c) == company)) };
    if let Some(pl) = opt_id(&p, "specific_property_product_pricelist").filter(|pl| ok(*pl)) { return Ok(Some(pl)); }
    let all = orm::search(&e, "product.pricelist", &Domain::True, Some("sequence, id"), None, 0)?;
    let valid: Vec<i64> = all.into_iter().filter(|pl| ok(*pl)).collect();
    if let Some(c) = opt_id(&p, "country_id") {
        for pl in &valid {
            let r = rec(&e, "product.pricelist", *pl)?;
            for g in ids(&r, "country_group_ids") { if ids(&rec(&e, "res.country.group", g)?, "country_ids").contains(&c) { return Ok(Some(*pl)); } }
        }
    }
    for pl in &valid { if ids(&rec(&e, "product.pricelist", *pl)?, "country_group_ids").is_empty() { return Ok(Some(*pl)); } }
    Ok(valid.first().copied())
}

fn normalize_item(mut v: Row, env: &Env) -> Result<Row> {
    let set = |v: &Row, k: &str| v.get(k).map_or(false, |x| x.truthy());
    if set(&v, "product_id") && !set(&v, "product_tmpl_id") {
        let p = rec(env, "product.product", v["product_id"].as_i64().unwrap_or(0))?;
        if let Some(t) = opt_id(&p, "product_tmpl_id") { v.insert("product_tmpl_id".into(), t.into()); }
    }
    // `applied_on` carries the field default "3_global" by the time we run: a scope field given with it means "infer"
    let scoped = set(&v, "product_id") || set(&v, "product_tmpl_id") || set(&v, "categ_id");
    if !set(&v, "applied_on") || (scoped && text(&v, "applied_on").as_deref() == Some("3_global")) {
        let a = if set(&v, "product_id") { "0_product_variant" } else if set(&v, "product_tmpl_id") { "1_product" } else if set(&v, "categ_id") { "2_product_category" } else { "3_global" };
        v.insert("applied_on".into(), a.into());
    }
    let a = text(&v, "applied_on").unwrap_or_default();
    match a.as_str() {
        "3_global" => { for k in ["product_id", "product_tmpl_id", "categ_id"] { v.insert(k.into(), Value::Null); } }
        "2_product_category" => { for k in ["product_id", "product_tmpl_id"] { v.insert(k.into(), Value::Null); } }
        "1_product" => { for k in ["product_id", "categ_id"] { v.insert(k.into(), Value::Null); } }
        "0_product_variant" => { v.insert("categ_id".into(), Value::Null); }
        _ => {}
    }
    Ok(v)
}

/// `_inverse_price_markup`: writing the markup sets `price_discount = -markup`.
fn markup_inverse(v: &mut Row) {
    // price_discount arrives with its field default (0) on create, so only an explicit non-zero discount wins
    if let Some(m) = v.get("price_markup").and_then(|m| m.as_f64()) { if v.get("price_discount").and_then(|d| d.as_f64()).unwrap_or(0.0) == 0.0 { v.insert("price_discount".into(), (-m).into()); } }
}

fn item_name(env: &Env, r: &Row) -> Result<String> {
    Ok(match text(r, "applied_on").as_deref() {
        Some("2_product_category") if opt_id(r, "categ_id").is_some() => { let c = rec(env, "product.category", opt_id(r, "categ_id").unwrap())?; format!("Category: {}", text(&c, "complete_name").or_else(|| text(&c, "name")).unwrap_or_default()) }
        Some("1_product") if opt_id(r, "product_tmpl_id").is_some() => text(&rec(env, "product.template", opt_id(r, "product_tmpl_id").unwrap())?, "name").unwrap_or_default(),
        Some("0_product_variant") if opt_id(r, "product_id").is_some() => format!("Variant: {}", product_display_name(env, opt_id(r, "product_id").unwrap())?),
        _ => if text(r, "display_applied_on").as_deref() == Some("2_product_category") { "All Categories".into() } else { "All Products".into() },
    })
}

fn check_items(env: &Env, ids_: &[i64], _v: &Row) -> Result<()> {
    for id in ids_ {
        let r = rec(env, "product.pricelist.item", *id)?;
        if text(&r, "base").as_deref() == Some("pricelist") && opt_id(&r, "base_pricelist_id").is_none() { return user_error("A pricelist item with \"Other Pricelist\" as base must have a base_pricelist_id."); }
        if text(&r, "base").as_deref() == Some("pricelist") && opt_id(&r, "pricelist_id").is_some() && opt_id(&r, "pricelist_id") == opt_id(&r, "base_pricelist_id") { return user_error("You cannot assign the Main Pricelist as Other Pricelist in PriceList Item"); }
        if let (Some(s), Some(e)) = (text(&r, "date_start").filter(|s| !s.is_empty()), text(&r, "date_end").filter(|s| !s.is_empty())) {
            if s >= e { return user_error(format!("{}: end date ({e}) should be after start date ({s})", item_name(env, &r)?)); }
        }
        if num(&r, "price_min_margin") > num(&r, "price_max_margin") { return user_error("The minimum margin should be lower than the maximum margin."); }
        match text(&r, "applied_on").as_deref() {
            Some("2_product_category") if opt_id(&r, "categ_id").is_none() => return user_error("Please specify the category for which this rule should be applied"),
            Some("1_product") if opt_id(&r, "product_tmpl_id").is_none() => return user_error("Please specify the product for which this rule should be applied"),
            Some("0_product_variant") if opt_id(&r, "product_id").is_none() => return user_error("Please specify the product variant for which this rule should be applied"),
            _ => {}
        }
    }
    Ok(())
}

fn arg_f(a: &Row, k: &str, d: f64) -> f64 { a.get(k).and_then(|v| v.as_f64()).unwrap_or(d) }
fn arg_date(a: &Row) -> String { text(a, "date").unwrap_or_default() }

pub fn rules() -> Rules {
    Rules::default()
        .before_create("product.pricelist.item", |env, v| { let mut v = normalize_item(v, env)?; markup_inverse(&mut v); Ok(v) })
        .before_write("product.pricelist.item", |env, mut v| {
            markup_inverse(&mut v);
            if let Some(a) = v.get("applied_on").and_then(|a| a.as_str()).filter(|a| !a.is_empty()).map(String::from) {
                match a.as_str() {
                    "3_global" => { for k in ["product_id", "product_tmpl_id", "categ_id"] { v.insert(k.into(), Value::Null); } }
                    "2_product_category" => { for k in ["product_id", "product_tmpl_id"] { v.insert(k.into(), Value::Null); } }
                    "1_product" => { for k in ["product_id", "categ_id"] { v.insert(k.into(), Value::Null); } }
                    "0_product_variant" => { v.insert("categ_id".into(), Value::Null); }
                    _ => {}
                }
            }
            let _ = env; Ok(v)
        })
        .after_create("product.pricelist.item", check_items)
        .after_write("product.pricelist.item", check_items)
        .compute("product.pricelist.item", "name", |env, r| Ok(item_name(env, r)?.into()))
        .compute("product.pricelist.item", "price_markup", |_, r| Ok((-num(r, "price_discount")).into()))
        .on_unlink("product.pricelist", |env, ids_| {
            let e = env.sudo().with_ctx("active_test", Value::Bool(false));
            let items = orm::search(&e, "product.pricelist.item", &term("base", "=", "pricelist").and(Domain::Term("base_pricelist_id".into(), "in".into(), Value::List(ids_.iter().map(|i| Value::Int(*i)).collect()))), None, None, 0)?;
            let mut linked = vec![]; let mut names = vec![];
            for i in items { let r = rec(&e, "product.pricelist.item", i)?; let owner = opt_id(&r, "pricelist_id"); if owner.map_or(false, |o| !ids_.contains(&o)) { linked.push(rec(&e, "product.pricelist", opt_id(&r, "base_pricelist_id").unwrap_or(0))?); names.push(text(&rec(&e, "product.pricelist", owner.unwrap())?, "name").unwrap_or_default()); } }
            if !linked.is_empty() {
                let pl: Vec<String> = linked.iter().map(|p| format!("{} ({})", text(p, "name").unwrap_or_default(), opt_id(p, "currency_id").and_then(|c| rec(&e, "res.currency", c).ok()).and_then(|c| text(&c, "name")).unwrap_or_default())).collect();
                return Err(OdooError::User(format!("You cannot delete pricelist(s):\n({})\nThey are used within pricelist(s):\n{}", pl.join("\n"), names.join("\n"))));
            }
            Ok(vec![])
        })
        // Python-callable price API: args {product_id, quantity?, uom_id?, date?, currency_id?}; result {price, rule_id}
        .action("product.pricelist", "_get_product_price", |env, ids_, a| {
            let pid = a.get("product_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("product_id required".into()))?;
            Ok(get_product_price(env, ids_.first().copied(), pid, arg_f(a, "quantity", 1.0), a.get("uom_id").and_then(|v| v.as_i64()), &arg_date(a))?.into())
        })
        .action("product.pricelist", "_get_product_price_rule", |env, ids_, a| {
            let pid = a.get("product_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("product_id required".into()))?;
            let (p, r) = compute_price_rule(env, ids_.first().copied(), pid, arg_f(a, "quantity", 1.0), a.get("currency_id").and_then(|v| v.as_i64()), a.get("uom_id").and_then(|v| v.as_i64()), &arg_date(a), true)?;
            Ok(Value::List(vec![p.into(), idv(r)]))
        })
        .action("product.pricelist", "_get_product_rule", |env, ids_, a| {
            let pid = a.get("product_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("product_id required".into()))?;
            let pl = ids_.first().copied().ok_or_else(|| OdooError::User("pricelist required".into()))?;
            Ok(match get_product_rule(env, pl, pid, arg_f(a, "quantity", 1.0), a.get("uom_id").and_then(|v| v.as_i64()), &arg_date(a))? { Some(r) => Value::Int(r), None => Value::Bool(false) })
        })
        .action("product.pricelist", "_get_partner_pricelist_multi", |env, _, a| {
            let mut out = Row::new();
            if let Some(Value::List(ps)) = a.get("partner_ids") { for p in ps.iter().filter_map(|v| v.as_i64()) { out.insert(p.to_string(), idv(partner_pricelist(env, p)?)); } }
            Ok(Value::Map(out.into_iter().collect()))
        })
        .action("product.pricelist.item", "_is_applicable_for", |env, ids_, a| {
            let pid = a.get("product_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("product_id required".into()))?;
            let it = rec(env, "product.pricelist.item", *ids_.first().ok_or_else(|| OdooError::User("item required".into()))?)?;
            Ok(item_is_applicable(env, &it, pid, arg_f(a, "qty_in_product_uom", 1.0))?.into())
        })
        .action("product.pricelist.item", "_compute_price", |env, ids_, a| {
            let pid = a.get("product_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("product_id required".into()))?;
            let it = ids_.first().map(|i| rec(env, "product.pricelist.item", *i)).transpose()?;
            let t = tmpl_of(env, pid)?;
            let uom = a.get("uom_id").and_then(|v| v.as_i64()).or_else(|| opt_id(&t, "uom_id"));
            let cur = a.get("currency_id").and_then(|v| v.as_i64()).or_else(|| it.as_ref().and_then(|i| opt_id(i, "currency_id"))).or_else(|| env_company_currency(env));
            Ok(item_compute_price(env, it.as_ref(), pid, arg_f(a, "quantity", 1.0), uom, &arg_date(a), cur)?.into())
        })
}
