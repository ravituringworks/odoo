//! pos_loyalty: points earning, coupons, gift cards, eWallets and promotions for the POS.
//! Rewards are always recomputed here from the program definition; the terminal's preview is only a convenience.
use crate::{tax, util::*};
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Value};
use std::collections::BTreeMap;

/// One sold (non-reward) order line, with the facts rules/rewards look at.
#[derive(Clone, Debug)]
pub struct Item { pub product: i64, pub categ: Option<i64>, pub qty: f64, pub price: f64, pub tax_ids: Vec<i64>, pub untaxed: f64, pub total: f64 }

#[derive(Debug, Default)]
pub struct Outcome {
    pub lines: Vec<Row>,                 // pos.order.line values (reward lines, plus free-product lines)
    pub spends: Vec<(i64, f64, i64)>,    // (card, points, reward)
    pub earns: Vec<(i64, f64)>,          // (program, points)
}

pub fn available(env: &Env) -> bool { env.reg.model("loyalty.program").is_ok() && env.reg.field("loyalty.program", "pos_ok").is_ok() }

fn today() -> String { orm::today() }
/// Programs usable on this register today.
pub fn programs(env: &Env, config: i64) -> Result<Vec<Row>> {
    let e = env.sudo();
    let mut out = vec![];
    for id in orm::search(&e, "loyalty.program", &Domain::And(vec![term("active", "=", true), term("pos_ok", "=", true)]), Some("sequence, id"), None, 0)? {
        let p = rec(&e, "loyalty.program", id)?;
        let cfgs = ids(&p, "pos_config_ids"); if !cfgs.is_empty() && !cfgs.contains(&config) { continue; }
        let t = today();
        if text(&p, "date_from").map_or(false, |d| !d.is_empty() && d > t) || text(&p, "date_to").map_or(false, |d| !d.is_empty() && d < t) { continue; }
        out.push(p);
    }
    Ok(out)
}

fn categ_under(env: &Env, mut c: Option<i64>, target: i64) -> bool {
    let mut guard = 0;
    while let Some(id) = c { if id == target { return true; } guard += 1; if guard > 32 { break; } c = rec(env, "product.category", id).ok().and_then(|r| id_of(&r, "parent_id")); }
    false
}
fn rule_matches(env: &Env, r: &Row, i: &Item) -> bool {
    let (prods, valid) = (ids(r, "product_ids"), ids(r, "valid_product_ids"));
    let restricted = !flag(r, "any_product") && (!prods.is_empty() || id_of(r, "product_category_id").is_some());
    if !restricted { return true; }
    if prods.contains(&i.product) || valid.contains(&i.product) { return true; }
    id_of(r, "product_category_id").map_or(false, |c| categ_under(env, i.categ, c))
}
fn flag(r: &Row, k: &str) -> bool { r.get(k).map_or(false, |v| v.truthy()) }

/// Points one rule yields for the sold items.
pub fn rule_points(env: &Env, r: &Row, items: &[Item]) -> f64 {
    let el: Vec<&Item> = items.iter().filter(|i| rule_matches(env, r, i)).collect();
    if el.is_empty() { return 0.0; }
    let qty: f64 = el.iter().map(|i| i.qty).sum();
    let amt: f64 = el.iter().map(|i| if text(r, "minimum_amount_tax_mode").as_deref() == Some("excl") { i.untaxed } else { i.total }).sum();
    if qty < num(r, "minimum_qty") || amt < num(r, "minimum_amount") { return 0.0; }
    let k = num(r, "reward_point_amount");
    match text(r, "reward_point_mode").as_deref() { Some("money") => amt * k, Some("unit") => qty * k, _ => k }
}
pub fn program_points(env: &Env, p: &Row, items: &[Item], codes: &[String]) -> Result<f64> {
    let mut total = 0.0;
    for rid in ids(p, "rule_ids") {
        let r = rec(env, "loyalty.rule", rid)?;
        if text(&r, "mode").as_deref() == Some("with_code") || text(p, "trigger").as_deref() == Some("with_code") {
            let code = text(&r, "code").unwrap_or_default();
            if code.is_empty() || !codes.iter().any(|c| c.eq_ignore_ascii_case(&code)) { continue; }
        }
        total += rule_points(env, &r, items);
    }
    Ok(r2(total))
}

fn group_by_tax(items: &[&Item], amount_of: impl Fn(&Item) -> f64) -> BTreeMap<Vec<i64>, f64> {
    let mut g: BTreeMap<Vec<i64>, f64> = BTreeMap::new();
    for i in items { let mut k = i.tax_ids.clone(); k.sort(); *g.entry(k).or_default() += amount_of(i); }
    g
}

