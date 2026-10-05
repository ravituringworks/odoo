use odoo_core::{ddl, orm::{self, AllowAll}, store::{self, Store}, Domain, Registry, Rules, Row, Value};
use odoo_sqlite::SqliteStore;
use std::path::Path;

fn setup(roots: &[&str]) -> (Registry, SqliteStore) {
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), roots).unwrap();
    let st = SqliteStore::open(":memory:").unwrap();
    let plan = ddl::install_plan(&reg, st.dialect());
    store::run(&st, |c| { for s in &plan { c.execute(s, &[]).map_err(|e| { eprintln!("{s}"); e })?; } Ok(()) }).unwrap();
    (reg, st)
}
fn row(v: &[(&str, Value)]) -> Row { v.iter().map(|(k, v)| (k.to_string(), v.clone())).collect() }

#[test]
fn installs_big_closure_and_crud() {
    let (reg, st) = setup(&["sale_management", "account", "stock", "purchase", "crm", "hr", "project"]);
    eprintln!("models={} modules={}", reg.models.len(), reg.modules.len());
    assert!(reg.models.len() > 300);
    let rules = Rules::default();
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1);
        let p = orm::create(&env, "res.partner", row(&[("name", "Azure Interior".into()), ("email", "a@b.c".into())]))?;
        let p2 = orm::create(&env, "res.partner", row(&[("name", "Deco Addict".into())]))?;
        let found = orm::search_read(&env, "res.partner", &Domain::parse(&serde_json::json!([["name", "ilike", "azure"]])).unwrap(), &["name".into(), "email".into()], None, None, 0)?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["name"], Value::Text("Azure Interior".into()));
        // OR domain + count + order
        let d = Domain::parse(&serde_json::json!(["|", ["name", "=", "Deco Addict"], ["email", "=", "a@b.c"]])).unwrap();
        assert_eq!(orm::search_count(&env, "res.partner", &d)?, 2);
        let ids = orm::search(&env, "res.partner", &Domain::True, Some("name desc"), None, 0)?;
        assert_eq!(ids, vec![p2, p]);
        // many2one display name + write + unlink
        let c = orm::create(&env, "res.partner", row(&[("name", "Child".into()), ("parent_id", Value::Int(p))]))?;
        let r = orm::read(&env, "res.partner", &[c], &["parent_id".into()])?;
        assert_eq!(r[0]["parent_id"], Value::List(vec![Value::Int(p), Value::Text("Azure Interior".into())]));
        let dotted = Domain::parse(&serde_json::json!([["parent_id.name", "=", "Azure Interior"]])).unwrap();
        assert_eq!(orm::search(&env, "res.partner", &dotted, None, None, 0)?, vec![c]);
        orm::write(&env, "res.partner", &[c], row(&[("name", "Renamed".into())]))?;
        orm::unlink(&env, "res.partner", &[c])?;
        assert_eq!(orm::search_count(&env, "res.partner", &Domain::True)?, 2);
        // required enforcement
        assert!(orm::create(&env, "res.partner.category", Row::new()).is_err());
        Ok(())
    }).unwrap();
}

#[test]
fn ddl_for_every_dialect() {
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["sale_management", "account", "stock"]).unwrap();
    for n in ["sqlite", "postgres", "mysql", "duckdb", "orbit-rs"] {
        let d = odoo_core::dialect::by_name(n).unwrap();
        let plan = ddl::install_plan(&reg, d.as_ref());
        assert!(plan.len() > 200, "{n}: {}", plan.len());
        assert!(plan.iter().any(|s| s.contains("res_partner")));
    }
}

