use odoo_core::{ddl, orm::{self, AllowAll}, store::{self, Store}, Domain, Registry, Row, Value};
use odoo_modules::{rules_for, util::*};
use odoo_duckdb::DuckStore;
use std::path::Path;

fn setup() -> (Registry, DuckStore) {
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["sale_stock", "purchase_stock", "crm", "account"]).unwrap();
    let st = DuckStore::open(":memory:").unwrap();
    let plan = ddl::install_plan(&reg, st.dialect());
    store::run(&st, |c| { for s in &plan { c.execute(s, &[])?; } Ok(()) }).unwrap();
    (reg, st)
}
fn lines(v: Vec<Row>) -> Value { Value::List(v.into_iter().map(|m| Value::List(vec![0.into(), 0.into(), Value::Map(m)])).collect()) }

#[test]
fn order_to_cash() {
    let (reg, st) = setup();
    let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env)?;
        let partner = orm::create(&env, "res.partner", row(&[("name", "Azure".into())]))?;
        let tax = orm::create(&env, "account.tax", row(&[("name", "VAT 15%".into()), ("amount", 15.0.into()), ("amount_type", "percent".into()), ("type_tax_use", "sale".into())]))?;
        // delegation: product.product create writes the template
        let prod = orm::create(&env, "product.product", row(&[("name", "Desk".into()), ("list_price", 100.0.into()), ("type", "consu".into())]))?;
        let tmpl = orm::read(&env, "product.product", &[prod], &["product_tmpl_id".into(), "name".into(), "list_price".into()])?;
        assert_eq!(tmpl[0]["name"], Value::Text("Desk".into()));   // related read through template
        assert_eq!(tmpl[0]["list_price"].as_f64(), Some(100.0));

        let so = orm::create(&env, "sale.order", row(&[("partner_id", partner.into()), ("order_line", lines(vec![
            row(&[("product_id", prod.into()), ("product_uom_qty", 2.0.into()), ("discount", 10.0.into()), ("tax_id", Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(vec![tax.into()])])]))]),
            row(&[("name", "Extra".into()), ("product_uom_qty", 1.0.into()), ("price_unit", 50.0.into())]),
        ]))]))?;
        let o = rec(&env, "sale.order", so)?;
        assert_eq!(text(&o, "name").unwrap(), "S00001");
        assert_eq!(num(&o, "amount_untaxed"), 230.0);          // 2*100*0.9 + 50
        assert_eq!(num(&o, "amount_tax"), 27.0);               // 15% of 180
        assert_eq!(num(&o, "amount_total"), 257.0);

        orm::call(&env, "sale.order", "action_confirm", &[so], &Row::new())?;
        assert_eq!(text(&rec(&env, "sale.order", so)?, "state").unwrap(), "sale");
        assert_eq!(text(&rec(&env, "sale.order", so)?, "invoice_status").unwrap(), "to invoice");
        assert!(orm::call(&env, "sale.order", "action_confirm", &[so], &Row::new()).is_err()); // not re-confirmable

        // delivery
        let picks = orm::search(&env, "stock.picking", &Domain::Term("origin".into(), "=".into(), "S00001".into()), None, None, 0)?;
        assert_eq!(picks.len(), 1);
        orm::call(&env, "stock.picking", "button_validate", &picks, &Row::new())?;
        assert_eq!(text(&rec(&env, "stock.picking", picks[0])?, "state").unwrap(), "done");

        // invoice → post → balanced entry → pay
        let inv = orm::call(&env, "sale.order", "_create_invoices", &[so], &Row::new())?;
        let Value::List(l) = inv else { panic!() }; let mid = l[0].as_i64().unwrap();
        orm::call(&env, "account.move", "action_post", &[mid], &Row::new())?;
        let m = rec(&env, "account.move", mid)?;
        assert_eq!(text(&m, "state").unwrap(), "posted");
        assert!(text(&m, "name").unwrap().starts_with("INV/"));
        assert_eq!(num(&m, "amount_total"), 257.0);
        assert_eq!(num(&m, "amount_residual"), 257.0);
        let jl = children(&env, "account.move.line", "move_id", mid)?;
        let (d, cr): (f64, f64) = jl.iter().fold((0.0, 0.0), |a, l| (a.0 + num(l, "debit"), a.1 + num(l, "credit")));
        assert_eq!((d, cr), (257.0, 257.0));
        assert_eq!(text(&rec(&env, "sale.order", so)?, "invoice_status").unwrap(), "invoiced");
        orm::call(&env, "account.move", "action_register_payment", &[mid], &row(&[("amount", 100.0.into())]))?;
        assert_eq!(text(&rec(&env, "account.move", mid)?, "payment_state").unwrap(), "partial");
        orm::call(&env, "account.move", "action_register_payment", &[mid], &Row::new())?;
        let m = rec(&env, "account.move", mid)?;
        assert_eq!((num(&m, "amount_residual"), text(&m, "payment_state").unwrap()), (0.0, "paid".into()));
        Ok(())
    }).unwrap();
}

