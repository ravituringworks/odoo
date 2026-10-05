//! delivery (carriers, rating, shipping lines) and sale_management (templates, optional products).
use odoo_core::{ddl, orm::{self, AllowAll, Env}, store::{self, Store}, Registry, Row, Value};
use odoo_modules::{rules_for, sale_util::flag, util::*};
use odoo_sqlite::SqliteStore;
use std::path::Path;

fn setup() -> (Registry, SqliteStore) {
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["sale_stock", "sale_management", "delivery", "account"]).unwrap();
    let st = SqliteStore::open(":memory:").unwrap();
    let plan = ddl::install_plan(&reg, st.dialect());
    store::run(&st, |c| { for s in &plan { c.execute(s, &[])?; } Ok(()) }).unwrap();
    (reg, st)
}
fn lines(v: Vec<Row>) -> Value { Value::List(v.into_iter().map(|m| Value::List(vec![0.into(), 0.into(), Value::Map(m)])).collect()) }
fn m2m(ids: &[i64]) -> Value { Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(ids.iter().map(|i| Value::Int(*i)).collect())])]) }
fn call(env: &Env, model: &str, method: &str, ids: &[i64], args: &[(&str, Value)]) -> odoo_core::Result<Value> { orm::call(env, model, method, ids, &row(args)) }
fn rd(env: &Env, model: &str, id: i64) -> odoo_core::Result<Row> { Ok(orm::read(env, model, &[id], &[])?.remove(0)) }
fn rid(r: &Row) -> i64 { r["id"].as_i64().unwrap() }
fn env_of<'a>(reg: &'a Registry, c: &'a dyn odoo_core::store::Conn, rules: &'a odoo_core::Rules) -> Env<'a> { let env = Env::new(reg, c, rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env).unwrap(); env }
fn product(env: &Env, name: &str, price: f64, ty: &str, weight: f64) -> i64 {
    orm::create(env, "product.product", row(&[("name", name.into()), ("list_price", price.into()), ("type", ty.into()), ("weight", weight.into())])).unwrap()
}
fn order(env: &Env, partner: i64, ls: Vec<Row>) -> i64 { orm::create(env, "sale.order", row(&[("partner_id", partner.into()), ("order_line", lines(ls))])).unwrap() }
fn sol(env: &Env, order: i64) -> Vec<Row> { let mut v = children(env, "sale.order.line", "order_id", order).unwrap(); v.sort_by_key(|l| (l["sequence"].as_i64().unwrap_or(0), rid(l))); v }
fn line(p: i64, q: f64) -> Row { row(&[("product_id", p.into()), ("product_uom_qty", q.into())]) }

