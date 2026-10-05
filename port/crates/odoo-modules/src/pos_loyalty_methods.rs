//! pos_loyalty back-office behaviour ported from `addons/pos_loyalty/models`: program availability per register, coupon code
//! checks, order-side coupon confirmation with loyalty history, session opening checks and the computes of programs and rules.
//! The terminal runtime (`pos_loyalty.rs`) computes rewards itself; these are the server methods Odoo's own POS app calls.
//!
//! Not ported: creation mails and printed gift card reports (no mail/report engine), tag-based rule products.
use crate::pos_methods::{flag, has_model, id_list, user_err};
use crate::util::*;
use odoo_core::orm::{self, Env};
use odoo_core::{Domain, OdooError, Result, Row, Rules, Value};
use std::collections::{BTreeMap, BTreeSet};

fn has_loyalty(env: &Env) -> bool { has_model(env, "loyalty.program") && env.reg.field("loyalty.program", "pos_ok").is_ok() }
fn first(ids_: &[i64]) -> Result<i64> { ids_.first().copied().ok_or_else(|| OdooError::User("Select a record".into())) }
fn as_map(v: Option<&Value>) -> Row { if let Some(Value::Map(m)) = v { m.clone() } else { Row::new() } }
fn clean(x: f64) -> f64 { (x * 1e6).round() / 1e6 }
/// `loyalty.card._generate_code`: `'044' + str(uuid4())[7:-18]`.
pub fn generate_code() -> String { let u = crate::pos_order_methods::new_uuid(); format!("044{}", &u[7..18]) }

// ---- programs ---------------------------------------------------------------------------------------------------------
/// `pos_order_count`: per reward, the distinct POS orders with a line of that reward, summed.
fn program_pos_order_count(env: &Env, p: i64) -> Result<i64> {
    let e = env.sudo(); let mut n = 0;
    for r in children(&e, "loyalty.reward", "program_id", p)? {
        let mut orders: BTreeSet<i64> = BTreeSet::new();
        for l in children(&e, "pos.order.line", "reward_id", r["id"].as_i64().unwrap())? { if let Some(o) = id_of(&l, "order_id") { orders.insert(o); } }
        n += orders.len() as i64;
    }
    Ok(n)
}
fn available_pricelists(env: &Env, cfg: &Row) -> Vec<i64> {
    if flag(cfg, "use_pricelist") { ids(cfg, "available_pricelist_ids") } else { id_of(cfg, "pricelist_id").into_iter().collect() }
}
/// `pos.config._get_program_ids`: the programs this register may apply right now.
pub fn config_programs(env: &Env, cfg_id: i64) -> Result<Vec<Row>> {
    let e = env.sudo(); let cfg = rec(&e, "pos.config", cfg_id)?; let today = orm::today(); let pls = available_pricelists(&e, &cfg);
    let mut out = vec![];
    for pid in orm::search(&e, "loyalty.program", &term("pos_ok", "=", true), Some("sequence, id"), None, 0)? {
        let p = rec(&e, "loyalty.program", pid)?;
        let cfgs = ids(&p, "pos_config_ids"); if !cfgs.is_empty() && !cfgs.contains(&cfg_id) { continue; }
        if text(&p, "date_from").map_or(false, |d| !d.is_empty() && d > today) || text(&p, "date_to").map_or(false, |d| !d.is_empty() && d < today) { continue; }
        let ppl = ids(&p, "pricelist_ids"); if !ppl.is_empty() && !ppl.iter().any(|x| pls.contains(x)) { continue; }
        if flag(&p, "limit_usage") && program_pos_order_count(&e, pid)? >= p.get("max_usage").and_then(|v| v.as_i64()).unwrap_or(0) { continue; }
        out.push(p);
    }
    Ok(out)
}