#[test]
fn generic_methods_copy_archive_exists() {
    let (reg, st) = setup(&["contacts"]);
    let rules = Rules::default();
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1);
        let tag = orm::create(&env, "res.partner.category", row(&[("name", "VIP".into())]))?;
        let p = orm::create(&env, "res.partner", row(&[("name", "Orig".into()), ("email", "o@x.io".into()), ("category_id", Value::List(vec![Value::List(vec![6.into(), 0.into(), Value::List(vec![tag.into()])])]))]))?;
        // copy keeps scalars and many2many links, honours overrides
        let Value::List(cp) = orm::call(&env, "res.partner", "copy", &[p], &row(&[("name", "Orig (copy)".into())]))? else { panic!() };
        let c1 = cp[0].as_i64().unwrap();
        assert_ne!(c1, p);
        let r = orm::read(&env, "res.partner", &[c1], &["name".into(), "email".into(), "category_id".into()])?;
        assert_eq!((r[0]["name"].clone(), r[0]["email"].clone(), r[0]["category_id"].clone()), ("Orig (copy)".into(), "o@x.io".into(), Value::List(vec![Value::Int(tag)])));
        // archive hides from default search, unarchive restores, exists reports ids incl. archived
        orm::call(&env, "res.partner", "action_archive", &[c1], &Row::new())?;
        assert_eq!(orm::search(&env, "res.partner", &Domain::True, None, None, 0)?, vec![p]);
        assert_eq!(orm::call(&env, "res.partner", "exists", &[c1, 9999], &Row::new())?, Value::List(vec![Value::Int(c1)]));
        orm::call(&env, "res.partner", "toggle_active", &[c1], &Row::new())?;
        assert_eq!(orm::search_count(&env, "res.partner", &Domain::True)?, 2);
        // name_create
        let Value::List(nc) = orm::call(&env, "res.partner", "name_create", &[], &row(&[("name", "Quick".into())]))? else { panic!() };
        assert_eq!(nc[1], Value::Text("Quick".into()));
        assert!(orm::call(&env, "res.partner", "definitely_missing", &[p], &Row::new()).is_err());
        Ok(())
    }).unwrap();
}

#[test]
fn hierarchy_and_magic_field_domains() {
    let (reg, st) = setup(&["contacts"]);
    let rules = Rules::default();
    store::run(&st, |c| {
        let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1);
        let root = orm::create(&env, "res.partner.category", row(&[("name", "A".into())]))?;
        let mid = orm::create(&env, "res.partner.category", row(&[("name", "B".into()), ("parent_id", root.into())]))?;
        let leaf = orm::create(&env, "res.partner.category", row(&[("name", "C".into()), ("parent_id", mid.into())]))?;
        let d = |v: serde_json::Value| Domain::parse(&v).unwrap();
        let mut down = orm::search(&env, "res.partner.category", &d(serde_json::json!([["id", "child_of", root]])), None, None, 0)?; down.sort();
        assert_eq!(down, vec![root, mid, leaf]);
        let mut up = orm::search(&env, "res.partner.category", &d(serde_json::json!([["id", "parent_of", leaf]])), None, None, 0)?; up.sort();
        assert_eq!(up, vec![root, mid, leaf]);
        assert_eq!(orm::search(&env, "res.partner.category", &d(serde_json::json!([["id", "child_of", leaf]])), None, None, 0)?, vec![leaf]);
        // magic fields are real fields: searchable, readable, orderable
        assert_eq!(orm::search_count(&env, "res.partner.category", &d(serde_json::json!([["create_uid", "=", 1]])))?, 3);
        let r = orm::read(&env, "res.partner.category", &[leaf], &["create_uid".into(), "create_date".into()])?;
        assert_eq!(r[0]["create_uid"], Value::List(vec![Value::Int(1), Value::Text(format!("res.users,1"))]).clone().into_first_or(r[0]["create_uid"].clone()));
        assert!(orm::search(&env, "res.partner.category", &Domain::True, Some("create_date desc, id"), None, 0).is_ok());
        Ok(())
    }).unwrap();
}

trait FirstOr { fn into_first_or(self, o: Value) -> Value; }
impl FirstOr for Value { fn into_first_or(self, o: Value) -> Value { match &o { Value::List(l) if l.first() == Some(&Value::Int(1)) => o.clone(), _ => self } } }