#[test]
fn carrier_rating_matching_and_shipping_lines() {
    let (reg, st) = setup(); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = env_of(&reg, c, &rules);
        let be = orm::create(&env, "res.country", row(&[("name", "Belgium".into()), ("code", "be".into())]))?;
        let fr = orm::create(&env, "res.country", row(&[("name", "France".into()), ("code", "FR".into())]))?;
        let cust = orm::create(&env, "res.partner", row(&[("name", "Azure".into()), ("country_id", be.into()), ("zip", "1000".into())]))?;
        let goods = product(&env, "Anvil", 100.0, "consu", 2.0);
        let svc = product(&env, "Setup", 50.0, "service", 0.0);
        let ship = product(&env, "Shipping", 10.0, "service", 0.0);

        // fixed carrier: fixed_price follows the delivery product, and writing it writes the product back
        let fixed = orm::create(&env, "delivery.carrier", row(&[("name", "Standard".into()), ("product_id", ship.into()), ("delivery_type", "fixed".into())]))?;
        assert_eq!(num(&rd(&env, "delivery.carrier", fixed)?, "fixed_price"), 10.0);
        orm::write(&env, "delivery.carrier", &[fixed], row(&[("fixed_price", 12.0.into())]))?;
        assert_eq!(num(&orm::read(&env, "product.product", &[ship], &["list_price".to_string()])?[0], "list_price"), 12.0);
        assert_eq!(num(&rd(&env, "delivery.carrier", fixed)?, "fixed_price"), 12.0);

        let so = order(&env, cust, vec![line(goods, 3.0), line(svc, 1.0)]);
        // shipping weight: consumables only (3 x 2kg)
        assert_eq!(num(&rec(&env, "sale.order", so)?, "shipping_weight"), 6.0);
        assert_eq!(call(&env, "sale.order", "_get_estimated_weight", &[so], &[])?.as_f64(), Some(6.0));
        let Value::Map(r) = call(&env, "delivery.carrier", "rate_shipment", &[fixed], &[("order_id", so.into())])? else { panic!() };
        assert_eq!((r["success"].clone(), r["price"].as_f64().unwrap(), r["carrier_price"].as_f64().unwrap()), (Value::Bool(true), 12.0, 12.0));
        assert_eq!(r["error_message"], Value::Bool(false));

        // free_over: order amount (without shipping) >= amount makes the shipping free, with a warning
        orm::write(&env, "delivery.carrier", &[fixed], row(&[("free_over", true.into()), ("amount", 100.0.into())]))?;
        let Value::Map(r) = call(&env, "delivery.carrier", "rate_shipment", &[fixed], &[("order_id", so.into())])? else { panic!() };
        assert_eq!((r["price"].as_f64().unwrap(), r["carrier_price"].as_f64().unwrap()), (0.0, 12.0));
        assert_eq!(r["warning_message"], Value::Text("The shipping is free since the order amount exceeds 100.00.".into()));

        // rule based carrier: weight <= 5 -> 5.00 ; weight > 5 -> 5.00 + 1.5 * weight ; margins apply
        let rule = orm::create(&env, "delivery.carrier", row(&[("name", "By weight".into()), ("product_id", ship.into()), ("delivery_type", "base_on_rule".into()), ("margin", 0.1.into()), ("fixed_margin", 1.0.into()),
            ("price_rule_ids", lines(vec![
                row(&[("sequence", 1.into()), ("variable", "weight".into()), ("operator", "<=".into()), ("max_value", 5.0.into()), ("list_base_price", 5.0.into()), ("list_price", 0.0.into()), ("variable_factor", "weight".into())]),
                row(&[("sequence", 2.into()), ("variable", "weight".into()), ("operator", ">".into()), ("max_value", 5.0.into()), ("list_base_price", 5.0.into()), ("list_price", 1.5.into()), ("variable_factor", "weight".into())]),
            ]))]))?;
        let pr = children(&env, "delivery.price.rule", "carrier_id", rule)?;
        assert_eq!(text(&rd(&env, "delivery.price.rule", rid(&pr[0]))?, "name").unwrap(), "if weight <= 5.00 then fixed price 5.00");
        assert_eq!(text(&rd(&env, "delivery.price.rule", rid(&pr[1]))?, "name").unwrap(), "if weight > 5.00 then fixed price 5.00 plus 1.50 times weight");
        let rate = |car: i64, o: i64, w: Option<f64>| -> Row { let mut a = vec![("order_id", Value::Int(o))]; if let Some(w) = w { a.push(("order_weight", w.into())); } let Value::Map(m) = call(&env, "delivery.carrier", "rate_shipment", &[car], &a).unwrap() else { panic!() }; m.into_iter().collect() };
        let r = rate(rule, so, None);       // 6kg -> 5 + 9 = 14, +10% + 1 = 16.4
        assert!((r["price"].as_f64().unwrap() - 16.4).abs() < 1e-9, "{r:?}");
        let r = rate(rule, so, Some(4.0));   // weight chosen in the wizard overrides: 5 *1.1 + 1
        assert!((r["price"].as_f64().unwrap() - 6.5).abs() < 1e-9, "{r:?}");
        // no matching rule => failure with the Odoo message
        let (r1, r2) = (pr[0].clone(), pr[1].clone());
        orm::unlink(&env, "delivery.price.rule", &[rid(&r1), rid(&r2)])?;
        let r = rate(rule, so, None);
        assert_eq!((r["success"].clone(), text(&r, "error_message").unwrap()), (Value::Bool(false), "Not available for current order".into()));
        assert_eq!(call(&env, "delivery.carrier", "_is_available_for_order", &[rule], &[("order_id", so.into())])?, Value::Bool(false));
        assert_eq!(call(&env, "delivery.carrier", "_is_available_for_order", &[fixed], &[("order_id", so.into())])?, Value::Bool(true));

        // address matching: countries, zip prefixes (uppercased; regex-style anchors)
        let be_only = orm::create(&env, "delivery.carrier", row(&[("name", "BE only".into()), ("product_id", ship.into()), ("country_ids", m2m(&[be]))]))?;
        let fr_only = orm::create(&env, "delivery.carrier", row(&[("name", "FR only".into()), ("product_id", ship.into()), ("country_ids", m2m(&[fr]))]))?;
        assert_eq!(call(&env, "delivery.carrier", "_match_address", &[be_only], &[("partner_id", cust.into())])?, Value::Bool(true));
        assert_eq!(call(&env, "delivery.carrier", "_match_address", &[fr_only], &[("partner_id", cust.into())])?, Value::Bool(false));
        let Value::Map(r) = call(&env, "delivery.carrier", "rate_shipment", &[fr_only], &[("order_id", so.into())])? else { panic!() };
        assert_eq!(r["error_message"], Value::Text("Error: this delivery method is not available for this address.".into()));
        let z = orm::create(&env, "delivery.zip.prefix", row(&[("name", "b10".into())]))?;
        assert_eq!(text(&rec(&env, "delivery.zip.prefix", z)?, "name").unwrap(), "B10");
        let z2 = orm::create(&env, "delivery.zip.prefix", row(&[("name", "100$".into())]))?;
        let zip_car = orm::create(&env, "delivery.carrier", row(&[("name", "Zip".into()), ("product_id", ship.into()), ("zip_prefix_ids", m2m(&[z2]))]))?;
        assert_eq!(call(&env, "delivery.carrier", "_match_address", &[zip_car], &[("partner_id", cust.into())])?, Value::Bool(false));   // "1000" is not "100$"
        orm::write(&env, "res.partner", &[cust], row(&[("zip", "100".into())]))?;
        assert_eq!(call(&env, "delivery.carrier", "_match_address", &[zip_car], &[("partner_id", cust.into())])?, Value::Bool(true));
        let Value::List(av) = call(&env, "delivery.carrier", "available_carriers", &[fixed, be_only, fr_only, zip_car], &[("partner_id", cust.into()), ("order_id", so.into())])? else { panic!() };
        assert_eq!(av.iter().filter_map(|v| v.as_i64()).collect::<Vec<_>>(), vec![fixed, be_only, zip_car]);
        // weight limit
        orm::write(&env, "delivery.carrier", &[fixed], row(&[("max_weight", 5.0.into())]))?;
        assert_eq!(call(&env, "delivery.carrier", "_match", &[fixed], &[("partner_id", cust.into()), ("order_id", so.into())])?, Value::Bool(false));

        // shipping line: set_delivery_line adds it (and the carrier), setting again replaces it
        orm::write(&env, "delivery.carrier", &[fixed], row(&[("max_weight", 0.0.into()), ("free_over", false.into())]))?;
        call(&env, "sale.order", "set_delivery_line", &[so], &[("carrier_id", fixed.into()), ("amount", 12.0.into())])?;
        let d: Vec<Row> = sol(&env, so).into_iter().filter(|l| flag(l, "is_delivery")).collect();
        assert_eq!(d.len(), 1);
        assert_eq!((num(&d[0], "price_unit"), num(&d[0], "product_uom_qty"), id_of(&d[0], "product_id")), (12.0, 1.0, Some(ship)));
        assert_eq!(text(&d[0], "name").unwrap(), "Standard");
        assert_eq!(id_of(&rec(&env, "sale.order", so)?, "carrier_id"), Some(fixed));
        assert_eq!(rd(&env, "sale.order", so)?["delivery_set"], Value::Bool(true));
        assert_eq!(num(&rec(&env, "sale.order", so)?, "amount_untaxed"), 300.0 + 50.0 + 12.0);
        call(&env, "sale.order", "set_delivery_line", &[so], &[("carrier_id", fixed.into()), ("amount", 7.0.into())])?;
        let d: Vec<Row> = sol(&env, so).into_iter().filter(|l| flag(l, "is_delivery")).collect();
        assert_eq!((d.len(), num(&d[0], "price_unit")), (1, 7.0));
        // free shipping wording
        orm::write(&env, "delivery.carrier", &[fixed], row(&[("free_over", true.into())]))?;
        call(&env, "sale.order", "set_delivery_line", &[so], &[("carrier_id", fixed.into()), ("amount", 0.0.into())])?;
        let d: Vec<Row> = sol(&env, so).into_iter().filter(|l| flag(l, "is_delivery")).collect();
        assert_eq!(text(&d[0], "name").unwrap(), "Standard\nFree Shipping");
        // the delivery line is excluded from the 'without delivery' total and from the shipping weight
        assert_eq!(call(&env, "sale.order", "_compute_amount_total_without_delivery", &[so], &[])?.as_f64(), Some(350.0));

        // wizard: pick the carrier, price it, confirm
        orm::write(&env, "delivery.carrier", &[fixed], row(&[("free_over", false.into())]))?;
        let w = orm::create(&env, "choose.delivery.carrier", row(&[("order_id", so.into()), ("carrier_id", fixed.into())]))?;
        call(&env, "choose.delivery.carrier", "update_price", &[w], &[])?;
        let wr = rec(&env, "choose.delivery.carrier", w)?;
        assert_eq!((num(&wr, "delivery_price"), num(&wr, "display_price")), (12.0, 12.0));
        call(&env, "choose.delivery.carrier", "button_confirm", &[w], &[])?;
        let d: Vec<Row> = sol(&env, so).into_iter().filter(|l| flag(l, "is_delivery")).collect();
        assert_eq!((d.len(), num(&d[0], "price_unit")), (1, 12.0));
        assert!(!flag(&rec(&env, "sale.order", so)?, "recompute_delivery_price"));

        // confirmed + invoiced shipping can no longer be replaced; an uninvoiced one can be deleted (resetting the carrier)
        call(&env, "sale.order", "action_confirm", &[so], &[])?;
        orm::unlink(&env, "sale.order.line", &[rid(&d[0])])?;
        assert_eq!(id_of(&rec(&env, "sale.order", so)?, "carrier_id"), None);
        call(&env, "sale.order", "set_delivery_line", &[so], &[("carrier_id", fixed.into()), ("amount", 12.0.into())])?;
        let inv = call(&env, "sale.order", "_create_invoices", &[so], &[])?;
        assert_eq!(match inv { Value::List(l) => l.len(), _ => 0 }, 1);
        let e = call(&env, "sale.order", "set_delivery_line", &[so], &[("carrier_id", fixed.into()), ("amount", 5.0.into())]).unwrap_err().to_string();
        assert!(e.starts_with("You can not update the shipping costs on an order where it was already invoiced!"), "{e}");
        // quantities/prices are printed like Python's str(float)
        assert!(e.contains(": 1.0 x 12.0"), "{e}");
        assert!(e.contains("- Shipping: 1.0 x 12.0"), "{e}");
        // a lone delivery line left to invoice does not make the order invoiceable (can't be invoiced alone)
        assert_eq!(text(&rd(&env, "sale.order", so)?, "invoice_status").unwrap(), "invoiced");

        // tag constraint
        let tag = orm::create(&env, "product.tag", row(&[("name", "Fragile".into())]))?;
        let e = orm::create(&env, "delivery.carrier", row(&[("name", "Bad".into()), ("product_id", ship.into()), ("must_have_tag_ids", m2m(&[tag])), ("excluded_tag_ids", m2m(&[tag]))])).unwrap_err().to_string();
        assert_eq!(e, "Carrier Bad cannot have the same tag in both Must Have Tags and Excluded Tags.");
        Ok(())
    }).unwrap();
}

