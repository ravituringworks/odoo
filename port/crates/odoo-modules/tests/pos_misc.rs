//! Small POS extensions of other models (pos_misc_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{rules_for, stock, util::*};
use pos_common::*;

#[test]
fn partner_journal_and_tax_guards() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        // partner: orders of the contact and of its children count; partners with orders cannot be deleted
        let company = mk(&env, "res.partner", &[("name", "Corp".into()), ("is_company", true.into())]);
        let contact = mk(&env, "res.partner", &[("name", "Ann".into()), ("parent_id", company.into())]);
        let o = draft_order(&env, &f, s, 1.0);
        orm::write(&env, "pos.order", &[o], row(&[("partner_id", contact.into())]))?;
        assert_eq!(rd(&env, "res.partner", contact, "pos_order_count"), Value::Int(1));
        assert_eq!(rd(&env, "res.partner", company, "pos_order_count"), Value::Int(1));
        assert_eq!(rd(&env, "res.partner", f.partner, "pos_order_count"), Value::Int(0));
        assert!(err_of(c, || orm::unlink(&env, "res.partner", &[contact])).contains("cannot delete a customer that has point of sales orders"));
        let Value::Map(a) = call(&env, "res.partner", "action_view_pos_order", &[company], Row::new())? else { panic!() };
        assert_eq!(a["res_model"], Value::Text("pos.order".into()));
        let Value::List(d) = &a["domain"] else { panic!() }; assert!(matches!(&d[0], Value::List(t) if t[0] == Value::Text("partner_id.commercial_partner_id".into())));

        // journal: a journal with a payment method keeps its type; a method used in an opened session blocks archiving/deleting
        assert!(err_of(c, || orm::write(&env, "account.journal", &[f.journal], row(&[("type", "bank".into())]))).contains("associated with a payment method"));
        assert!(err_of(c, || call(&env, "account.journal", "_check_type", &[f.journal], Row::new())).contains("You cannot modify its type"));
        pay(&env, o, f.cash, 11.0); call(&env, "pos.order", "action_pos_order_paid", &[o], Row::new())?;
        let e = err_of(c, || call(&env, "account.journal", "action_archive", &[f.journal], Row::new()));
        assert!(e.contains("payment method Cash that is being used by order") && e.contains("in the active pos session POS/"), "{e}");
        assert!(err_of(c, || orm::unlink(&env, "account.journal", &[f.journal])).contains("being used by order"));
        let free = mk(&env, "account.journal", &[("name", "Misc".into()), ("code", "MSC".into()), ("type", "general".into())]);
        call(&env, "account.journal", "action_archive", &[free], Row::new())?;
        let j1 = call(&env, "account.journal", "_ensure_company_account_journal", &[], Row::new())?;
        assert_eq!(call(&env, "account.journal", "_ensure_company_account_journal", &[], Row::new())?, j1);
        assert_eq!(text(&rec(&env, "account.journal", j1.as_i64().unwrap())?, "code").unwrap(), "POSS");

        // tax: frozen while an order using it sits in an unclosed session, free once the session is closed
        assert!(err_of(c, || orm::write(&env, "account.tax", &[f.tax], row(&[("amount", 12.0.into())]))).contains("forbidden to modify a tax used in a POS order"));
        orm::write(&env, "account.tax", &[f.tax], row(&[("name", "VAT ten".into())]))?;   // a label is harmless
        orm::write(&env, "pos.session", &[s], row(&[("state", "closed".into())]))?;
        orm::write(&env, "account.tax", &[f.tax], row(&[("amount", 12.0.into())]))?;
        Ok(())
    }).unwrap();
}