// ---- rules ------------------------------------------------------------------------------------------------------------
fn rule_domain_is_default(d: &str) -> bool { let d = d.trim(); d.is_empty() || d == "[]" || d == "[['sale_ok', '=', True]]" }
/// `any_product`: a rule without product, category, tag or domain restriction matches everything.
fn rule_any_product(r: &Row) -> bool {
    let restricted = !ids(r, "product_ids").is_empty() || id_of(r, "product_category_id").is_some() || id_of(r, "product_tag_id").is_some() || !rule_domain_is_default(&text(r, "product_domain").unwrap_or_default());
    !restricted
}
fn categ_children(env: &Env, root: i64) -> Result<Vec<i64>> {
    let mut all = vec![root]; let mut frontier = vec![root];
    while !frontier.is_empty() {
        let kids = orm::search(&env.sudo(), "product.category", &term("parent_id", "in", id_list(&frontier)), None, None, 0)?;
        frontier = kids.into_iter().filter(|k| !all.contains(k)).collect(); all.extend(frontier.iter().copied());
    }
    Ok(all)
}
/// `valid_product_ids`: products available in the POS that the rule's products / category / domain select.
pub fn rule_valid_products(env: &Env, r: &Row) -> Result<Vec<i64>> {
    if rule_any_product(r) { return Ok(vec![]); }
    let e = env.sudo(); let mut sets: Vec<Domain> = vec![];
    if !ids(r, "product_ids").is_empty() { sets.push(term("id", "in", id_list(&ids(r, "product_ids")))); }
    if let Some(c) = id_of(r, "product_category_id") { sets.push(term("categ_id", "in", id_list(&categ_children(&e, c)?))); }
    let mut dom = if sets.is_empty() { Domain::True } else { Domain::Or(sets) };
    // a simple JSON domain (list of [field, op, value]) narrows further
    if let Some(Ok(serde_json::Value::Array(terms))) = text(r, "product_domain").filter(|d| !rule_domain_is_default(d)).map(|d| serde_json::from_str::<serde_json::Value>(&d.replace('\'', "\"").replace("True", "true").replace("False", "false"))) {
        let mut extra = vec![];
        for t in terms { if let serde_json::Value::Array(x) = t { if let (Some(f), Some(op)) = (x.first().and_then(|v| v.as_str()), x.get(1).and_then(|v| v.as_str())) { extra.push(Domain::Term(f.into(), op.into(), Value::from_json(x.get(2).unwrap_or(&serde_json::Value::Null)))); } } }
        if !extra.is_empty() { extra.push(dom); dom = Domain::And(extra); }
    }
    orm::search(&e, "product.product", &Domain::And(vec![term("available_in_pos", "=", true), dom]), Some("id"), None, 0)
}

