//! pos.session methods (pos_session_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{rules_for, util::*};
use pos_common::*;

#[test]
fn session_opening_cash_control_and_closing_with_cash_difference() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        let s = mk(&env, "pos.session", &[("config_id", f.cfg.into())]);
        assert_eq!(text(&rec(&env, "pos.session", s)?, "state").unwrap(), "opening_control");
        // computes: cash journal of the first cash method, cash control, currency
        assert_eq!(rid(&rd(&env, "pos.session", s, "cash_journal_id")), Some(f.journal));
        assert_eq!(rd(&env, "pos.session", s, "cash_control"), Value::Bool(true));
        assert_eq!(rd(&env, "pos.session", s, "is_in_company_currency"), Value::Bool(true));
        assert_eq!(rd(&env, "pos.session", s, "order_count"), Value::Int(0));
        // only one open session per register
        assert!(err_of(c, || orm::create(&env, "pos.session", row(&[("config_id", f.cfg.into())]))).contains("Another session is already opened"));
        // opening control counts the float and names the session
        call(&env, "pos.session", "set_opening_control", &[s], row(&[("cashbox_value", 50.0.into()), ("notes", "float counted".into())]))?;
        let r = rec(&env, "pos.session", s)?;
        assert_eq!((text(&r, "state").unwrap(), num(&r, "cash_register_balance_start"), text(&r, "opening_notes").unwrap()), ("opened".to_string(), 50.0, "float counted".to_string()));
        assert!(text(&r, "name").unwrap().starts_with("POS/"));
        // the same call on an opened session does nothing
        call(&env, "pos.session", "set_opening_control", &[s], row(&[("cashbox_value", 99.0.into()), ("notes", "".into())]))?;
        assert_eq!(num(&rec(&env, "pos.session", s)?, "cash_register_balance_start"), 50.0);

        // two orders: 22 in cash and 22 by card on an invoice
        let a = draft_order(&env, &f, s, 2.0);
        assert_eq!((num(&rec(&env, "pos.order", a)?, "amount_total"), num(&rec(&env, "pos.order", a)?, "amount_tax")), (22.0, 2.0));
        assert_eq!(rd(&env, "pos.session", s, "order_count"), Value::Int(1));
        // a draft order blocks closing
        assert!(err_of(c, || call(&env, "pos.session", "action_pos_session_closing_control", &[s], Row::new())).contains("orders are still in draft"));
        let chk = call(&env, "pos.session", "_cannot_close_session", &[s], Row::new())?;
        let Value::Map(m) = chk else { panic!("{chk:?}") }; assert_eq!(m["message"], Value::Text("You cannot close the POS when orders are still in draft".into()));
        assert!(err_of(c, || call(&env, "pos.session", "_check_if_no_draft_orders", &[s], Row::new())).contains("Pay or cancel the following orders"));
        pay(&env, a, f.cash, 22.0);
        call(&env, "pos.order", "action_pos_order_paid", &[a], Row::new())?;
        let b = draft_order(&env, &f, s, 2.0);
        orm::write(&env, "pos.order", &[b], row(&[("partner_id", f.partner.into())]))?;
        pay(&env, b, f.bank, 22.0);
        call(&env, "pos.order", "action_pos_order_invoice", &[b], Row::new())?;

        // expected cash = float + cash payments; cash in/out moves it
        assert_eq!(rd(&env, "pos.session", s, "cash_register_balance_end").as_f64(), Some(72.0));
        assert_eq!(rd(&env, "pos.session", s, "total_payments_amount").as_f64(), Some(44.0));
        call(&env, "pos.session", "try_cash_in_out", &[s], row(&[("_type", "out".into()), ("amount", 5.0.into()), ("reason", "milk".into()), ("extras", Value::Map(row(&[("translatedType", "out".into())])))]))?;
        assert_eq!(rd(&env, "pos.session", s, "cash_register_balance_end").as_f64(), Some(67.0));
        let data = call(&env, "pos.session", "get_closing_control_data", &[s], Row::new())?;
        let Value::Map(d) = data else { panic!() };
        let Value::Map(cd) = &d["default_cash_details"] else { panic!() };
        assert_eq!((cd["amount"].as_f64(), cd["opening"].as_f64(), cd["payment_amount"].as_f64()), (Some(67.0), Some(50.0), Some(22.0)));
        let Value::List(moves) = &cd["moves"] else { panic!() }; assert_eq!(moves.len(), 1);
        let Value::List(nc) = &d["non_cash_payment_methods"] else { panic!() };
        assert!(nc.iter().any(|x| matches!(x, Value::Map(m) if m["name"] == Value::Text("Card".into()) && m["amount"].as_f64() == Some(22.0) && m["number"].as_i64() == Some(1))));
        // counted 65 instead of 67: a loss
        let r = call(&env, "pos.session", "post_closing_cash_details", &[s], row(&[("counted_cash", 65.0.into())]))?;
        let Value::Map(r) = r else { panic!() }; assert_eq!(r["successful"], Value::Bool(true));
        assert_eq!(rd(&env, "pos.session", s, "cash_register_difference").as_f64(), Some(-2.0));
        let out = call(&env, "pos.session", "close_session_from_ui", &[s], Row::new())?;
        let Value::Map(out) = out else { panic!() }; assert_eq!(out["successful"], Value::Bool(true), "{out:?}");
        let r = rec(&env, "pos.session", s)?;
        assert_eq!(text(&r, "state").unwrap(), "closed"); assert!(id_of(&r, "move_id").is_some()); assert!(text(&r, "stop_at").is_some());
        assert_eq!(text(&rec(&env, "pos.order", a)?, "state").unwrap(), "done");      // paid orders are settled
        assert_eq!(text(&rec(&env, "pos.order", b)?, "state").unwrap(), "invoiced");  // invoiced ones stay
        // the loss was booked on the cash journal against its loss account: a balanced entry of 2
        let lines = children(&env, "account.bank.statement.line", "journal_id", f.journal)?;
        let loss_line = lines.iter().find(|l| num(l, "amount") == -2.0).expect("statement line with the cash loss");
        let ml = children(&env, "account.move.line", "move_id", id_of(loss_line, "move_id").unwrap())?;
        assert_eq!(ml.iter().map(|l| num(l, "debit")).sum::<f64>(), 2.0); assert_eq!(ml.iter().map(|l| num(l, "credit")).sum::<f64>(), 2.0);
        // a closed session cannot be closed again
        assert!(err_of(c, || call(&env, "pos.session", "action_pos_session_closing_control", &[s], Row::new())).contains("already closed"));
        let again = call(&env, "pos.session", "_cannot_close_session", &[s], Row::new())?;
        let Value::Map(again) = again else { panic!() }; assert_eq!(again["title"], Value::Text("Session already closed".into()));
        // the next session starts with what was counted
        let s2 = mk(&env, "pos.session", &[("config_id", f.cfg.into())]);
        assert_eq!(num(&rec(&env, "pos.session", s2)?, "cash_register_balance_start"), 65.0);
        assert_eq!(rd(&env, "pos.config", f.cfg, "last_session_closing_cash").as_f64(), Some(65.0));
        assert!(matches!(rd(&env, "pos.config", f.cfg, "last_session_closing_date"), Value::Text(_)));
        // a session without orders can be dropped while it is still in opening control
        let Value::Map(del) = call(&env, "pos.session", "delete_opening_control_session", &[s2], Row::new())? else { panic!() };
        assert_eq!(del["status"], Value::Text("success".into()));
        assert!(rec(&env, "pos.session", s2).is_err());
        Ok(())
    }).unwrap();
}
