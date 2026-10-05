use odoo_core::{ddl, store, Registry};
use odoo_postgres::PgStore;
use std::path::Path;

#[test]
fn startup_sync_heals_on_postgres() {
    let Ok(url) = std::env::var("ODOO_PG_URL") else { eprintln!("ODOO_PG_URL not set: skipping"); return };
    let schema = format!("sync{}", std::process::id());
    let mut admin = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    admin.batch_execute(&format!("CREATE SCHEMA {schema}")).unwrap();
    let st = PgStore::open(&format!("{url} options='-c search_path={schema}'")).unwrap();
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["contacts"]).unwrap();
    store::apply_plan(&st, &ddl::install_plan(&reg, store::Store::dialect(&st))).unwrap();
    assert_eq!(store::sync_schema(&st, &reg).unwrap(), 0);
    store::run(&st, |c| { c.execute("DROP TABLE res_partner_category CASCADE", &[])?; c.execute("ALTER TABLE res_partner DROP COLUMN comment", &[])?; Ok(()) }).unwrap();
    assert!(store::sync_schema(&st, &reg).unwrap() >= 2);
    let after = store::existing_columns(&st).unwrap();
    assert!(after.contains_key("res_partner_category") && after["res_partner"].contains("comment"));
    assert_eq!(store::sync_schema(&st, &reg).unwrap(), 0);
    admin.batch_execute(&format!("DROP SCHEMA {schema} CASCADE")).unwrap();
}