/// Lines (taxes, negative untaxed amount) a reward grants, plus the points it costs. `balance` = points the customer can spend.
pub fn reward_discount(env: &Env, reward: &Row, items: &[Item], balance: f64) -> Result<(Vec<(Vec<i64>, f64)>, f64)> {
    let all: Vec<&Item> = items.iter().collect();
    let eligible: Vec<&Item> = match text(reward, "discount_applicability").as_deref() {
        Some("specific") => {
            let prods = { let mut v = ids(reward, "discount_product_ids"); v.extend(ids(reward, "all_discount_product_ids")); v };
            let cat = id_of(reward, "discount_product_category_id");
            all.iter().copied().filter(|i| prods.contains(&i.product) || cat.map_or(false, |c| categ_under(env, i.categ, c))).collect()
        }
        Some("cheapest") => all.iter().copied().filter(|i| i.qty > 0.0).min_by(|a, b| (a.untaxed / a.qty).partial_cmp(&(b.untaxed / b.qty)).unwrap()).into_iter().collect(),
        _ => all.clone(),
    };
    let cheapest = text(reward, "discount_applicability").as_deref() == Some("cheapest");
    let base_of = |i: &Item| if cheapest { i.untaxed / i.qty.max(1.0) } else { i.untaxed };
    let base: f64 = eligible.iter().map(|i| base_of(i)).sum();
    if base <= 0.0 { return Err(OdooError::User("This reward does not apply to anything in the order".into())); }
    let d = num(reward, "discount"); let req = num(reward, "required_points");
    let (mut amount, cost) = match text(reward, "discount_mode").as_deref() {
        Some("per_point") => { let spend_pts = balance; let a = (d * spend_pts).min(base); (a, if d > 0.0 { a / d } else { 0.0 }) }
        Some("per_order") => (d.min(base), req),
        _ => (base * d / 100.0, req),
    };
    let cap = num(reward, "discount_max_amount"); if cap > 0.0 { amount = amount.min(cap); }
    let amount = r2(amount);
    let groups = group_by_tax(&eligible, base_of);
    Ok((groups.into_iter().map(|(k, v)| (k, -r2(amount * v / base))).collect(), cost))
}

fn reward_product_line(env: &Env, reward: &Row, items: &[Item]) -> Result<(Option<Row>, Vec<(Vec<i64>, f64)>)> {
    let pid = id_of(reward, "reward_product_id").or_else(|| ids(reward, "reward_product_ids").first().copied()).ok_or_else(|| OdooError::User("The reward has no product".into()))?;
    let want = num(reward, "reward_product_qty").max(1.0);
    let p = rec(env, "product.product", pid)?; let t = id_of(&p, "product_tmpl_id").map(|t| rec(env, "product.template", t)).transpose()?.unwrap_or_default();
    let (price, tx) = (num(&t, "list_price"), ids(&t, "taxes_id"));
    let in_cart: f64 = items.iter().filter(|i| i.product == pid).map(|i| i.qty).sum();
    let free = want.min(in_cart.max(want * if in_cart >= want { 1.0 } else { 0.0 }));
    // free product already in the basket → discount it; otherwise add it and discount it entirely
    let (add, qty) = if in_cart >= want { (None, free) } else { (Some(row(&[("product_id", pid.into()), ("qty", want.into()), ("price_unit", price.into())])), want) };
    let (u, _, _) = tax::compute(qty, price, 0.0, &tax::load(env, &tx)?);
    Ok((add, vec![(tx, -u)]))
}

fn line_vals(env: &Env, product: i64, name: &str, qty: f64, price: f64, taxes: &[i64], extra: &[(&str, Value)]) -> Result<Row> {
    let (u, _, tot) = tax::compute(qty, price, 0.0, &tax::load(env, taxes)?);
    let mut v = row(&[("product_id", product.into()), ("full_product_name", name.into()), ("name", name.into()), ("qty", qty.into()), ("price_unit", price.into()), ("discount", 0.0.into()), ("price_subtotal", u.into()), ("price_subtotal_incl", tot.into()), ("tax_ids", Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(taxes.iter().map(|i| Value::Int(*i)).collect())])]))]);
    for (k, x) in extra { v.insert(k.to_string(), x.clone()); }
    Ok(v)
}
fn discount_product(env: &Env, reward: &Row) -> Result<i64> {
    if let Some(p) = id_of(reward, "discount_line_product_id") { return Ok(p); }
    find_or_create(env, "product.product", term("name", "=", "Discount"), row(&[("name", "Discount".into()), ("type", "service".into()), ("list_price", 0.0.into()), ("available_in_pos", false.into())]))
}

