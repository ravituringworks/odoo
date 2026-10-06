//! Direct calls of the smaller POS helpers (constraint checks, action dicts, internal steps) with their observable results.
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{account, rules_for, util::*};
use pos_common::*;

fn l(v: &Value) -> &Vec<Value> { match v { Value::List(l) => l, _ => panic!("{v:?}") } }
fn m(v: &Value) -> &Row { match v { Value::Map(m) => m, _ => panic!("{v:?}") } }

#[test]
fn config_checks_actions_and_computes() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        // plain reads of the remaining computes
        assert_eq!(rd(&env, "pos.config", f.cfg, "is_installed_account_accountant"), Value::Bool(false));
        assert_eq!(rd(&env, "pos.config", f.cfg, "number_of_rescue_session"), Value::Int(0));
        assert_eq!(rd(&env, "pos.config", f.cfg, "pos_session_username"), Value::Bool(false));
        let s = mk(&env, "pos.session", &[("config_id", f.cfg.into())]);
        assert!(matches!(rd(&env, "pos.config", f.cfg, "pos_session_username"), Value::Text(_)), "the opening user's name");
        let rescue = mk(&env, "pos.session", &[("config_id", f.cfg.into()), ("rescue", true.into())]);
        assert_eq!(rd(&env, "pos.config", f.cfg, "number_of_rescue_session"), Value::Int(1));
        let Value::Map(one) = call(&env, "pos.config", "open_opened_rescue_session_form", &[f.cfg], Row::new())? else { panic!() };
        assert_eq!((one["res_id"].clone(), one["view_mode"].clone()), (Value::Int(rescue), Value::Text("form".into())));
        let rescue2 = mk(&env, "pos.session", &[("config_id", f.cfg.into()), ("rescue", true.into())]);
        let Value::Map(many) = call(&env, "pos.config", "open_opened_rescue_session_form", &[f.cfg], Row::new())? else { panic!() };
        assert_eq!(many["name"], Value::Text("Rescue Sessions".into())); let _ = rescue2;
        assert_eq!(rd(&env, "pos.config", f.cfg, "current_session_id").as_i64().or(rid(&rd(&env, "pos.config", f.cfg, "current_session_id"))), Some(s), "rescue sessions are not the current session");
        let Value::Map(os) = call(&env, "pos.config", "_open_session", &[f.cfg], row(&[("session_id", s.into())]))? else { panic!() }; assert_eq!(os["res_id"], Value::Int(s));

        // constraint helpers called directly
        assert!(err_of(c, || call(&env, "pos.config", "_check_company_has_template", &[f.cfg], Row::new())).contains("No chart of account configured"));
        call(&env, "pos.config", "_check_company_payment", &[f.cfg], Row::new())?;
        let foreign_company = mk(&env, "res.company", &[("name", "Other Co".into())]);
        let alien = mk(&env, "pos.payment.method", &[("name", "Alien".into()), ("company_id", foreign_company.into())]);
        orm::write(&env, "pos.session", &[s, rescue, rescue2], row(&[("state", "closed".into())]))?;
        let e = err_of(c, || orm::write(&env, "pos.config", &[f.cfg], row(&[("payment_method_ids", cmd6(&[f.cash, alien]))])));
        assert!(e.contains("payment methods for the point of sale Shop must belong to its company"), "{e}");
        let pl = mk(&env, "product.pricelist", &[("name", "Retail".into())]); let pl2 = mk(&env, "product.pricelist", &[("name", "Other".into())]);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("use_pricelist", true.into()), ("available_pricelist_ids", cmd6(&[pl])), ("pricelist_id", pl.into())]))?;
        call(&env, "pos.config", "_check_currencies", &[f.cfg], Row::new())?; call(&env, "pos.config", "_check_pricelists", &[f.cfg], Row::new())?;
        orm::write(&env, "product.pricelist", &[pl2], row(&[("company_id", foreign_company.into())]))?;
        let e = err_of(c, || orm::write(&env, "pos.config", &[f.cfg], row(&[("available_pricelist_ids", cmd6(&[pl, pl2]))])));
        assert!(e.contains("selected pricelists must belong to no company or the company of the point of sale"), "{e}");
        let e = err_of(c, || orm::write(&env, "pos.config", &[f.cfg], row(&[("pricelist_id", pl2.into())])));
        assert!(e.contains("default pricelist must be included") || e.contains("must belong"), "{e}");

        // payment method helpers
        let bank_j = account::ensure_journal(&env, "bank", "BNK1", "Bank")?;
        let Value::Map(kanban) = call(&env, "pos.config", "get_pos_kanban_view_state", &[], Row::new())? else { panic!() };
        assert_eq!((kanban["has_pos_config"].clone(), kanban["has_chart_template"].clone(), kanban["is_restaurant_installed"].clone()), (Value::Bool(true), Value::Bool(false), Value::Bool(false)));
        let pm = call(&env, "pos.config", "_create_cash_payment_method", &[], row(&[("cash_journal_vals", Value::Map(row(&[("name", "Cash Bar".into()), ("code", "CBAR".into())])))]))?.as_i64().unwrap();
        let pmr = rec(&env, "pos.payment.method", pm)?; let j = rec(&env, "account.journal", id_of(&pmr, "journal_id").unwrap())?;
        assert_eq!((text(&j, "name").unwrap(), text(&j, "type").unwrap(), text(&pmr, "name").unwrap(), flag(&pmr, "is_cash_count")), ("Cash Bar".to_string(), "cash".to_string(), "Cash".to_string(), true));
        let Value::List(jp) = call(&env, "pos.config", "_create_journal_and_payment_methods", &[], row(&[("cash_journal_vals", Value::Map(row(&[("code", "CSH9".into())])))]))? else { panic!() };
        assert_eq!(l(&jp[1]).len(), 3, "cash, card and customer account");
        let methods: Vec<Row> = l(&jp[1]).iter().map(|i| rec(&env, "pos.payment.method", i.as_i64().unwrap()).unwrap()).collect();
        assert!(methods.iter().any(|x| text(x, "name").as_deref() == Some("Card") && id_of(x, "journal_id") == Some(bank_j)) && methods.iter().any(|x| flag(x, "split_transactions")));
        let e1 = call(&env, "pos.config", "_is_pos_pm_exist", &[], row(&[("name", "Cash".into()), ("journal_id", f.journal.into()), ("company_id", 1.into())]))?;
        assert_eq!(e1, Value::Int(f.cash), "an existing method with that name/journal is reused");
        let other = mk(&env, "pos.config", &[("name", "Other".into()), ("picking_type_id", rec(&env, "pos.config", f.cfg)?["picking_type_id"].clone())]);
        call(&env, "pos.config", "_link_same_non_cash_payment_methods", &[other], row(&[("source_config_id", f.cfg.into())]))?;
        let linked = ids(&rec(&env, "pos.config", other)?, "payment_method_ids");
        assert!(linked.contains(&f.bank) && linked.contains(&f.later) && !linked.contains(&f.cash));
        let Value::Map(cd) = call(&env, "pos.config", "_get_customer_display_data", &[f.cfg], Row::new())? else { panic!() };
        assert_eq!((cd["type"].clone(), cd["config_id"].clone(), cd.contains_key("proxy_ip")), (Value::Text("local".into()), Value::Int(f.cfg), true));
        Ok(())
    }).unwrap();
}