// ---- program checks before a session opens -----------------------------------------------------------------------------
fn plan_template(env: &Env, p: i64) -> Result<(bool, bool)> {
    let plans = if has_model(env, "loyalty.mail") { children(env, "loyalty.mail", "program_id", p)? } else { vec![] };
    let mail = plans.iter().any(|x| id_of(x, "mail_template_id").is_some());
    let report = plans.iter().any(|x| id_of(x, "pos_report_print_id").is_some());
    Ok((mail, report))
}
/// `pos.config._check_before_creating_new_session` (the loyalty part): reward and gift card products must be sellable.
pub fn check_programs(env: &Env, cfg: i64) -> Result<()> {
    if !has_loyalty(env) { return Ok(()); }
    let e = env.sudo(); let programs = config_programs(&e, cfg)?; let mut msg = String::new();
    for p in &programs {
        for rid in ids(p, "reward_ids") {
            let r = rec(&e, "loyalty.reward", rid)?;
            if text(&r, "reward_type").as_deref() != Some("product") { continue; }
            for pid in ids(&r, "reward_product_ids") {
                let pr = rec(&e, "product.product", pid)?;
                let tmpl = id_of(&pr, "product_tmpl_id").and_then(|t| rec(&e, "product.template", t).ok()).unwrap_or_default();
                if flag(&tmpl, "available_in_pos") { continue; }
                msg += &format!("\n\t{}", format!("Program: {}, Reward Product: `{}`", text(p, "name").unwrap_or_default(), text(&tmpl, "name").unwrap_or_default()));
            }
        }
    }
    let gift: Vec<&Row> = programs.iter().filter(|p| text(p, "program_type").as_deref() == Some("gift_card")).collect();
    for p in &gift {
        for rid in ids(p, "rule_ids") {
            let rule = rec(&e, "loyalty.rule", rid)?;
            // valid_product_ids only lists products already available in the POS, so look at the rule's own products too
            for pid in ids(&rule, "product_ids") {
                let pr = rec(&e, "product.product", pid)?; let tmpl = id_of(&pr, "product_tmpl_id").and_then(|t| rec(&e, "product.template", t).ok()).unwrap_or_default();
                if flag(&tmpl, "available_in_pos") { continue; }
                msg += &format!("\n\t{}", format!("Program: {}, Rule Product: `{}`", text(p, "name").unwrap_or_default(), text(&tmpl, "name").unwrap_or_default()));
            }
        }
    }
    if !msg.is_empty() { return user_err(format!("To continue, make the following reward products available in Point of Sale.\n{msg}")); }
    for p in gift {
        let (rules, rewards) = (ids(p, "rule_ids"), ids(p, "reward_ids"));
        if rewards.len() > 1 { return user_err("Invalid gift card program. More than one reward."); }
        if rules.len() > 1 { return user_err("Invalid gift card program. More than one rule."); }
        if let Some(r) = rules.first().map(|r| rec(&e, "loyalty.rule", *r)).transpose()? {
            if num(&r, "reward_point_amount") != 1.0 || text(&r, "reward_point_mode").as_deref() != Some("money") { return user_err("Invalid gift card program rule. Use 1 point per currency spent."); }
        }
        if let Some(r) = rewards.first().map(|r| rec(&e, "loyalty.reward", *r)).transpose()? {
            if text(&r, "reward_type").as_deref() != Some("discount") || text(&r, "discount_mode").as_deref() != Some("per_point") || num(&r, "discount") != 1.0 { return user_err("Invalid gift card program reward. Use 1 currency per point discount."); }
        }
        let (mail, report) = plan_template(&e, p["id"].as_i64().unwrap())?;
        if !mail { return user_err("There is no email template on the gift card program and your pos is set to print them."); }
        if !report { return user_err("There is no print report on the gift card program and your pos is set to print them."); }
    }
    Ok(())
}