fn fresh_code(seed: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new(); seed.hash(&mut h); std::time::SystemTime::now().hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Validate every requested reward, price it, and work out the points this order earns.
pub fn process(env: &Env, config: i64, partner: Option<i64>, kw: &Row, items: &[Item], refund: bool) -> Result<Outcome> {
    let mut out = Outcome::default();
    if !available(env) { return Ok(out); }
    let progs = programs(env, config)?;
    let codes: Vec<String> = match kw.get("codes") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_str().map(String::from)).collect(), _ => vec![] };
    let mut spent: BTreeMap<i64, f64> = BTreeMap::new();
    // 1. rewards being redeemed
    if !refund { if let Some(Value::List(reqs)) = kw.get("rewards") { for rq in reqs {
        let rq = if let Value::Map(m) = rq { m.clone() } else { continue };
        let rid = rq.get("reward_id").and_then(|v| v.as_i64()).ok_or_else(|| OdooError::User("Reward without id".into()))?;
        let reward = rec(env, "loyalty.reward", rid)?;
        let pid = id_of(&reward, "program_id").unwrap_or(0);
        let prog = progs.iter().find(|p| p["id"].as_i64() == Some(pid)).ok_or_else(|| OdooError::User("That promotion is not available on this register".into()))?;
        let (card, balance) = if let Some(c) = rq.get("card_id").and_then(|v| v.as_i64()) {
            let card = rec(env, "loyalty.card", c)?;
            if id_of(&card, "program_id") != Some(pid) { return Err(OdooError::User("That card does not belong to this program".into())); }
            if !flag(&card, "active") { return Err(OdooError::User("That card is no longer valid".into())); }
            if text(&card, "expiration_date").map_or(false, |d| !d.is_empty() && d < today()) { return Err(OdooError::User("That card has expired".into())); }
            if let (Some(cp), Some(op)) = (id_of(&card, "partner_id"), partner) { if cp != op { return Err(OdooError::User("That card belongs to another customer".into())); } }
            if id_of(&card, "partner_id").is_some() && partner.is_none() { return Err(OdooError::User("Select the card's customer first".into())); }
            let bal = num(&card, "points") - spent.get(&c).copied().unwrap_or(0.0);
            (Some(c), bal)
        } else {   // automatic promotion: points come from this very order
            (None, program_points(env, prog, items, &codes)?)
        };
        if balance + 1e-9 < num(&reward, "required_points") { return Err(OdooError::User(format!("Not enough points: {:.2} available, {:.2} needed", balance, num(&reward, "required_points")))); }
        let name = text(&reward, "description").unwrap_or_else(|| text(prog, "name").unwrap_or_default());
        let dp = discount_product(env, &reward)?;
        let (groups, cost) = if text(&reward, "reward_type").as_deref() == Some("product") {
            let (add, g) = reward_product_line(env, &reward, items)?;
            if let Some(a) = add { let pid2 = a["product_id"].as_i64().unwrap(); let p = rec(env, "product.product", pid2)?; let t = id_of(&p, "product_tmpl_id").map(|t| rec(env, "product.template", t)).transpose()?.unwrap_or_default();
                out.lines.push(line_vals(env, pid2, &text(&t, "name").unwrap_or_default(), num(&a, "qty"), num(&a, "price_unit"), &ids(&t, "taxes_id"), &[])?); }
            (g, num(&reward, "required_points"))
        } else { reward_discount(env, &reward, items, balance)? };
        for (tx, amt) in groups { if amt == 0.0 { continue; }
            out.lines.push(line_vals(env, dp, &name, 1.0, amt, &tx, &[("is_reward_line", true.into()), ("reward_id", rid.into()), ("points_cost", cost.into()), ("coupon_id", card.map_or(Value::Null, Value::Int))])?); }
        if let Some(c) = card { *spent.entry(c).or_default() += cost; out.spends.push((c, cost, rid)); }
    } } }
    // 2. points earned by the sold items (never from reward lines)
    if !refund { for p in &progs {
        let pts = program_points(env, p, items, &codes)?;
        if pts > 0.0 && matches!(text(p, "applies_on").as_deref(), Some("future" | "both")) { out.earns.push((p["id"].as_i64().unwrap(), pts)); }
    } }
    Ok(out)
}

fn history(env: &Env, card: i64, issued: f64, used: f64, order: i64, why: &str) -> Result<()> {
    if env.reg.model("loyalty.history").is_err() { return Ok(()); }
    let mut v = row(&[("card_id", card.into()), ("issued", issued.into()), ("used", used.into()), ("order_id", order.into()), ("order_model", "pos.order".into()), ("description", why.into())]);
    v.retain(|k, _| env.reg.field("loyalty.history", k).is_ok());
    orm::create(env, "loyalty.history", v).map(|_| ())
}

