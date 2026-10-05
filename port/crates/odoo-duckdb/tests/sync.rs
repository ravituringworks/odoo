use odoo_core::{ddl, store, Registry};
use odoo_duckdb::DuckStore;
use std::path::Path;

#[test]
fn startup_sync_heals_on_duckdb() {
    let st = DuckStore::open(":memory:").unwrap();
    let reg = Registry::load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema")), &["contacts"]).unwrap();
    store::apply_plan(&st, &ddl::install_plan(&reg, store::Store::dialect(&st))).unwrap();
    assert_eq!(store::sync_schema(&st, &reg).unwrap(), 0);
    store::run(&st, |c| { c.execute("DROP TABLE res_partner_category", &[])?; Ok(()) }).unwrap();
    assert!(store::sync_schema(&st, &reg).unwrap() >= 1);
    assert!(store::existing_columns(&st).unwrap().contains_key("res_partner_category"));
    assert_eq!(store::sync_schema(&st, &reg).unwrap(), 0);
}