#[test]
fn procure_to_pay_and_crm() {
    let (reg, st) = setup();
    let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env)?;
        let vendor = orm::create(&env, "res.partner", row(&[("name", "Vendor".into())]))?;
        let prod = orm::create(&env, "product.product", row(&[("name", "Chair".into()), ("standard_price", 40.0.into()), ("type", "consu".into())]))?;
        let po = orm::create(&env, "purchase.order", row(&[("partner_id", vendor.into()), ("order_line", lines(vec![row(&[("product_id", prod.into()), ("product_qty", 5.0.into()), ("price_unit", 40.0.into())])]))]))?;
        assert_eq!(num(&rec(&env, "purchase.order", po)?, "amount_total"), 200.0);
        orm::call(&env, "purchase.order", "button_confirm", &[po], &Row::new())?;
        let picks = orm::search(&env, "stock.picking", &Domain::Term("origin".into(), "=".into(), "P00001".into()), None, None, 0)?;
        assert_eq!(picks.len(), 1);
        orm::call(&env, "stock.picking", "button_validate", &picks, &Row::new())?;
        let internal = odoo_modules::stock::ensure_location(&env, "internal", "x")?;
        assert_eq!(odoo_modules::stock::on_hand(&env, prod, internal)?, 5.0);
        let Value::List(b) = orm::call(&env, "purchase.order", "action_create_invoice", &[po], &Row::new())? else { panic!() };
        orm::call(&env, "account.move", "action_post", &[b[0].as_i64().unwrap()], &Row::new())?;
        assert_eq!(num(&rec(&env, "account.move", b[0].as_i64().unwrap())?, "amount_total"), 200.0);

        // CRM
        orm::create(&env, "crm.stage", row(&[("name", "New".into()), ("sequence", 1.into())]))?;
        orm::create(&env, "crm.stage", row(&[("name", "Won".into()), ("sequence", 9.into()), ("is_won", true.into())]))?;
        let lead = orm::create(&env, "crm.lead", row(&[("name", "Big deal".into()), ("expected_revenue", 5000.0.into())]))?;
        orm::call(&env, "crm.lead", "action_set_won", &[lead], &Row::new())?;
        assert_eq!(num(&rec(&env, "crm.lead", lead)?, "probability"), 100.0);
        Ok(())
    }).unwrap();
}


#[test]
fn seed_and_catalog() {
    let (reg, st) = setup();
    let rules = rules_for(&reg);
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"));
    let mods = reg.modules.clone();
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env)?;
        let rep = odoo_modules::data::seed(&env, dir, &mods)?;
        eprintln!("SEED loaded={} skipped={} failed={:?}\n{}", rep.loaded, rep.skipped, rep.failed, rep.first_errors.join("\n"));
        assert!(rep.loaded > 100);
        let countries = orm::search_count(&env, "res.country", &Domain::True)?;
        assert!(countries > 200, "countries={countries}");
        Ok(())
    }).unwrap();
    let cat = odoo_modules::data::Catalog::load(dir, &mods);
    let tree = cat.menu_tree(&reg);
    let n = tree.as_array().unwrap().len();
    eprintln!("MENU roots={n} actions={}", cat.actions.len());
    assert!(n >= 3);
    let acl = odoo_modules::data::access_table(&reg, dir, &mods);
    eprintln!("ACL entries={}", acl.entries.len());
    assert!(acl.entries.len() > 100);
}