/// Persist spends and earnings once the order exists.
pub fn commit(env: &Env, order: i64, name: &str, partner: Option<i64>, o: &Outcome) -> Result<Vec<Value>> {
    let mut issued = vec![];
    for (card, pts, _) in &o.spends {
        let c = rec(env, "loyalty.card", *card)?;
        orm::write(env, "loyalty.card", &[*card], row(&[("points", r2(num(&c, "points") - pts).into())]))?;
        history(env, *card, 0.0, *pts, order, &format!("Used on {name}"))?;
    }
    for (pid, pts) in &o.earns {
        let p = rec(env, "loyalty.program", *pid)?;
        let kind = text(&p, "program_type").unwrap_or_default();
        // gift cards / next-order coupons / eWallet top-ups mint a fresh card; loyalty accumulates on the customer's card
        let nominative = kind == "loyalty" || kind == "ewallet" || flag(&p, "is_nominative");
        let existing = match (nominative, partner) {
            (true, Some(pt)) => find_one(env, "loyalty.card", Domain::And(vec![term("program_id", "=", *pid), term("partner_id", "=", pt)]))?,
            (true, None) => continue,
            _ => None,
        };
        let card = match existing {
            Some(c) => { let r = rec(env, "loyalty.card", c)?; orm::write(env, "loyalty.card", &[c], row(&[("points", r2(num(&r, "points") + pts).into())]))?; c }
            None => { let mut v = row(&[("program_id", (*pid).into()), ("points", (*pts).into()), ("code", fresh_code(name).into())]);
                if nominative { if let Some(pt) = partner { v.insert("partner_id".into(), pt.into()); } }
                if env.reg.field("loyalty.card", "source_pos_order_id").is_ok() { v.insert("source_pos_order_id".into(), order.into()); }
                orm::create(env, "loyalty.card", v)? }
        };
        history(env, card, *pts, 0.0, order, &format!("Earned on {name}"))?;
        let c = rec(env, "loyalty.card", card)?;
        issued.push(Value::Map(row(&[("card_id", card.into()), ("program", text(&p, "name").unwrap_or_default().into()), ("type", kind.into()), ("code", text(&c, "code").unwrap_or_default().into()), ("points", num(&c, "points").into()), ("earned", (*pts).into())])));
    }
    Ok(issued)
}

/// A refund undoes the original order's loyalty movements.
pub fn reverse(env: &Env, original: i64, refund: i64) -> Result<()> {
    if !available(env) || env.reg.model("loyalty.history").is_err() { return Ok(()); }
    for h in children_where(env, original)? {
        let card = match id_of(&h, "card_id") { Some(c) => c, None => continue };
        let c = rec(env, "loyalty.card", card)?;
        orm::write(env, "loyalty.card", &[card], row(&[("points", r2(num(&c, "points") - num(&h, "issued") + num(&h, "used")).into())]))?;
        history(env, card, num(&h, "used"), num(&h, "issued"), refund, "Refund")?;
    }
    Ok(())
}
fn children_where(env: &Env, order: i64) -> Result<Vec<Row>> {
    let ids = orm::search(env, "loyalty.history", &Domain::And(vec![term("order_id", "=", order), term("order_model", "=", "pos.order"), term("description", "not ilike", "Refund")]), None, None, 0)?;
    ids.into_iter().map(|i| rec(env, "loyalty.history", i)).collect()
}

/// Cards a customer (or a scanned code) can spend, for the terminal.
pub fn cards(env: &Env, config: i64, partner: Option<i64>, code: Option<&str>) -> Result<Vec<Value>> {
    let e = env.sudo();
    let progs: Vec<i64> = programs(&e, config)?.iter().filter_map(|p| p["id"].as_i64()).collect();
    let mut dom = vec![term("active", "=", true), term("program_id", "in", Value::List(progs.iter().map(|i| Value::Int(*i)).collect()))];
    match (partner, code) {
        (_, Some(c)) if !c.is_empty() => dom.push(term("code", "=", c)),
        (Some(p), _) => dom.push(term("partner_id", "=", p)),
        _ => return Ok(vec![]),
    }
    let mut out = vec![];
    for id in orm::search(&e, "loyalty.card", &Domain::And(dom), Some("id"), None, 0)? {
        let c = rec(&e, "loyalty.card", id)?;
        if text(&c, "expiration_date").map_or(false, |d| !d.is_empty() && d < today()) { continue; }
        out.push(Value::Map(row(&[("id", id.into()), ("code", text(&c, "code").unwrap_or_default().into()), ("points", num(&c, "points").into()), ("program_id", id_of(&c, "program_id").unwrap_or(0).into()), ("partner_id", id_of(&c, "partner_id").map_or(Value::Null, Value::Int))])));
    }
    Ok(out)
}
