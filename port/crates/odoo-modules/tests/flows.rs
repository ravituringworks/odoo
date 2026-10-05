use odoo_core::{ddl, orm::{self, AllowAll}, store::{self, Store}, Domain, Registry, Row, Value};
use odoo_modules::{rules_for, util::*};
use odoo_sqlite::SqliteStore;
use std::path::Path;

fn setup() -> (Registry, SqliteStore) {
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["sale_stock", "purchase_stock", "crm", "account", "hr_holidays", "project"]).unwrap();
    let st = SqliteStore::open(":memory:").unwrap();
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

#[test]
fn hr_project_onchange_defaults() {
    let (reg, st) = setup();
    let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env)?;
        // leave workflow
        let emp = orm::create(&env, "hr.employee", row(&[("name", "Ann".into())]))?;
        let lt = orm::create(&env, "hr.leave.type", row(&[("name", "Paid".into())]))?;
        let lv = orm::create(&env, "hr.leave", row(&[("employee_id", emp.into()), ("holiday_status_id", lt.into()), ("request_date_from", "2026-10-05".into()), ("request_date_to", "2026-10-11".into())]))?;
        let r = rec(&env, "hr.leave", lv)?;
        assert_eq!((text(&r, "state").unwrap(), num(&r, "number_of_days")), ("confirm".into(), 5.0));
        orm::call(&env, "hr.leave", "action_approve", &[lv], &Row::new())?;
        assert_eq!(text(&rec(&env, "hr.leave", lv)?, "state").unwrap(), "validate");
        assert!(orm::call(&env, "hr.leave", "action_approve", &[lv], &Row::new()).is_err()); // already approved
        // project task defaults + close
        let stg = orm::create(&env, "project.task.type", row(&[("name", "To Do".into()), ("sequence", 1.into())]))?;
        let prj = orm::create(&env, "project.project", row(&[("name", "Website".into())]))?;
        let tk = orm::create(&env, "project.task", row(&[("name", "Design".into()), ("project_id", prj.into())]))?;
        let r = rec(&env, "project.task", tk)?;
        assert_eq!((id_of(&r, "stage_id"), text(&r, "state").unwrap()), (Some(stg), "01_in_progress".into()));
        orm::call(&env, "project.task", "action_done", &[tk], &Row::new())?;
        assert_eq!(text(&rec(&env, "project.task", tk)?, "state").unwrap(), "1_done");
        // onchange: product fills price/name/subtotal on an order line
        let prod = orm::create(&env, "product.product", row(&[("name", "Lamp".into()), ("list_price", 25.0.into())]))?;
        let ch = orm::onchange(&env, "sale.order.line", row(&[("product_id", Value::List(vec![prod.into(), "Lamp".into()])), ("product_uom_qty", 3.0.into())]))?;
        assert_eq!(ch["price_unit"].as_f64(), Some(25.0));
        assert_eq!(ch["price_subtotal"].as_f64(), Some(75.0));
        // default_get: ambient many2one + static
        let d = orm::default_get(&env, "sale.order", &["company_id".into(), "state".into()])?;
        assert!(d.contains_key("company_id"));
        Ok(())
    }).unwrap();
}

#[test]
fn related_fields_are_searchable() {
    let (reg, st) = setup();
    let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env)?;
        let p = orm::create(&env, "product.product", row(&[("name", "Standing Desk".into()), ("list_price", 300.0.into())]))?;
        let d = Domain::Term("name".into(), "ilike".into(), "standing".into());
        assert_eq!(orm::search(&env, "product.product", &d, None, None, 0)?, vec![p]);
        assert_eq!(orm::name_search(&env, "product.product", "desk", 5)?.len(), 1);
        Ok(())
    }).unwrap();
}

#[test]
fn partner_complete_name() {
    let (reg, st) = setup();
    let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env)?;
        let co = orm::create(&env, "res.partner", row(&[("name", "Acme".into())]))?;
        let p = orm::create(&env, "res.partner", row(&[("name", "Jo".into()), ("parent_id", co.into())]))?;
        assert_eq!(text(&rec(&env, "res.partner", p)?, "complete_name").unwrap(), "Acme, Jo");
        assert_eq!(text(&rec(&env, "res.partner", co)?, "complete_name").unwrap(), "Acme");
        orm::write(&env, "res.partner", &[p], row(&[("name", "Joanna".into())]))?;
        assert_eq!(text(&rec(&env, "res.partner", p)?, "complete_name").unwrap(), "Acme, Joanna");
        Ok(())
    }).unwrap();
}

#[test]
fn stored_related_fields_fill_generically() {
    let (reg, st) = setup();
    let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env)?;
        let partner = orm::create(&env, "res.partner", row(&[("name", "Cust".into())]))?;
        let so = orm::create(&env, "sale.order", row(&[("partner_id", partner.into()), ("order_line", lines(vec![row(&[("name", "x".into()), ("price_unit", 5.0.into())])]))]))?;
        // sale.order.line.currency_id / order_partner_id are stored related fields: filled from the order without a hand-written compute
        let l = children(&env, "sale.order.line", "order_id", so)?;
        assert_eq!(id_of(&l[0], "order_partner_id"), Some(partner));
        Ok(())
    }).unwrap();
}

#[test]
fn country_codes_are_stored_upper_case() {
    let (reg, st) = setup();
    let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1);
        let id = orm::create(&env, "res.country", row(&[("name", "Testland".into()), ("code", "tl".into())]))?;
        assert_eq!(text(&rec(&env, "res.country", id)?, "code").unwrap(), "TL");
        orm::write(&env, "res.country", &[id], row(&[("code", "tz".into())]))?;
        assert_eq!(text(&rec(&env, "res.country", id)?, "code").unwrap(), "TZ");
        Ok(())
    }).unwrap();
}