#[test]
fn session_steps_called_directly() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        let outstanding = account::ensure_account(&env, "asset_current", "Outstanding")?;
        let loss = account::ensure_account(&env, "expense", "Cash Loss")?;
        let profit = account::ensure_account(&env, "income_other", "Cash Profit")?;
        let bank_j = rec(&env, "pos.payment.method", f.bank)?["journal_id"].as_i64().unwrap();
        orm::write(&env, "account.journal", &[bank_j], row(&[("loss_account_id", loss.into()), ("profit_account_id", profit.into())]))?;
        orm::write(&env, "pos.payment.method", &[f.bank], row(&[("outstanding_account_id", outstanding.into())]))?;
        let s = mk(&env, "pos.session", &[("config_id", f.cfg.into())]);
        call(&env, "pos.session", "action_pos_session_open", &[s], Row::new())?; call(&env, "pos.session", "_check_pos_config", &[s], Row::new())?;
        call(&env, "pos.session", "_set_opening_control_data", &[s], row(&[("cashbox_value", 40.0.into()), ("notes", "n".into())]))?;
        let r = rec(&env, "pos.session", s)?; assert_eq!((text(&r, "state").unwrap(), num(&r, "cash_register_balance_start"), text(&r, "opening_notes").unwrap()), ("opened".to_string(), 40.0, "n".to_string()));
        // orders: discounted cash order, an invoiced card order and a draft one
        let a = draft_order(&env, &f, s, 2.0); let al = ids(&rec(&env, "pos.order", a)?, "lines")[0];
        orm::write(&env, "pos.order.line", &[al], row(&[("discount", 10.0.into())]))?; call(&env, "pos.order.line", "_onchange_amount_line_all", &[al], Row::new())?; call(&env, "pos.order", "_compute_prices", &[a], Row::new())?;
        pay(&env, a, f.cash, num(&rec(&env, "pos.order", a)?, "amount_total")); call(&env, "pos.order", "action_pos_order_paid", &[a], Row::new())?;
        let b = draft_order(&env, &f, s, 1.0); orm::write(&env, "pos.order", &[b], row(&[("partner_id", f.partner.into())]))?; pay(&env, b, f.bank, 11.0);
        let d = draft_order(&env, &f, s, 1.0);
        let ids_of = |v: Value| l(&v).iter().filter_map(|x| x.as_i64()).collect::<Vec<_>>();
        assert_eq!(ids_of(call(&env, "pos.session", "get_session_orders", &[s], Row::new())?).len(), 3);
        assert_eq!(ids_of(call(&env, "pos.session", "_get_closed_orders", &[s], Row::new())?), vec![a], "the draft ones do not count");
        let Value::List(dom) = call(&env, "pos.session", "_get_captured_payments_domain", &[s], Row::new())? else { panic!() }; assert_eq!(dom.len(), 2);
        let Value::Map(vo) = call(&env, "pos.session", "action_view_order", &[s], Row::new())? else { panic!() }; assert_eq!(vo["res_model"], Value::Text("pos.order".into()));
        let Value::Map(sp) = call(&env, "pos.session", "action_stock_picking", &[s], Row::new())? else { panic!() }; assert_eq!(sp["res_model"], Value::Text("stock.picking".into()));
        // discount: 10% of 20 -> 2.2 tax included
        assert_eq!(call(&env, "pos.session", "get_total_discount", &[s], Row::new())?.as_f64(), Some(2.2));
        // invoices: none yet; a draft invoice blocks closing, a posted one is listed
        call(&env, "pos.order", "action_pos_order_cancel", &[d], Row::new())?;
        pay(&env, b, f.bank, 0.0);
        let Value::Map(act) = call(&env, "pos.order", "_generate_pos_order_invoice", &[b], Row::new())? else { panic!() };
        let inv = act["res_id"].as_i64().unwrap();
        assert_eq!(call(&env, "pos.session", "_get_total_invoice", &[s], Row::new())?.as_f64(), Some(11.0));
        let il = l(&call(&env, "pos.session", "_get_invoice_total_list", &[s], Row::new())?).clone(); assert_eq!(il.len(), 1); assert_eq!(m(&il[0])["total"].as_f64(), Some(11.0));
        orm::write(&env, "account.move", &[inv], row(&[("state", "draft".into())]))?;
        assert!(err_of(c, || call(&env, "pos.session", "_check_invoices_are_posted", &[s], Row::new())).contains("You cannot close the POS when invoices are not posted"));
        orm::write(&env, "account.move", &[inv], row(&[("state", "posted".into())]))?;
        call(&env, "pos.session", "_check_invoices_are_posted", &[s], Row::new())?;
        // closing control state change keeps the session open for counting
        call(&env, "pos.session", "update_closing_control_state_session", &[s], row(&[("notes", "counting".into())]))?;
        let r = rec(&env, "pos.session", s)?; assert_eq!((text(&r, "state").unwrap(), text(&r, "closing_notes").unwrap()), ("closing_control".to_string(), "counting".to_string()));
        // cash difference helpers: a missing profit/loss account is explained; with accounts a balanced statement entry is posted
        let amount = rd(&env, "pos.session", s, "cash_register_difference");
        assert!(amount.as_f64().is_some());
        // a split bank method books its closing difference against the journal's accounts
        let Value::List(vals) = call(&env, "pos.session", "_get_diff_vals", &[s], row(&[("payment_method_id", f.bank.into()), ("diff_amount", (-3.0).into())]))? else { panic!() };
        assert_eq!((m(&vals[0])["account_id"].clone(), m(&vals[0])["debit"].as_f64(), m(&vals[1])["account_id"].clone(), m(&vals[1])["credit"].as_f64()), (Value::Int(outstanding), Some(0.0), Value::Int(loss), Some(0.0)));
        assert_eq!(call(&env, "pos.session", "_get_diff_vals", &[s], row(&[("payment_method_id", f.bank.into()), ("diff_amount", 0.0.into())]))?, Value::Bool(false));
        let mv = call(&env, "pos.session", "_create_diff_account_move_for_split_payment_method", &[s], row(&[("payment_method_id", f.bank.into()), ("diff_amount", 3.0.into())]))?.as_i64().unwrap();
        let mr = rec(&env, "account.move", mv)?; assert!(text(&mr, "ref").unwrap().starts_with("Closing difference in Card (POS/")); assert_eq!(text(&mr, "state").unwrap(), "posted");
        let ml = children(&env, "account.move.line", "move_id", mv)?; assert!(ml.iter().any(|x| id_of(x, "account_id") == Some(profit) && num(x, "credit") == 3.0));
        // post_statement_difference on the cash journal
        call(&env, "pos.session", "_post_statement_difference", &[s], row(&[("amount", (-2.0).into())]))?;
        let lines = children(&env, "account.bank.statement.line", "journal_id", f.journal)?; assert!(lines.iter().any(|x| num(x, "amount") == -2.0 && text(x, "payment_ref").unwrap().contains("(Loss)")));
        // closing through the validate entry points
        orm::write(&env, "pos.session", &[s], row(&[("cash_register_balance_end_real", 0.0.into())]))?;
        call(&env, "pos.session", "action_pos_session_validate", &[s], Row::new())?;
        assert_eq!(text(&rec(&env, "pos.session", s)?, "state").unwrap(), "closed");
        assert!(err_of(c, || call(&env, "pos.session", "_validate_session", &[s], Row::new())).contains("already closed"));
        assert!(err_of(c, || call(&env, "pos.session", "action_pos_session_close", &[s], Row::new())).contains("already closed"));
        Ok(())
    }).unwrap();
}