// ---- coupon codes -----------------------------------------------------------------------------------------------------
fn use_coupon_code(env: &Env, cfg: i64, code: &str, creation_date: &str, partner: Option<i64>, pricelist: Option<i64>) -> Result<Value> {
    let e = env.sudo(); let programs = config_programs(&e, cfg)?; let pids: Vec<i64> = programs.iter().filter_map(|p| p["id"].as_i64()).collect();
    let mut cards: Vec<Row> = children_by(&e, "loyalty.card", &Domain::And(vec![term("code", "=", code), term("program_id", "in", id_list(&pids))]))?;
    cards.retain(|c| { let ptype = id_of(c, "program_id").and_then(|p| programs.iter().find(|x| x["id"].as_i64() == Some(p))).and_then(|p| text(p, "program_type")); id_of(c, "partner_id").is_none() || id_of(c, "partner_id") == partner || ptype.as_deref() == Some("gift_card") });
    // the customer's own card first, then points descending (a coupon can be used several times in coupon mode)
    cards.sort_by(|a, b| id_of(a, "partner_id").is_none().cmp(&id_of(b, "partner_id").is_none()).then(id_of(a, "partner_id").cmp(&id_of(b, "partner_id"))).then(num(b, "points").partial_cmp(&num(a, "points")).unwrap_or(std::cmp::Ordering::Equal)));
    let fail = |m: String| Value::Map(row(&[("successful", false.into()), ("payload", Value::Map(row(&[("error_message", m.into())])))]));
    let Some(card) = cards.first() else { return Ok(fail(format!("This coupon is invalid ({code})."))) };
    let program = rec(&e, "loyalty.program", id_of(card, "program_id").unwrap_or(0))?;
    if !flag(&program, "active") { return Ok(fail(format!("This coupon is invalid ({code})."))); }
    let check_date: String = creation_date.chars().take(10).collect(); let today = orm::today();
    let rewards = crate::pos_methods::many(&e, "loyalty.reward", &ids(&program, "reward_ids"))?;
    let used = program_pos_order_count(&e, program["id"].as_i64().unwrap())?;
    let mut error: Option<String> = None;
    if text(card, "expiration_date").map_or(false, |d| !d.is_empty() && d < check_date) || text(&program, "date_to").map_or(false, |d| !d.is_empty() && d < today) || (flag(&program, "limit_usage") && used >= program.get("max_usage").and_then(|v| v.as_i64()).unwrap_or(0)) {
        error = Some(format!("This coupon is expired ({code})."));
    } else if text(&program, "date_from").map_or(false, |d| !d.is_empty() && d > today) {
        error = Some(format!("This coupon is not yet valid ({code})."));
    } else if rewards.is_empty() || !rewards.iter().any(|r| num(r, "required_points") <= num(card, "points")) {
        error = Some("No reward can be claimed with this coupon.".into());
    } else if !ids(&program, "pricelist_ids").is_empty() && !pricelist.map_or(false, |p| ids(&program, "pricelist_ids").contains(&p)) {
        error = Some("This coupon is not available with the current pricelist.".into());
    } else if text(&program, "program_type").as_deref() == Some("promo_code") {
        error = Some("This programs requires a code to be applied.".into());
    }
    if let Some(m) = error { return Ok(fail(m)); }
    Ok(Value::Map(row(&[("successful", true.into()), ("payload", Value::Map(row(&[
        ("program_id", program["id"].clone()), ("coupon_id", card["id"].clone()), ("coupon_partner_id", id_of(card, "partner_id").map_or(Value::Bool(false), Value::Int)),
        ("points", card.get("points").cloned().unwrap_or(0.0.into())), ("has_source_order", id_of(card, "source_pos_order_id").is_some().into())])))])))
}
fn children_by(e: &Env, model: &str, dom: &Domain) -> Result<Vec<Row>> { orm::search(e, model, dom, None, None, 0)?.into_iter().map(|i| rec(e, model, i)).collect() }