#[test]
fn categories_bills_and_products() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        // categories: children share the parent's colour, cycles are refused, hierarchy and descendants
        let root = mk(&env, "pos.category", &[("name", "Food".into()), ("color", 7.into())]);
        let kid = mk(&env, "pos.category", &[("name", "Snacks".into()), ("parent_id", root.into())]);
        let leaf = mk(&env, "pos.category", &[("name", "Chips".into()), ("parent_id", kid.into())]);
        assert_eq!(num(&rec(&env, "pos.category", kid)?, "color"), 7.0); assert_eq!(num(&rec(&env, "pos.category", leaf)?, "color"), 7.0);
        let other = mk(&env, "pos.category", &[("name", "Drinks".into()), ("color", 3.into())]);
        orm::write(&env, "pos.category", &[kid], row(&[("parent_id", other.into())]))?;
        assert_eq!(num(&rec(&env, "pos.category", kid)?, "color"), 3.0);
        orm::write(&env, "pos.category", &[kid], row(&[("parent_id", root.into())]))?;
        assert!(err_of(c, || orm::write(&env, "pos.category", &[root], row(&[("parent_id", leaf.into())]))).contains("recursive categories"));
        assert_eq!(call(&env, "pos.category", "_get_hierarchy", &[leaf], Row::new())?, Value::List(vec!["Food".into(), "Snacks".into(), "Chips".into()]));
        let Value::List(desc) = call(&env, "pos.category", "_get_descendants", &[root], Row::new())? else { panic!() };
        assert_eq!(desc.iter().filter_map(|v| v.as_i64()).collect::<std::collections::BTreeSet<_>>(), [root, kid, leaf].into_iter().collect());
        assert_eq!(rd(&env, "pos.category", root, "has_image"), Value::Bool(false));
        // limited categories use the descendants
        orm::write(&env, "pos.config", &[f.cfg], row(&[("limit_categories", true.into()), ("iface_available_categ_ids", cmd6(&[root]))]))?;
        let Value::List(av) = call(&env, "pos.config", "_get_available_categories", &[f.cfg], Row::new())? else { panic!() };
        assert_eq!(av.len(), 3);
        let Value::List(dom) = call(&env, "pos.config", "_get_available_product_domain", &[f.cfg], Row::new())? else { panic!() };
        assert!(dom.iter().any(|t| matches!(t, Value::List(x) if x[0] == Value::Text("pos_categ_ids".into()))));

        // bills: named after their value
        let Value::List(b) = call(&env, "pos.bill", "name_create", &[], row(&[("name", "0.5".into())]))? else { panic!() };
        assert_eq!(num(&rec(&env, "pos.bill", b[0].as_i64().unwrap())?, "value"), 0.5);
        assert!(err_of(c, || call(&env, "pos.bill", "name_create", &[], row(&[("name", "half".into())]))).contains("must be a number"));

        // product colour follows the first POS category; deleting a POS product needs closed sessions
        let tmpl = id_of(&rec(&env, "product.product", f.pen)?, "product_tmpl_id").unwrap();
        orm::write(&env, "product.template", &[tmpl], row(&[("pos_categ_ids", cmd6(&[kid]))]))?;
        assert_eq!(num(&rec(&env, "product.template", tmpl)?, "color"), 7.0);
        let s = opened_session(&env, &f, 0.0);
        assert!(err_of(c, || orm::unlink(&env, "product.template", &[tmpl])).contains("make sure all point of sale sessions are closed"));
        assert!(err_of(c, || orm::unlink(&env, "product.product", &[f.pen])).contains("make sure all point of sale sessions are closed"));
        let plain = mk(&env, "product.template", &[("name", "Not for POS".into())]);
        orm::unlink(&env, "product.template", &[plain])?;
        orm::write(&env, "pos.session", &[s], row(&[("state", "closed".into())]))?;
        // a product inside a combo must stay available in the POS
        let combo = mk(&env, "product.combo", &[("name", "Menu".into())]);
        mk(&env, "product.combo.item", &[("combo_id", combo.into()), ("product_id", f.pen.into())]);
        assert!(err_of(c, || orm::write(&env, "product.template", &[tmpl], row(&[("available_in_pos", false.into())]))).contains("remove this product from the Menu combo"));
        Ok(())
    }).unwrap();
}

