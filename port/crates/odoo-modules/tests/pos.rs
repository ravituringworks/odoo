//! Tests for the point_of_sale back-office methods in `pos_methods` (config, payment methods, payments).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{account, rules_for, util::*};
use pos_common::*;

#[test]
fn config_computes_constraints_and_guards() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        // cash_control follows the payment methods; currency comes from the company; warehouse from the picking type / company
        assert_eq!(rd(&env, "pos.config", f.cfg, "cash_control"), Value::Bool(true));
        assert!(rid(&rd(&env, "pos.config", f.cfg, "currency_id")).is_some());
        let other = mk(&env, "pos.config", &[("name", "Online".into()), ("picking_type_id", rec(&env, "pos.config", f.cfg)?["picking_type_id"].clone()), ("payment_method_ids", cmd6(&[f.bank]))]);
        assert_eq!(rd(&env, "pos.config", other, "cash_control"), Value::Bool(false));
        assert_eq!(rd(&env, "pos.config", f.cfg, "has_active_session"), Value::Bool(false));
        assert_eq!(rd(&env, "pos.config", f.cfg, "current_session_state"), Value::Bool(false));
        assert_eq!(rd(&env, "pos.config", f.cfg, "pos_session_duration"), Value::Text("0".into()));
        // no chart yet -> cannot start; then an accounting entry exists -> can start
        assert!(err_of(c, || call(&env, "pos.config", "_check_before_creating_new_session", &[f.cfg], Row::new())).contains("No chart of account configured"));
        assert_eq!(rd(&env, "pos.config", f.cfg, "company_has_template"), Value::Bool(false));
        let m = mk(&env, "account.move", &[("move_type", "entry".into())]);
        mk(&env, "account.move.line", &[("move_id", m.into()), ("account_id", account::ensure_account(&env, "asset_cash", "Cash")?.into()), ("debit", 1.0.into()), ("credit", 0.0.into())]);
        mk(&env, "account.move.line", &[("move_id", m.into()), ("account_id", account::ensure_account(&env, "income", "Sales")?.into()), ("debit", 0.0.into()), ("credit", 1.0.into())]);
        assert_eq!(rd(&env, "pos.config", f.cfg, "company_has_template"), Value::Bool(true));
        call(&env, "pos.config", "_check_before_creating_new_session", &[f.cfg], Row::new())?;
        // no payment method -> refused; no loss/profit account on a cash journal -> refused
        let bare = mk(&env, "pos.config", &[("name", "Bare".into()), ("picking_type_id", rec(&env, "pos.config", f.cfg)?["picking_type_id"].clone())]);
        assert!(err_of(c, || call(&env, "pos.config", "_check_payment_method_ids", &[bare], Row::new())).contains("at least one payment method"));
        let j2 = mk(&env, "account.journal", &[("name", "Cash 2".into()), ("code", "CSH2".into()), ("type", "cash".into())]);
        let pm2 = mk(&env, "pos.payment.method", &[("name", "Cash 2".into()), ("journal_id", j2.into())]);
        orm::write(&env, "pos.config", &[bare], row(&[("payment_method_ids", cmd6(&[pm2]))]))?;
        assert!(err_of(c, || call(&env, "pos.config", "_check_profit_loss_cash_journal", &[bare], Row::new())).contains("loss and profit account"));
        // a cash method belongs to one register only
        assert!(err_of(c, || orm::write(&env, "pos.config", &[other], row(&[("payment_method_ids", cmd6(&[f.cash]))]))).contains("already used in another Point of Sale"));
        // default pricelist must be available when pricelists are used
        let pl = mk(&env, "product.pricelist", &[("name", "Retail".into())]);
        assert!(err_of(c, || orm::write(&env, "pos.config", &[other], row(&[("use_pricelist", true.into()), ("pricelist_id", pl.into())]))).contains("default pricelist must be included"));
        orm::write(&env, "pos.config", &[other], row(&[("use_pricelist", true.into()), ("available_pricelist_ids", cmd6(&[pl])), ("pricelist_id", pl.into())]))?;
        assert_eq!(call(&env, "pos.config", "_get_available_pricelists", &[other], Row::new())?, Value::List(vec![Value::Int(pl)]));
        // customer display needs an IoT box
        assert!(err_of(c, || orm::write(&env, "pos.config", &[other], row(&[("customer_display_type", "proxy".into())]))).contains("iot box"));
        // an emptied tip product with tips enabled has no default product here
        assert!(err_of(c, || orm::write(&env, "pos.config", &[other], row(&[("iface_tipproduct", true.into()), ("tip_product_id", Value::Bool(false))]))).contains("default tip product is missing"));
        // payment method lookup by type and helper actions
        assert_eq!(call(&env, "pos.config", "_get_payment_method", &[f.cfg], row(&[("payment_type", "bank".into())]))?, Value::Int(f.bank));
        assert_eq!(call(&env, "pos.config", "_get_payment_method", &[f.cfg], row(&[("payment_type", "pay_later".into())]))?, Value::Int(f.later));
        let jx = call(&env, "pos.config", "_is_journal_exist", &[], row(&[("journal_code", "XYZ".into()), ("name", "Extra cash".into()), ("company_id", 1.into())]))?;
        assert_eq!(call(&env, "pos.config", "_is_journal_exist", &[], row(&[("journal_code", "XYZ".into()), ("name", "Extra cash".into()), ("company_id", 1.into())]))?, jx);
        // the register is frozen while a session is open
        let s = mk(&env, "pos.session", &[("config_id", f.cfg.into())]);
        assert_eq!(rd(&env, "pos.config", f.cfg, "has_active_session"), Value::Bool(true));
        assert_eq!(rid(&rd(&env, "pos.config", f.cfg, "current_session_id")), Some(s));
        assert_eq!(rd(&env, "pos.config", f.cfg, "current_session_state"), Value::Text("opening_control".into()));
        assert_eq!(rd(&env, "pos.config", f.cfg, "pos_session_state"), Value::Text("opening_control".into()));
        assert_eq!(rid(&rd(&env, "pos.config", f.cfg, "current_user_id")), Some(1));
        let e = err_of(c, || orm::write(&env, "pos.config", &[f.cfg], row(&[("limit_categories", true.into())])));
        assert!(e.contains("can't modify Limit Categories while a session is open") || e.contains("while a session is open"), "{e}");
        orm::write(&env, "pos.config", &[f.cfg], row(&[("name", "Shop 1".into())]))?;   // free fields stay editable
        // _open_session / open_existing_session_cb return the session form
        let act = call(&env, "pos.config", "open_existing_session_cb", &[f.cfg], Row::new())?;
        let Value::Map(a) = act else { panic!() }; assert_eq!(a["res_id"], Value::Int(s)); assert_eq!(a["res_model"], Value::Text("pos.session".into()));
        Ok(())
    }).unwrap();
}