// ---- orders ------------------------------------------------------------------------------------------------------------
/// `validate_coupon_programs`: the cards used must still exist with enough points, and sold codes must be new.
fn validate_coupons(env: &Env, point_changes: &Row, new_codes: &[String]) -> Result<Value> {
    let e = env.sudo();
    let wanted: BTreeMap<i64, f64> = point_changes.iter().filter_map(|(k, v)| Some((k.parse::<i64>().ok()?, v.as_f64()?))).collect();
    let mut found: BTreeMap<i64, Row> = BTreeMap::new();
    for id in wanted.keys() {
        if let Ok(c) = rec(&e, "loyalty.card", *id) { if id_of(&c, "program_id").and_then(|p| rec(&e, "loyalty.program", p).ok()).map_or(false, |p| flag(&p, "active")) { found.insert(*id, c); } }
    }
    let diff: Vec<i64> = wanted.keys().copied().filter(|k| !found.contains_key(k)).collect();
    let fail = |payload: Row| Value::Map(row(&[("successful", false.into()), ("payload", Value::Map(payload))]));
    if !diff.is_empty() { return Ok(fail(row(&[("message", "Some coupons are invalid. The applied coupons have been updated. Please check the order.".into()), ("removed_coupons", id_list(&diff))]))); }
    for (id, c) in &found {
        if num(c, "points") < -wanted[id] - 0.005 {
            let updated: Row = found.iter().map(|(i, c)| (i.to_string(), c.get("points").cloned().unwrap_or(0.0.into()))).collect();
            return Ok(fail(row(&[("message", format!("There are not enough points for the coupon: {}.", text(c, "code").unwrap_or_default()).into()), ("updated_points", Value::Map(updated))])));
        }
    }
    let dup = children_by(&e, "loyalty.card", &term("code", "in", Value::List(new_codes.iter().map(|c| Value::Text(c.clone())).collect())))?;
    if !dup.is_empty() { return Ok(fail(row(&[("message", format!("The following codes already exist in the database, perhaps they were already sold?\n{}", dup.iter().filter_map(|c| text(c, "code")).collect::<Vec<_>>().join(", ")).into())]))); }
    Ok(Value::Map(row(&[("successful", true.into()), ("payload", Value::Map(Row::new()))])))
}
fn add_history_lines(env: &Env, oid: i64, coupon_data: &[Value], updates: &[Value]) -> Result<()> {
    let e = env.sudo();
    let mapping: BTreeMap<i64, i64> = updates.iter().filter_map(|u| { let u = as_map(Some(u)); Some((u.get("old_id")?.as_i64()?, u.get("id")?.as_i64()?)) }).collect();
    let name = text(&rec(&e, "pos.order", oid)?, "name").unwrap_or_default();
    for c in coupon_data {
        let c = as_map(Some(c)); let raw = c.get("card_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let card = *mapping.get(&raw).unwrap_or(&raw);
        if rec(&e, "loyalty.card", card).is_err() { continue; }
        let (issued, cost) = (num(&c, "won"), num(&c, "spent"));
        if (issued != 0.0 || cost != 0.0) && card > 0 {
            orm::create(&e, "loyalty.history", row(&[("card_id", card.into()), ("order_model", "pos.order".into()), ("order_id", oid.into()), ("description", format!("Onsite {name}").into()), ("used", cost.into()), ("issued", issued.into())]))?;
        }
    }
    Ok(())
}
/// A partner has one loyalty/eWallet card per program: reuse it instead of minting another.
fn check_existing_cards(env: &Env, data: &mut BTreeMap<i64, Row>) -> Result<()> {
    let e = env.sudo(); let mut moves = vec![];
    for (id, v) in data.iter() {
        let Some(partner) = v.get("partner_id").and_then(|p| p.as_i64()).filter(|p| *p != 0) else { continue };
        let prog = v.get("program_id").and_then(|p| p.as_i64()).unwrap_or(0);
        for c in children_by(&e, "loyalty.card", &Domain::And(vec![term("partner_id", "=", partner), term("program_id", "=", prog)]))? {
            let ptype = rec(&e, "loyalty.program", prog).ok().and_then(|p| text(&p, "program_type"));
            if matches!(ptype.as_deref(), Some("loyalty" | "ewallet")) { moves.push((*id, c["id"].as_i64().unwrap())); break; }
        }
    }
    for (old, new) in moves { if let Some(mut v) = data.remove(&old) { v.insert("coupon_id".into(), new.into()); data.insert(new, v); } }
    Ok(())
}
/// An order that already has history for a program must not be confirmed twice.
fn remove_duplicates(env: &Env, oid: i64, data: &mut BTreeMap<i64, Row>) -> Result<()> {
    let e = env.sudo(); let mut drop = vec![];
    for (id, v) in data.iter() {
        let prog = v.get("program_id").and_then(|p| p.as_i64()).unwrap_or(0);
        for h in children_by(&e, "loyalty.history", &Domain::And(vec![term("order_model", "=", "pos.order"), term("order_id", "=", oid)]))? {
            if id_of(&h, "card_id").and_then(|c| rec(&e, "loyalty.card", c).ok()).and_then(|c| id_of(&c, "program_id")) == Some(prog) { drop.push(*id); break; }
        }
    }
    for d in drop { data.remove(&d); }
    Ok(())
}
/// `confirm_coupon_programs`: create the coupons an order awarded, add points, link reward lines and write the history.
fn confirm_coupons(env: &Env, oid: i64, raw: &Row) -> Result<Value> {
    let e = env.sudo(); let order = rec(&e, "pos.order", oid)?;
    let mut data: BTreeMap<i64, Row> = raw.iter().filter_map(|(k, v)| Some((k.parse::<i64>().ok()?, as_map(Some(v))))).collect();
    check_existing_cards(&e, &mut data)?; remove_duplicates(&e, oid, &mut data)?;
    let mut id_map: BTreeMap<i64, i64> = data.keys().filter(|k| **k > 0).map(|k| (*k, *k)).collect();   // new/real id -> key in `data`
    let partner_of = |p: Option<i64>| p.filter(|p| *p != 0).filter(|p| rec(&e, "res.partner", *p).is_ok());
    let to_create: Vec<(i64, Row)> = data.iter().filter(|(k, v)| **k < 0 && !v.get("giftCardId").map_or(false, |g| g.truthy()) && (num(v, "points") != 0.0 || v.get("line_codes").map_or(false, |l| matches!(l, Value::List(x) if !x.is_empty())))).map(|(k, v)| (*k, v.clone())).collect();
    let mut new_cards: Vec<i64> = vec![];
    for (_, p) in &to_create {
        let code = text(p, "gift_code").or_else(|| text(p, "code")).or_else(|| text(p, "barcode")).filter(|c| !c.is_empty()).unwrap_or_else(generate_code);
        let mut v = row(&[("program_id", p.get("program_id").cloned().unwrap_or(Value::Null)), ("code", code.into()), ("points", 0.0.into()), ("source_pos_order_id", oid.into())]);
        if let Some(pt) = partner_of(p.get("partner_id").and_then(|x| x.as_i64()).or(id_of(&order, "partner_id"))) { v.insert("partner_id".into(), pt.into()); }
        if let Some(d) = text(p, "expiration_date").filter(|d| !d.is_empty()) { v.insert("expiration_date".into(), d.into()); }
        new_cards.push(orm::create(&e.with_ctx("action_no_send_mail", Value::Bool(true)), "loyalty.card", v)?);
    }
    // gift cards sold earlier and scanned now take their points and owner from the order
    let mut updated_gift: Vec<i64> = vec![];
    for v in data.values() {
        if let Some(g) = v.get("giftCardId").and_then(|g| g.as_i64()) {
            let mut w = row(&[("points", v.get("points").cloned().unwrap_or(0.0.into())), ("source_pos_order_id", oid.into())]);
            w.insert("partner_id".into(), partner_of(v.get("partner_id").and_then(|x| x.as_i64())).map_or(Value::Null, Value::Int));
            orm::write(&e, "loyalty.card", &[g], w)?; updated_gift.push(g);
        }
    }
    for ((old, _), nid) in to_create.iter().zip(&new_cards) { id_map.insert(*nid, *old); }
    let mut lines_by_code: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    for l in children(&e, "pos.order.line", "order_id", oid)? { if let Some(c) = text(&l, "reward_identifier_code").filter(|c| !c.is_empty()) { lines_by_code.entry(c).or_default().push(l["id"].as_i64().unwrap()); } }
    let mut all_cards: Vec<Row> = vec![];
    for (cid, key) in &id_map {
        let Ok(card) = rec(&e, "loyalty.card", *cid) else { continue };
        let vals = data.get(key).cloned().unwrap_or_default();
        orm::write(&e, "loyalty.card", &[*cid], row(&[("points", clean(num(&card, "points") + num(&vals, "points")).into())]))?;
        if let Some(Value::List(codes)) = vals.get("line_codes") { for code in codes { if let Some(ls) = code.as_str().and_then(|c| lines_by_code.get(c)) { orm::write(&e, "pos.order.line", ls, row(&[("coupon_id", (*cid).into())]))?; } } }
        all_cards.push(rec(&e, "loyalty.card", *cid)?);
    }
    // reports to print per program: the "create" plans' print report
    let mut coupon_report: Row = Row::new();
    if has_model(&e, "loyalty.mail") && e.reg.field("loyalty.mail", "pos_report_print_id").is_ok() {
        for cid in new_cards.iter().chain(updated_gift.iter()) {
            let card = rec(&e, "loyalty.card", *cid)?;
            for plan in children(&e, "loyalty.mail", "program_id", id_of(&card, "program_id").unwrap_or(0))? {
                if text(&plan, "trigger").as_deref() != Some("create") { continue; }
                if let Some(rep) = id_of(&plan, "pos_report_print_id") {
                    match coupon_report.entry(rep.to_string()).or_insert(Value::List(vec![])) { Value::List(l) => l.push(Value::Int(*cid)), _ => {} }
                }
            }
        }
    }
    // history
    let mut points = vec![];
    for (cid, v) in &data {
        let (won, spent) = if v.contains_key("points_earned") && v.contains_key("points_spent") { (num(v, "points_earned"), num(v, "points_spent")) } else { (num(v, "points").max(0.0), (-num(v, "points")).max(0.0)) };
        points.push(Value::Map(row(&[("order_id", oid.into()), ("card_id", (*cid).into()), ("spent", spent.into()), ("won", won.into())])));
    }
    let updates: Vec<Value> = all_cards.iter().map(|c| Value::Map(row(&[("id", c["id"].clone()), ("old_id", (*id_map.get(&c["id"].as_i64().unwrap()).unwrap_or(&0)).into())]))).collect();
    add_history_lines(&e, oid, &points, &updates)?;
    let prog_of = |c: &Row| id_of(c, "program_id").and_then(|p| rec(&e, "loyalty.program", p).ok());
    let mut program_updates: Vec<Value> = vec![]; let mut seen: BTreeSet<i64> = BTreeSet::new();
    for c in &all_cards { if let Some(p) = prog_of(c) { let pid = p["id"].as_i64().unwrap(); if seen.insert(pid) { program_updates.push(Value::Map(row(&[("program_id", pid.into()), ("usages", program_pos_order_count(&e, pid)?.into())]))); } } }
    let new_info: Vec<Value> = new_cards.iter().filter_map(|c| rec(&e, "loyalty.card", *c).ok()).filter_map(|c| {
        let p = prog_of(&c)?;
        (text(&p, "applies_on").as_deref() == Some("future") && !matches!(text(&p, "program_type").as_deref(), Some("gift_card" | "ewallet"))).then(|| Value::Map(row(&[("program_name", p.get("name").cloned().unwrap_or(Value::Null)), ("expiration_date", c.get("expiration_date").cloned().unwrap_or(Value::Bool(false))), ("code", c.get("code").cloned().unwrap_or(Value::Null))])))
    }).collect();
    Ok(Value::Map(row(&[
        ("coupon_updates", Value::List(all_cards.iter().filter(|c| prog_of(c).map_or(false, |p| text(&p, "applies_on").as_deref() == Some("both"))).map(|c| Value::Map(row(&[("old_id", (*id_map.get(&c["id"].as_i64().unwrap()).unwrap_or(&0)).into()), ("id", c["id"].clone()), ("points", c.get("points").cloned().unwrap_or(0.0.into())), ("code", c.get("code").cloned().unwrap_or(Value::Null)), ("program_id", c.get("program_id").cloned().unwrap_or(Value::Null)), ("partner_id", id_of(c, "partner_id").map_or(Value::Bool(false), Value::Int))]))).collect())),
        ("program_updates", Value::List(program_updates)), ("new_coupon_info", Value::List(new_info)), ("coupon_report", Value::Map(coupon_report)),
    ])))
}

pub fn rules() -> Rules {
    Rules::default()
        .compute("loyalty.program", "pos_order_count", |env, r| Ok(program_pos_order_count(env, r["id"].as_i64().unwrap())?.into()))
        // the generic count is the POS one here (sales add theirs in sale_loyalty)
        .compute("loyalty.program", "total_order_count", |env, r| Ok(program_pos_order_count(env, r["id"].as_i64().unwrap())?.into()))
        // nominative programs (one card per customer) are those that apply on both current and future orders
        .compute("loyalty.program", "is_nominative", |_, r| Ok((text(r, "applies_on").as_deref() == Some("both")).into()))
        .compute("loyalty.rule", "any_product", |_, r| Ok(rule_any_product(r).into()))
        // stored: a barcode is generated once, then kept (it is editable)
        .compute("loyalty.rule", "promo_barcode", |_, r| Ok(text(r, "promo_barcode").filter(|b| !b.is_empty()).unwrap_or_else(generate_code).into()))
        .action("loyalty.rule", "_get_valid_product_ids", |env, ids_, _| Ok(id_list(&rule_valid_products(env, &rec(env, "loyalty.rule", first(ids_)?)?)?)))
        .action("pos.config", "_get_program_ids", |env, ids_, _| Ok(id_list(&config_programs(env, first(ids_)?)?.iter().filter_map(|p| p["id"].as_i64()).collect::<Vec<_>>())))
        .action("pos.config", "_check_loyalty_programs", |env, ids_, _| { check_programs(env, first(ids_)?)?; Ok(Value::Bool(true)) })
        .action("pos.config", "use_coupon_code", |env, ids_, kw| use_coupon_code(env, first(ids_)?, &text(kw, "code").unwrap_or_default(), &text(kw, "creation_date").unwrap_or_else(orm::now), kw.get("partner_id").and_then(|v| v.as_i64()), kw.get("pricelist_id").and_then(|v| v.as_i64())))
        .action("pos.order", "validate_coupon_programs", |env, _, kw| {
            let codes: Vec<String> = match kw.get("new_codes") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_str().map(String::from)).collect(), _ => vec![] };
            validate_coupons(env, &as_map(kw.get("point_changes")), &codes)
        })
        .action("pos.order", "add_loyalty_history_lines", |env, ids_, kw| {
            let list = |k: &str| match kw.get(k) { Some(Value::List(l)) => l.clone(), _ => vec![] };
            add_history_lines(env, first(ids_)?, &list("coupon_data"), &list("coupon_updates"))?; Ok(Value::Null)
        })
        .action("pos.order", "confirm_coupon_programs", |env, ids_, kw| confirm_coupons(env, first(ids_)?, &as_map(kw.get("coupon_data"))))
        .action("pos.order", "_check_existing_loyalty_cards", |env, _, kw| {
            let mut d: BTreeMap<i64, Row> = as_map(kw.get("coupon_data")).iter().filter_map(|(k, v)| Some((k.parse().ok()?, as_map(Some(v))))).collect();
            check_existing_cards(env, &mut d)?; Ok(Value::Map(d.into_iter().map(|(k, v)| (k.to_string(), Value::Map(v))).collect()))
        })
        .action("pos.order", "_remove_duplicate_coupon_data", |env, ids_, kw| {
            let mut d: BTreeMap<i64, Row> = as_map(kw.get("coupon_data")).iter().filter_map(|(k, v)| Some((k.parse().ok()?, as_map(Some(v))))).collect();
            remove_duplicates(env, first(ids_)?, &mut d)?; Ok(Value::Map(d.into_iter().map(|(k, v)| (k.to_string(), Value::Map(v))).collect()))
        })
}