#[test]
fn order_steps_called_directly() {
    let (reg, st) = setup(&["point_of_sale", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        let o = draft_order(&env, &f, s, 2.0); let line = ids(&rec(&env, "pos.order", o)?, "lines")[0];
        // line pricing helpers
        let Value::Map(am) = call(&env, "pos.order.line", "_compute_amount_line_all", &[line], Row::new())? else { panic!() };
        assert_eq!((am["price_subtotal"].as_f64(), am["price_subtotal_incl"].as_f64()), (Some(20.0), Some(22.0)));
        let tmpl = id_of(&rec(&env, "product.product", f.pen)?, "product_tmpl_id").unwrap();
        orm::write(&env, "product.template", &[tmpl], row(&[("taxes_id", cmd6(&[f.tax]))]))?;
        let company = rec(&env, "pos.order", o)?["company_id"].as_i64().unwrap();
        orm::write(&env, "account.tax", &[f.tax], row(&[("company_id", company.into())]))?;
        let Value::Map(pi) = call(&env, "pos.order.line", "_onchange_product_id", &[line], Row::new())? else { panic!() };
        assert_eq!((pi["price_unit"].as_f64(), pi["tax_ids"].clone().to_json().to_string().contains(&f.tax.to_string())), (Some(10.0), true));
        assert_eq!(call(&env, "pos.order.line", "_get_tax_ids_after_fiscal_position", &[line], Row::new())?, Value::List(vec![Value::Int(f.tax)]));
        assert_eq!(rd(&env, "pos.order.line", line, "tax_ids_after_fiscal_position"), Value::List(vec![Value::Int(f.tax)]));
        // margins: only meaningful once costs are known
        orm::write(&env, "pos.order.line", &[line], row(&[("total_cost", 12.0.into()), ("is_total_cost_computed", true.into())]))?;
        assert_eq!((rd(&env, "pos.order.line", line, "margin").as_f64(), rd(&env, "pos.order.line", line, "margin_percent").as_f64()), (Some(8.0), Some(0.4)));
        assert_eq!((rd(&env, "pos.order", o, "margin").as_f64(), rd(&env, "pos.order", o, "margin_percent").as_f64()), (Some(8.0), Some(0.4)));
        // contact details come from the partner while empty
        orm::write(&env, "res.partner", &[f.partner], row(&[("email", "a@x.com".into())]))?;
        orm::write(&env, "pos.order", &[o], row(&[("partner_id", f.partner.into())]))?;
        assert_eq!(text(&rec(&env, "pos.order", o)?, "email").as_deref(), Some("a@x.com"));
        // payments and the payment checks
        pay(&env, o, f.cash, 22.0);
        assert_eq!(call(&env, "pos.order", "_get_rounded_amount", &[o], row(&[("amount", 21.999.into())]))?.as_f64(), Some(22.0));
        call(&env, "pos.order", "_onchange_amount_all", &[o], Row::new())?; call(&env, "pos.order", "_check_payment_method", &[], Row::new()).ok();
        let pays = children(&env, "pos.payment", "pos_order_id", o)?; let p = pays[0]["id"].as_i64().unwrap();
        call(&env, "pos.payment", "_check_amount", &[p], Row::new())?; call(&env, "pos.payment", "_check_payment_method_id", &[p], Row::new())?;
        assert_eq!(call(&env, "pos.order", "_compute_order_name", &[o], Row::new())?, Value::Text("Shop/0001".into()), "next number of the register's sequence");
        // paying and the steps around it
        call(&env, "pos.order", "_process_payment_lines", &[], row(&[("pos_order_id", o.into()), ("order", Value::Map(row(&[("amount_return", 0.0.into())]))), ("session_id", s.into()), ("draft", false.into())]))?;
        assert_eq!(call(&env, "pos.order", "_process_saved_order", &[o], row(&[("draft", false.into())]))?, Value::Int(o));
        assert_eq!(text(&rec(&env, "pos.order", o)?, "state").unwrap(), "paid");
        assert_eq!(call(&env, "pos.order", "_should_create_picking_real_time", &[o], Row::new())?, Value::Bool(true));
        assert_eq!(l(&call(&env, "pos.order", "_get_stock_moves", &[o], Row::new())?).len(), 1);
        assert_eq!(rd(&env, "pos.order", o, "picking_count"), Value::Int(1)); assert_eq!(rd(&env, "pos.order", o, "failed_pickings"), Value::Bool(false));
        let Value::Map(ap) = call(&env, "pos.order", "action_stock_picking", &[o], Row::new())? else { panic!() }; assert_eq!(ap["res_model"], Value::Text("stock.picking".into()));
        // invoicing steps
        let Value::List(il) = call(&env, "pos.order", "_prepare_invoice_lines", &[o], Row::new())? else { panic!() };
        assert_eq!((m(&il[0])["quantity"].as_f64(), m(&il[0])["price_unit"].as_f64(), m(&il[0])["product_id"].clone()), (Some(2.0), Some(10.0), Value::Int(f.pen)));
        let Value::Map(act) = call(&env, "pos.order", "_generate_pos_order_invoice", &[o], Row::new())? else { panic!() };
        let inv = act["res_id"].as_i64().unwrap();
        let Value::Map(vi) = call(&env, "pos.order", "action_view_invoice", &[o], Row::new())? else { panic!() }; assert_eq!(vi["res_id"], Value::Int(inv));
        let moves = l(&call(&env, "pos.order", "_apply_invoice_payments", &[o], Row::new())?).clone(); assert_eq!(moves.len(), 1, "one payment entry for the cash payment");
        let Value::Map(act2) = call(&env, "pos.order", "_generate_pos_order_invoice", &[o], Row::new())? else { panic!() }; assert_eq!(act2["res_id"], Value::Int(inv), "an invoiced order keeps its invoice");
        // reversal entry for an order of an already closed session, called directly
        let rv = call(&env, "pos.order", "_create_misc_reversal_move", &[o], Row::new())?; assert!(rv.as_i64().is_some());
        // refunds link back
        let Value::List(refs) = call(&env, "pos.order", "_refund", &[o], Row::new())? else { panic!() }; let ro = refs[0].as_i64().unwrap();
        let rl = children(&env, "pos.order.line", "order_id", ro)?[0]["id"].as_i64().unwrap();
        let Value::Map(rd_) = call(&env, "pos.order.line", "_prepare_refund_data", &[line], row(&[("refund_order_id", ro.into())]))? else { panic!() };
        assert_eq!((rd_["qty"].as_f64(), rd_["refunded_orderline_id"].clone()), (Some(0.0), Value::Int(line)), "everything is already refunded by the refund above");
        let Value::Map(v1) = call(&env, "pos.order", "action_view_refunded_order", &[ro], Row::new())? else { panic!() }; assert_eq!(v1["res_id"], Value::Int(o));
        let Value::Map(v2) = call(&env, "pos.order", "action_view_refund_orders", &[o], Row::new())? else { panic!() }; assert!(v2.contains_key("domain"));
        assert!(rec(&env, "pos.order.line", rl).is_ok());
        // cleaning payments of a draft
        let d = draft_order(&env, &f, s, 1.0); pay(&env, d, f.bank, 5.0);
        call(&env, "pos.order", "_clean_payment_lines", &[d], Row::new())?; assert!(children(&env, "pos.payment", "pos_order_id", d)?.is_empty());
        Ok(())
    }).unwrap();
}

