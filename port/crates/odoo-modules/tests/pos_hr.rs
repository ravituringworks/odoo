//! pos_hr: cashiers by employee (pos_hr_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{pos_hr_methods::sha1_hex, rules_for, util::*};
use pos_common::*;

#[test]
fn sha1_matches_the_reference_vectors() {
    assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    assert_eq!(sha1_hex(b"The quick brown fox jumps over the lazy dog"), "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
    assert_eq!(sha1_hex(&[b'a'; 1000]), "291e9a6c66994949b57ba5e650361e98fc36b1ba");
}

#[test]
fn cashiers_managers_hashed_credentials_and_per_employee_closing() {
    let (reg, st) = setup(&["point_of_sale", "pos_hr", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("module_pos_hr", true.into())]))?;   // frozen once a session is open
        let ann = mk(&env, "hr.employee", &[("name", "Ann".into()), ("barcode", "BADGE-A".into()), ("pin", "1234".into())]);
        let bob = mk(&env, "hr.employee", &[("name", "Bob".into())]);
        // hashed badge and PIN: false when missing
        let Value::List(h) = call(&env, "hr.employee", "get_barcodes_and_pin_hashed", &[ann, bob], Row::new())? else { panic!() };
        let Value::Map(a) = &h[0] else { panic!() }; let Value::Map(b) = &h[1] else { panic!() };
        assert_eq!((a["barcode"].clone(), a["pin"].clone()), (Value::Text(sha1_hex(b"BADGE-A")), Value::Text(sha1_hex(b"1234"))));
        assert_eq!((b["barcode"].clone(), b["pin"].clone()), (Value::Bool(false), Value::Bool(false)));

        // cashier name: the employee, else the user
        let s = opened_session(&env, &f, 0.0);
        let o1 = draft_order(&env, &f, s, 2.0); orm::write(&env, "pos.order", &[o1], row(&[("employee_id", ann.into())]))?;
        assert_eq!(text(&rec(&env, "pos.order", o1)?, "cashier").as_deref(), Some("Ann"));
        let o2 = draft_order(&env, &f, s, 1.0); assert!(rec(&env, "pos.order", o2)?.get("cashier").map_or(true, |v| !matches!(v, Value::Text(t) if t == "Ann")));
        pay(&env, o1, f.cash, 22.0); call(&env, "pos.order", "action_pos_order_paid", &[o1], Row::new())?;
        pay(&env, o2, f.bank, 11.0); call(&env, "pos.order", "action_pos_order_paid", &[o2], Row::new())?;
        assert_eq!(id_of(&children(&env, "pos.payment", "pos_order_id", o1)?[0], "employee_id"), Some(ann), "payments carry the order's cashier");

        // closing data: amounts per cashier, "Others" last; cash moves per employee
        call(&env, "pos.session", "try_cash_in_out", &[s], row(&[("_type", "in".into()), ("amount", 7.0.into()), ("reason", "float".into()), ("extras", Value::Map(row(&[("translatedType", "in".into()), ("employee_id", bob.into())])))]))?;
        let Value::Map(d) = call(&env, "pos.session", "get_closing_control_data", &[s], Row::new())? else { panic!() };
        let Value::Map(cash) = &d["default_cash_details"] else { panic!() };
        let Value::List(per) = &cash["amount_per_employee"] else { panic!() };
        assert!(matches!(&per[0], Value::Map(m) if m["id"] == Value::Int(ann) && m["name"] == Value::Text("Ann".into()) && m["amount"].as_f64() == Some(22.0)));
        let Value::List(moves) = &cash["moves_per_employee"] else { panic!() };
        assert!(matches!(&moves[0], Value::Map(m) if m["id"] == Value::Int(bob) && m["amount"].as_f64() == Some(7.0)));
        let Value::List(nc) = &d["non_cash_payment_methods"] else { panic!() };
        let card = nc.iter().find_map(|x| match x { Value::Map(m) if m["name"] == Value::Text("Card".into()) => Some(m), _ => None }).unwrap();
        let Value::List(cper) = &card["amount_per_employee"] else { panic!() };
        assert!(matches!(&cper[0], Value::Map(m) if m["id"] == Value::Text("others".into()) && m["name"] == Value::Text("Others".into()) && m["amount"].as_f64() == Some(11.0)));
        let pays = children(&env, "pos.payment", "pos_order_id", o1)?.iter().map(|p| p["id"].as_i64().unwrap()).collect::<Vec<_>>();
        let Value::List(agg) = call(&env, "pos.session", "_aggregate_payments_amounts_by_employee", &[s], row(&[("payment_ids", Value::List(pays.into_iter().map(Value::Int).collect()))]))? else { panic!() };
        assert_eq!(agg.len(), 1);

        // the cashier's own sales report
        let Value::Map(rep) = call(&env, "report.pos_hr.single_employee_sales_report", "get_sale_details", &[], row(&[("session_ids", Value::List(vec![s.into()])), ("employee_id", ann.into())]))? else { panic!() };
        assert_eq!((rep["nbr_orders"].as_i64(), rep["employee_name"].clone()), (Some(1), Value::Text("Ann".into())));
        let Value::List(dom) = call(&env, "report.pos_hr.single_employee_sales_report", "_get_domain", &[], row(&[("session_ids", Value::List(vec![s.into()])), ("employee_id", ann.into())]))? else { panic!() };
        assert!(dom.iter().any(|t| matches!(t, Value::List(x) if x[0] == Value::Text("employee_id".into()))));
        let Value::Map(multi) = call(&env, "report.pos_hr.multi_employee_sales_report", "_get_report_values", &[], row(&[("data", Value::Map(row(&[("employee_ids", Value::List(vec![ann.into()]))])))]))? else { panic!() };
        assert_eq!(multi["employee_ids"], Value::List(vec![Value::Int(ann)])); assert!(multi.contains_key("session_ids") && multi.contains_key("date_start"));

        // employee domain of a register restricted to named cashiers
        orm::write(&env, "pos.config", &[f.cfg], row(&[("basic_employee_ids", cmd6(&[ann]))]))?;
        let Value::List(dom) = call(&env, "pos.config", "_employee_domain", &[f.cfg], row(&[("user_id", 1.into())]))? else { panic!() };
        assert_eq!(dom.len(), 1);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("basic_employee_ids", cmd6(&[]))]))?;
        let Value::List(dom) = call(&env, "pos.config", "_employee_domain", &[f.cfg], row(&[("user_id", 1.into())]))? else { panic!() }; assert!(dom.is_empty());

        // employees possibly in use cannot be deleted while a session is open
        let e = err_of(c, || orm::unlink(&env, "hr.employee", &[bob]));
        assert!(e.contains("You cannot delete an employee that may be used in an active PoS session") && e.contains("Employee: Bob - PoS Config(s): Shop"), "{e}");
        orm::write(&env, "pos.session", &[s], row(&[("state", "closed".into())]))?;
        orm::unlink(&env, "hr.employee", &[bob])?;
        Ok(())
    }).unwrap();
}

#[test]
fn manager_employees_are_always_advanced() {
    let (reg, st) = setup(&["point_of_sale", "pos_hr", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        // a POS manager group with a user whose employee must appear on every register
        let grp = mk(&env, "res.groups", &[("name", "POS Manager".into())]);
        mk(&env, "ir.model.data", &[("module", "point_of_sale".into()), ("name", "group_pos_manager".into()), ("model", "res.groups".into()), ("res_id", grp.into())]);
        let boss_user = mk(&env, "res.users", &[("name", "Boss".into()), ("login", "boss".into())]);
        orm::write(&env, "res.users", &[boss_user], row(&[("groups_id", cmd6(&[grp]))]))?;
        let boss = mk(&env, "hr.employee", &[("name", "Boss".into()), ("user_id", boss_user.into())]);
        assert_eq!(call(&env, "pos.config", "_get_group_pos_manager", &[f.cfg], Row::new())?, Value::Int(grp));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("name", "Shop 2".into())]))?;
        assert_eq!(ids(&rec(&env, "pos.config", f.cfg)?, "advanced_employee_ids"), vec![boss]);
        Ok(())
    }).unwrap();
}