#[test]
fn product_info_pos_prices_stock_and_suppliers() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        let tmpl = id_of(&rec(&env, "product.product", f.pen)?, "product_tmpl_id").unwrap();
        let company = rec(&env, "pos.config", f.cfg)?["company_id"].as_i64().unwrap();
        orm::write(&env, "account.tax", &[f.tax], row(&[("company_id", company.into())]))?;
        orm::write(&env, "product.template", &[tmpl], row(&[("taxes_id", cmd6(&[f.tax]))]))?;
        // a pricelist with a 20% rule from 5 units and a fixed rule for everything else
        let pl = mk(&env, "product.pricelist", &[("name", "Trade".into())]);
        mk(&env, "product.pricelist.item", &[("pricelist_id", pl.into()), ("applied_on", "1_product".into()), ("product_tmpl_id", tmpl.into()), ("compute_price", "percentage".into()), ("percent_price", 20.0.into()), ("min_quantity", 5.0.into())]);
        mk(&env, "product.pricelist.item", &[("pricelist_id", pl.into()), ("applied_on", "3_global".into()), ("compute_price", "fixed".into()), ("fixed_price", 9.0.into())]);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("use_pricelist", true.into()), ("available_pricelist_ids", cmd6(&[pl])), ("pricelist_id", pl.into())]))?;
        // a warehouse with stock, a pending delivery and a supplier
        let loc = mk(&env, "stock.location", &[("name", "WH/Stock".into()), ("usage", "internal".into())]);
        let wh = mk(&env, "stock.warehouse", &[("name", "Main".into()), ("code", "MAIN".into()), ("lot_stock_id", loc.into()), ("company_id", company.into())]);
        stock::adjust_quant(&env, f.pen, loc, 20.0)?;
        let cust = stock::ensure_location(&env, "customer", "Customers")?;
        let pk = mk(&env, "stock.picking", &[("picking_type_id", stock::ensure_picking_type(&env, "outgoing")?.into()), ("location_id", loc.into()), ("location_dest_id", cust.into())]);
        mk(&env, "stock.move", &[("picking_id", pk.into()), ("product_id", f.pen.into()), ("product_uom_qty", 4.0.into()), ("location_id", loc.into()), ("location_dest_id", cust.into()), ("state", "confirmed".into())]);
        let vendor = mk(&env, "res.partner", &[("name", "Pens Inc".into())]);
        mk(&env, "product.supplierinfo", &[("partner_id", vendor.into()), ("product_tmpl_id", tmpl.into()), ("price", 3.0.into()), ("delay", 4.into()), ("min_qty", 0.0.into())]);
        mk(&env, "product.supplierinfo", &[("partner_id", vendor.into()), ("product_tmpl_id", tmpl.into()), ("price", 2.0.into()), ("delay", 9.into()), ("min_qty", 100.0.into())]);
        let Value::Map(i) = call(&env, "product.product", "get_product_info_pos", &[f.pen], row(&[("price", 10.0.into()), ("quantity", 2.0.into()), ("pos_config_id", f.cfg.into())]))? else { panic!() };
        let Value::Map(p) = &i["all_prices"] else { panic!() };
        assert_eq!((p["price_without_tax"].as_f64(), p["price_with_tax"].as_f64()), (Some(10.0), Some(11.0)));
        let Value::List(td) = &p["tax_details"] else { panic!() }; assert!(matches!(&td[0], Value::Map(t) if t["name"] == Value::Text("VAT ten".into()) || t["name"] == Value::Text("VAT 10%".into())) && matches!(&td[0], Value::Map(t) if t["amount"].as_f64() == Some(1.0)));
        let Value::List(pls) = &i["pricelists"] else { panic!() };
        assert!(matches!(&pls[0], Value::Map(m) if m["price"].as_f64() == Some(9.0)), "{pls:?}");
        let Value::Map(i5) = call(&env, "product.product", "get_product_info_pos", &[f.pen], row(&[("price", 10.0.into()), ("quantity", 5.0.into()), ("pos_config_id", f.cfg.into())]))? else { panic!() };
        let Value::List(pls5) = &i5["pricelists"] else { panic!() }; assert!(matches!(&pls5[0], Value::Map(m) if m["price"].as_f64() == Some(8.0)));
        let Value::List(whs) = &i["warehouses"] else { panic!() };
        assert!(matches!(&whs[0], Value::Map(m) if m["id"] == Value::Int(wh) && m["available_quantity"].as_f64() == Some(20.0) && m["forecasted_quantity"].as_f64() == Some(16.0)), "{whs:?}");
        let Value::List(sup) = &i["suppliers"] else { panic!() };
        assert_eq!(sup.len(), 1); assert!(matches!(&sup[0], Value::Map(m) if m["price"].as_f64() == Some(3.0) && m["name"] == Value::Text("Pens Inc".into())));
        Ok(())
    }).unwrap();
}