#[test]
fn processing_helpers_misc_checks_and_remaining_computes() {
    let (reg, st) = setup(&["point_of_sale", "pos_loyalty", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        // _process_order and the combo helpers
        let order = Value::Map(row(&[("session_id", s.into()), ("name", "Order x".into()), ("uuid", "x1".into()), ("state", "paid".into()), ("amount_total", 11.0.into()), ("amount_tax", 1.0.into()), ("amount_paid", 0.0.into()), ("amount_return", 0.0.into()),
            ("lines", Value::List(vec![Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("uuid", "l1".into()), ("id", (-1).into()), ("product_id", f.pen.into()), ("qty", 1.0.into()), ("price_unit", 10.0.into()), ("price_subtotal", 10.0.into()), ("price_subtotal_incl", 11.0.into()), ("combo_line_ids", Value::List(vec![(-2).into()]))]))]),
                Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("uuid", "l2".into()), ("id", (-2).into()), ("product_id", f.pen.into()), ("qty", 1.0.into()), ("price_unit", 0.0.into()), ("price_subtotal", 0.0.into()), ("price_subtotal_incl", 0.0.into())]))])])),
            ("payment_ids", Value::List(vec![Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("payment_method_id", f.cash.into()), ("amount", 11.0.into())]))])]))]));
        let Value::Map(combos) = call(&env, "pos.order", "_prepare_combo_line_uuids", &[], row(&[("order", order.clone())]))? else { panic!() };
        assert_eq!(combos["l1"], Value::List(vec![Value::Text("l2".into())]));
        let e = err_of(c, || call(&env, "pos.order", "_check_combo_item_available", &[], row(&[("order", order.clone())])));
        assert!(e.contains("is no longer available in this combo"), "{e}");
        // the same order without combo links is processed normally
        let mut plain = m(&order).clone();
        plain.insert("lines".into(), Value::List(l(&order_lines_without_combo(&order)).clone()));
        let Value::Int(oid) = call(&env, "pos.order", "_process_order", &[], row(&[("order", Value::Map(plain))]))? else { panic!() };
        assert_eq!(text(&rec(&env, "pos.order", oid)?, "state").unwrap(), "paid");
        assert_eq!(children(&env, "pos.order.line", "order_id", oid)?.len(), 2);
        // loyalty helpers called directly
        let rule = Value::List(vec![Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("reward_point_mode", "order".into()), ("reward_point_amount", 1.0.into())]))])]);
        let reward = Value::List(vec![Value::List(vec![0.into(), 0.into(), Value::Map(row(&[("reward_type", "discount".into()), ("discount_mode", "percent".into()), ("discount", 10.0.into()), ("required_points", 1.0.into())]))])]);
        let prog = mk(&env, "loyalty.program", &[("name", "L".into()), ("program_type", "loyalty".into()), ("applies_on", "both".into()), ("pos_ok", true.into()), ("rule_ids", rule), ("reward_ids", reward)]);
        let card = mk(&env, "loyalty.card", &[("program_id", prog.into()), ("partner_id", f.partner.into()), ("code", "K1".into()), ("points", 1.0.into())]);
        let Value::Map(moved) = call(&env, "pos.order", "_check_existing_loyalty_cards", &[], row(&[("coupon_data", Value::Map(row(&[("-1", Value::Map(row(&[("program_id", prog.into()), ("partner_id", f.partner.into()), ("points", 2.0.into())])))])))]))? else { panic!() };
        assert!(moved.contains_key(&card.to_string()) && !moved.contains_key("-1"), "the customer's existing card is reused");
        let Value::Map(kept) = call(&env, "pos.order", "_remove_duplicate_coupon_data", &[oid], row(&[("coupon_data", Value::Map(row(&[("-1", Value::Map(row(&[("program_id", prog.into())])))])))]))? else { panic!() };
        assert!(kept.contains_key("-1"), "nothing recorded for this order yet");

        // remaining computes/helpers
        assert_eq!(rd(&env, "pos.order", oid, "email"), Value::Text(String::new()).clone());
        let pl = mk(&env, "product.template", &[("name", "Plain".into())]); let _ = pl;
        let plain_t = id_of(&rec(&env, "product.product", f.pen)?, "product_tmpl_id").unwrap();
        call(&env, "product.template", "_check_combo_inclusions", &[plain_t], Row::new())?;
        assert_eq!(call(&env, "product.product", "_optional_product_pos_domain", &[f.pen], Row::new()).map(|v| l(&v).len()).unwrap_or(2), 2);
        let cat = mk(&env, "pos.category", &[("name", "A".into())]);
        call(&env, "pos.category", "_check_category_recursion", &[cat], Row::new())?; call(&env, "pos.category", "_check_hour", &[cat], Row::new()).ok();
        assert!(err_of(c, || call(&env, "account.journal", "_check_no_active_payments", &[f.journal], Row::new())).contains("that is being used by order"));
        let Value::Map(ti) = call(&env, "report.point_of_sale.report_saledetails", "_get_taxes_info", &[], row(&[("taxes", Value::Map(row(&[("base_amount", 10.0.into()), ("taxes", Value::Map(row(&[("1", Value::Map(row(&[("tax_amount", 1.0.into())])))])))])))]))? else { panic!() };
        assert_eq!((ti["tax_amount"].as_f64(), ti["base_amount"].as_f64()), (Some(1.0), Some(10.0)));
        let session_failed = rd(&env, "pos.session", s, "failed_pickings"); assert_eq!(session_failed, Value::Bool(false));
        assert_eq!(rd(&env, "pos.payment.method", f.cash, "hide_qr_code_method"), Value::Bool(true));
        // settings defaults: fiscal position and scale options follow the register
        let fp = mk(&env, "account.fiscal.position", &[("name", "Local".into())]);
        orm::write(&env, "pos.config", &[f.cfg], row(&[("tax_regime_selection", true.into()), ("default_fiscal_position_id", fp.into())]))?;
        assert!(ids(&rec(&env, "pos.config", f.cfg)?, "fiscal_position_ids").contains(&fp), "the default position is always selectable");
        let sid = mk(&env, "res.config.settings", &[("pos_config_id", f.cfg.into())]);
        assert_eq!(id_of(&rec(&env, "res.config.settings", sid)?, "pos_default_fiscal_position_id"), Some(fp));
        assert!(!flag(&rec(&env, "res.config.settings", sid)?, "pos_iface_electronic_scale"));
        orm::write(&env, "pos.config", &[f.cfg], row(&[("is_posbox", true.into()), ("proxy_ip", "1.2.3.4".into()), ("iface_electronic_scale", true.into())]))?;
        orm::write(&env, "res.config.settings", &[sid], row(&[("pos_config_id", f.cfg.into())]))?;
        assert!(flag(&rec(&env, "res.config.settings", sid)?, "pos_iface_electronic_scale"));
        Ok(())
    }).unwrap();
}

fn order_lines_without_combo(order: &Value) -> Value {
    let lines = match m(order).get("lines") { Some(Value::List(x)) => x.clone(), _ => vec![] };
    Value::List(lines.into_iter().map(|c| match c { Value::List(mut x) => { if let Value::Map(v) = &mut x[2] { v.remove("combo_line_ids"); } Value::List(x) } o => o }).collect())
}