#[test]
fn quotation_templates_and_optional_products() {
    let (reg, st) = setup(); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = env_of(&reg, c, &rules);
        let cust = orm::create(&env, "res.partner", row(&[("name", "Azure".into())]))?;
        let a = product(&env, "Desk", 100.0, "consu", 0.0);
        let b = product(&env, "Chair", 40.0, "consu", 0.0);
        let tpl = orm::create(&env, "sale.order.template", row(&[("name", "Office".into()), ("number_of_days", 15.into()),
            ("sale_order_template_line_ids", lines(vec![
                row(&[("display_type", "line_section".into()), ("name", "Furniture".into()), ("sequence", 1.into())]),
                row(&[("product_id", a.into()), ("product_uom_qty", 2.0.into()), ("sequence", 2.into())]),
                row(&[("display_type", "line_note".into()), ("name", "Assembly included".into()), ("sequence", 3.into())]),
            ])),
            ("sale_order_template_option_ids", lines(vec![row(&[("product_id", b.into()), ("quantity", 4.0.into())])]))]))?;
        // template lines normalise: sections drop product/qty, product lines take the product uom
        let tl = children(&env, "sale.order.template.line", "sale_order_template_id", tpl)?;
        assert_eq!((id_of(&tl[0], "product_id"), num(&tl[0], "product_uom_qty"), id_of(&tl[0], "product_uom_id")), (None, 0.0, None));
        assert!(id_of(&tl[1], "product_uom_id").is_some());
        // template inherits the company's portal settings
        assert!(flag(&rec(&env, "sale.order.template", tpl)?, "require_signature"));
        let opt_t = children(&env, "sale.order.template.option", "sale_order_template_id", tpl)?;
        assert_eq!(text(&opt_t[0], "name").unwrap(), "Chair");
        assert!(orm::write(&env, "sale.order.template", &[tpl], row(&[("require_payment", true.into()), ("prepayment_percent", 2.0.into())])).unwrap_err().to_string().contains("Prepayment percentage must be a valid percentage."));

        // order with the template: validity from the template, lines/options loaded by the onchange
        let so = orm::create(&env, "sale.order", row(&[("partner_id", cust.into()), ("sale_order_template_id", tpl.into())]))?;
        assert_eq!(text(&rec(&env, "sale.order", so)?, "validity_date").unwrap(), odoo_core::orm::shift_date(&orm::today(), 15));
        // values passed explicitly to create() win over the template-driven computes
        let so2 = orm::create(&env, "sale.order", row(&[("partner_id", cust.into()), ("sale_order_template_id", tpl.into()), ("require_signature", false.into())]))?;
        assert!(!flag(&rec(&env, "sale.order", so2)?, "require_signature"));
        assert!(flag(&rec(&env, "sale.order", so)?, "require_signature"));
        call(&env, "sale.order", "_onchange_sale_order_template_id", &[so], &[])?;
        let ls = sol(&env, so);
        assert_eq!(ls.len(), 3);
        assert_eq!(ls[0]["sequence"].as_i64(), Some(-99));
        assert_eq!((text(&ls[0], "display_type").unwrap(), text(&ls[0], "name").unwrap()), ("line_section".into(), "Furniture".into()));
        assert_eq!((id_of(&ls[1], "product_id"), num(&ls[1], "product_uom_qty"), num(&ls[1], "price_unit")), (Some(a), 2.0, 100.0));
        assert_eq!(num(&rec(&env, "sale.order", so)?, "amount_untaxed"), 200.0);
        let opts = children(&env, "sale.order.option", "order_id", so)?;
        assert_eq!(opts.len(), 1);
        assert_eq!((id_of(&opts[0], "product_id"), num(&opts[0], "quantity"), num(&opts[0], "price_unit")), (Some(b), 4.0, 40.0));
        assert_eq!(rd(&env, "sale.order.option", rid(&opts[0]))?["is_present"], Value::Bool(false));
        // applying again replaces instead of duplicating
        call(&env, "sale.order", "_onchange_sale_order_template_id", &[so], &[])?;
        assert_eq!((sol(&env, so).len(), children(&env, "sale.order.option", "order_id", so)?.len()), (3, 1));

        // add the option to the quotation
        let opt = rid(&children(&env, "sale.order.option", "order_id", so)?[0]);
        assert_eq!(call(&env, "sale.order", "_can_be_edited_on_portal", &[so], &[])?, Value::Bool(true));
        call(&env, "sale.order.option", "button_add_to_order", &[opt], &[])?;
        let o = rec(&env, "sale.order.option", opt)?;
        let new_line = id_of(&o, "line_id").unwrap();
        let nl = rec(&env, "sale.order.line", new_line)?;
        assert_eq!((id_of(&nl, "product_id"), num(&nl, "product_uom_qty"), num(&nl, "price_unit")), (Some(b), 4.0, 40.0));
        assert_eq!(num(&rec(&env, "sale.order", so)?, "amount_untaxed"), 360.0);
        assert_eq!(rd(&env, "sale.order.option", opt)?["is_present"], Value::Bool(true));
        // not on a confirmed order
        call(&env, "sale.order", "action_confirm", &[so], &[])?;
        assert_eq!(call(&env, "sale.order", "_can_be_edited_on_portal", &[so], &[])?, Value::Bool(false));
        let e = call(&env, "sale.order.option", "add_option_to_order", &[opt], &[]).unwrap_err().to_string();
        assert_eq!(e, "You cannot add options to a confirmed order.");
        Ok(())
    }).unwrap();
}