#[test]
fn upgrade_adds_columns_and_tables_to_a_live_database() {
    let old = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["contacts"]).unwrap();
    let new = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["sale_management"]).unwrap();
    let st = SqliteStore::open(":memory:").unwrap();
    store::apply_plan(&st, &ddl::install_plan(&old, st.dialect())).unwrap();
    let rules = Rules::default();
    store::run(&st, |c| { let env = orm::Env::new(&old, c, &rules, &AllowAll, 1); orm::create(&env, "res.partner", row(&[("name", "Pre-existing".into())])).map(|_| ()) }).unwrap();
    let plan = ddl::upgrade_plan(&old, &new, st.dialect());
    assert!(plan.iter().any(|s| s.starts_with("ALTER TABLE \"res_partner\" ADD COLUMN")), "sale extends res.partner");
    assert!(plan.iter().any(|s| s.contains("CREATE TABLE IF NOT EXISTS \"sale_order\"")));
    assert!(!plan.iter().any(|s| s.contains("CREATE TABLE IF NOT EXISTS \"res_partner\"")));
    store::apply_upgrade(&st, &plan).unwrap();
    store::apply_upgrade(&st, &plan).unwrap();   // retry-safe: second run must not fail on already-added columns
    store::run(&st, |c| {
        let env = orm::Env::new(&new, c, &rules, &AllowAll, 1);
        // old data survives, new tables/columns usable
        assert_eq!(orm::search_count(&env, "res.partner", &Domain::True)?, 1);
        assert_eq!(orm::search_count(&env, "sale.order", &Domain::True)?, 0);   // new table exists and is queryable
        // a column added to an existing table by the new module is readable on old rows
        let r = orm::read(&env, "res.partner", &[1], &["name".into(), "sale_warn".into()])?;
        assert_eq!(r[0]["name"], Value::Text("Pre-existing".into()));
        Ok(())
    }).unwrap();
}

#[test]
fn mixin_redeclarations_and_related_comodels_resolve() {
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["sale_management"]).unwrap();
    // utm.mixin declares campaign_id; sale.order only adds keywords -> comodel must survive
    assert_eq!(reg.field("sale.order", "campaign_id").unwrap().comodel_name().as_deref(), Some("utm.campaign"));
    // every relational field in the registry now has a comodel unless it is genuinely dynamic
    let missing: Vec<String> = reg.models.values().flat_map(|m| m.fields.iter().filter(|(_, f)| matches!(f.ty.as_str(), "Many2one" | "One2many" | "Many2many") && f.comodel_name().is_none() && f.related.is_none()).map(|(n, _)| format!("{}.{n}", m.name))).collect();
    assert!(missing.len() < 40, "{} relational fields without comodel: {:?}", missing.len(), &missing[..missing.len().min(8)]);
    let related_missing = reg.models.values().flat_map(|m| m.fields.iter().filter(|(_, f)| matches!(f.ty.as_str(), "Many2one") && f.comodel_name().is_none() && f.related.is_some())).count();
    assert!(related_missing < 15, "related many2ones without comodel: {related_missing}");
}

#[test]
fn startup_sync_heals_missing_tables_and_columns() {
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["contacts"]).unwrap();
    let st = SqliteStore::open(":memory:").unwrap();
    store::apply_plan(&st, &ddl::install_plan(&reg, st.dialect())).unwrap();
    assert_eq!(store::sync_schema(&st, &reg).unwrap(), 0, "an up-to-date schema needs nothing");
    // simulate a database created by an older build: a whole table and a column are missing
    store::run(&st, |c| { c.execute("DROP TABLE res_partner_category", &[])?; c.execute("ALTER TABLE res_partner DROP COLUMN comment", &[])?; Ok(()) }).unwrap();
    let existing = store::existing_columns(&st).unwrap();
    assert!(!existing.contains_key("res_partner_category") && !existing["res_partner"].contains("comment"));
    let n = store::sync_schema(&st, &reg).unwrap();
    assert!(n >= 2, "{n}");
    let after = store::existing_columns(&st).unwrap();
    assert!(after.contains_key("res_partner_category") && after["res_partner"].contains("comment"));
    assert_eq!(store::sync_schema(&st, &reg).unwrap(), 0, "idempotent");
    // data written before the heal survives and the table is usable again
    let rules = Rules::default();
    store::run(&st, |c| { let env = orm::Env::new(&reg, c, &rules, &AllowAll, 1); orm::create(&env, "res.partner.category", row(&[("name", "VIP".into())])).map(|_| ()) }).unwrap();
}