#[test]
fn pickings_from_order_lines_and_picking_type_guard() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        let pt = rec(&env, "pos.config", f.cfg)?["picking_type_id"].as_i64().unwrap();
        let ptr = rec(&env, "stock.picking.type", pt)?;
        let internal = id_of(&ptr, "default_location_src_id").unwrap(); let customers = id_of(&ptr, "default_location_dest_id").unwrap();
        stock::adjust_quant(&env, f.pen, internal, 10.0)?;
        let sale = draft_order(&env, &f, s, 3.0); let back = draft_order(&env, &f, s, -1.0);
        let lines: Vec<i64> = [sale, back].iter().flat_map(|o| ids(&rec(&env, "pos.order", *o).unwrap(), "lines")).collect();
        let Value::List(pks) = call(&env, "stock.picking", "_create_picking_from_pos_order_lines", &[], row(&[("location_dest_id", customers.into()), ("line_ids", Value::List(lines.iter().map(|l| Value::Int(*l)).collect())), ("picking_type_id", pt.into()), ("partner_id", f.partner.into())]))? else { panic!() };
        assert_eq!(pks.len(), 2, "one picking for the sale, one for the return");
        let out = rec(&env, "stock.picking", pks[0].as_i64().unwrap())?; let ret = rec(&env, "stock.picking", pks[1].as_i64().unwrap())?;
        assert_eq!((id_of(&out, "location_id"), id_of(&out, "location_dest_id"), text(&out, "state").unwrap()), (Some(internal), Some(customers), "done".to_string()));
        assert_eq!((id_of(&ret, "location_id"), id_of(&ret, "location_dest_id"), id_of(&ret, "partner_id")), (Some(customers), Some(internal), Some(f.partner)));
        // 3 delivered, 1 taken back
        assert_eq!(stock::on_hand(&env, f.pen, internal)?, 8.0);
        let moves = children(&env, "stock.move", "picking_id", pks[0].as_i64().unwrap())?;
        assert_eq!((num(&moves[0], "product_uom_qty"), id_of(&moves[0], "product_id")), (3.0, Some(f.pen)));
        // _prepare_picking_vals
        let Value::Map(v) = call(&env, "stock.picking", "_prepare_picking_vals", &[], row(&[("picking_type_id", pt.into()), ("location_id", internal.into()), ("location_dest_id", customers.into())]))? else { panic!() };
        assert_eq!((v["move_type"].clone(), v["state"].clone(), v["partner_id"].clone()), (Value::Text("direct".into()), Value::Text("draft".into()), Value::Bool(false)));
        // a picking type used by a register cannot be archived
        let e = err_of(c, || orm::write(&env, "stock.picking.type", &[pt], row(&[("active", false.into())])));
        assert!(e.contains("as it is used by POS configuration 'Shop'"), "{e}");
        Ok(())
    }).unwrap();
}

#[test]
fn settings_screen_and_digest() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("is_posbox", true.into()), ("proxy_ip", "10.0.0.2".into()), ("iface_print_via_proxy", true.into()), ("iface_cashdrawer", true.into()), ("is_header_or_footer", true.into()), ("receipt_header", "Welcome".into()), ("receipt_footer", "Bye".into()), ("is_order_printer", true.into())]))?;
        // the settings open on the last modified register and mirror it
        let st_id = mk(&env, "res.config.settings", &[]);
        let r = rec(&env, "res.config.settings", st_id)?;
        assert_eq!(id_of(&r, "pos_config_id"), Some(f.cfg));
        assert_eq!(text(&r, "pos_receipt_header").as_deref(), Some("Welcome")); assert_eq!(text(&r, "pos_receipt_footer").as_deref(), Some("Bye"));
        assert!(flag(&r, "pos_iface_print_via_proxy") && flag(&r, "pos_iface_cashdrawer") && flag(&r, "pos_is_order_printer"));
        assert!(!flag(&r, "pos_iface_scan_via_proxy"), "not set on the register");
        assert_eq!(call(&env, "res.config.settings", "_is_cashdrawer_displayed", &[st_id], Row::new())?, Value::Bool(true));
        // without the IoT box the proxy options are off; tips need the tip option
        orm::write(&env, "pos.config", &[f.cfg], row(&[("is_posbox", false.into())]))?;
        orm::write(&env, "res.config.settings", &[st_id], row(&[("pos_config_id", f.cfg.into())]))?;
        let r = rec(&env, "res.config.settings", st_id)?; assert!(!flag(&r, "pos_iface_print_via_proxy") && !flag(&r, "pos_iface_cashdrawer"));
        // the pricelist falls back to one in the register currency when the available ones are in another currency
        assert!(id_of(&r, "pos_pricelist_id").is_none() && id_of(&r, "pos_tip_product_id").is_none());
        let Value::Map(m) = call(&env, "res.config.settings", "action_pos_config_create_new", &[st_id], Row::new())? else { panic!() };
        assert_eq!(m["res_model"], Value::Text("pos.config".into()));
        let ui = call(&env, "res.config.settings", "pos_open_ui", &[], Row::new())?; assert_eq!(ui, Value::Null);

        // digest: sales of paid-but-not-invoiced orders in the period
        let s = opened_session(&env, &f, 0.0);
        let o = draft_order(&env, &f, s, 2.0); pay(&env, o, f.cash, 22.0); call(&env, "pos.order", "action_pos_order_paid", &[o], Row::new())?;
        let d = mk(&env, "digest.digest", &[("name", "Weekly".into())]);
        assert_eq!(rd(&env, "digest.digest", d, "kpi_pos_total_value").as_f64(), Some(22.0));
        let past = env.with_ctx("end_datetime", Value::Text("2000-01-01 00:00:00".into()));
        assert_eq!(rd(&past, "digest.digest", d, "kpi_pos_total_value").as_f64(), Some(0.0));
        Ok(())
    }).unwrap();
}