#[test]
fn payment_method_computes_and_guards() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        // type and is_cash_count come from the journal
        assert_eq!(rd(&env, "pos.payment.method", f.cash, "type"), Value::Text("cash".into()));
        assert_eq!(rd(&env, "pos.payment.method", f.cash, "is_cash_count"), Value::Bool(true));
        assert_eq!(rd(&env, "pos.payment.method", f.bank, "type"), Value::Text("bank".into()));
        assert_eq!(rd(&env, "pos.payment.method", f.bank, "is_cash_count"), Value::Bool(false));
        assert_eq!(rd(&env, "pos.payment.method", f.later, "type"), Value::Text("pay_later".into()));
        // moving the method to the bank journal changes both
        let bank_j = rec(&env, "pos.payment.method", f.bank)?["journal_id"].as_i64().unwrap();
        orm::write(&env, "pos.payment.method", &[f.cash], row(&[("journal_id", bank_j.into())]))?;
        assert_eq!(rd(&env, "pos.payment.method", f.cash, "type"), Value::Text("bank".into()));
        assert_eq!(rd(&env, "pos.payment.method", f.cash, "is_cash_count"), Value::Bool(false));
        orm::write(&env, "pos.payment.method", &[f.cash], row(&[("journal_id", f.journal.into())]))?;
        assert_eq!(rd(&env, "pos.payment.method", f.cash, "is_cash_count"), Value::Bool(true));
        // QR methods need a bank account on a bank journal and a QR format
        let e = err_of(c, || orm::create(&env, "pos.payment.method", row(&[("name", "QR".into()), ("payment_method_type", "qr_code".into()), ("journal_id", bank_j.into())])));
        assert!(e.contains("bank account must be defined"), "{e}");
        // the same rule holds when an existing method is switched to QR payments
        let plain = mk(&env, "pos.payment.method", &[("name", "Plain".into())]);
        let e = err_of(c, || orm::write(&env, "pos.payment.method", &[plain], row(&[("payment_method_type", "qr_code".into())])));
        assert!(e.contains("bank account must be defined"), "{e}");
        call(&env, "pos.payment.method", "_check_payment_method", &[plain], Row::new())?;   // the rolled-back write left it plain
        // terminal methods drop the QR format and vice versa
        let t = mk(&env, "pos.payment.method", &[("name", "Terminal".into()), ("payment_method_type", "terminal".into()), ("qr_code_method", "sepa".into())]);
        assert_eq!(rd(&env, "pos.payment.method", t, "qr_code_method"), Value::Bool(false));
        // open sessions freeze a method; only its sequence may change
        orm::write(&env, "pos.payment.method", &[f.cash], row(&[("config_ids", cmd6(&[f.cfg]))]))?;
        assert!(orm::read(&env, "pos.payment.method", &[f.cash], &["open_session_ids".into()]).is_ok());
        let s = mk(&env, "pos.session", &[("config_id", f.cfg.into())]);
        assert_eq!(call(&env, "pos.payment.method", "_is_write_forbidden", &[f.cash], row(&[("fields", Value::List(vec!["name".into()]))]))?, Value::Bool(true));
        assert_eq!(call(&env, "pos.payment.method", "_is_write_forbidden", &[f.cash], row(&[("fields", Value::List(vec!["sequence".into()]))]))?, Value::Bool(false));
        let e = err_of(c, || orm::write(&env, "pos.payment.method", &[f.cash], row(&[("name", "Till".into())])));
        assert!(e.contains("close and validate the following open PoS Sessions") && e.contains(&text(&rec(&env, "pos.session", s)?, "name").unwrap()), "{e}");
        orm::write(&env, "pos.payment.method", &[f.cash], row(&[("sequence", 5.into())]))?;
        Ok(())
    }).unwrap();
}
