//! pos_loyalty server methods (pos_loyalty_methods).
mod pos_common;
use odoo_core::{orm, store, Row, Value};
use odoo_modules::{rules_for, util::*};
use pos_common::*;

fn sub(model_vals: &[(&str, Value)]) -> Value { Value::List(vec![Value::List(vec![0.into(), 0.into(), Value::Map(row(model_vals))])]) }
fn program(env: &odoo_core::orm::Env, name: &str, ptype: &str, extra: &[(&str, Value)], rule: &[(&str, Value)], reward: &[(&str, Value)]) -> i64 {
    let mut v: Vec<(&str, Value)> = vec![("name", name.into()), ("program_type", ptype.into()), ("applies_on", "future".into()), ("pos_ok", true.into()), ("rule_ids", sub(rule)), ("reward_ids", sub(reward))];
    v.extend(extra.iter().map(|(k, x)| (*k, x.clone())));
    mk(env, "loyalty.program", &v)
}

#[test]
fn programs_rules_checks_and_coupon_codes() {
    let (reg, st) = setup(&["point_of_sale", "pos_loyalty", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env);
        let other_cfg = mk(&env, "pos.config", &[("name", "Other".into()), ("picking_type_id", rec(&env, "pos.config", f.cfg)?["picking_type_id"].clone()), ("payment_method_ids", cmd6(&[f.bank]))]);
        let rule = [("reward_point_mode", "order".into()), ("reward_point_amount", 1.0.into())];
        let disc = [("reward_type", "discount".into()), ("discount_mode", "percent".into()), ("discount", 10.0.into()), ("required_points", 2.0.into()), ("discount_applicability", "order".into())];
        let everywhere = program(&env, "Everywhere", "loyalty", &[("applies_on", "both".into())], &rule, &disc);
        let only_other = program(&env, "Other only", "loyalty", &[("pos_config_ids", cmd6(&[other_cfg]))], &rule, &disc);
        let expired = program(&env, "Expired", "promotion", &[("date_to", "2000-01-01".into())], &rule, &disc);
        let limited = program(&env, "Limited", "promotion", &[("limit_usage", true.into()), ("max_usage", 1.into())], &rule, &disc);
        let Value::List(av) = call(&env, "pos.config", "_get_program_ids", &[f.cfg], Row::new())? else { panic!() };
        let got: std::collections::BTreeSet<i64> = av.iter().filter_map(|v| v.as_i64()).collect();
        assert_eq!(got, [everywhere, limited].into_iter().collect(), "restricted to other register / expired programs are left out");
        // a POS order using a reward of the limited program uses its quota up
        let reward = ids(&rec(&env, "loyalty.program", limited)?, "reward_ids")[0];
        assert_eq!(rd(&env, "loyalty.program", limited, "pos_order_count"), Value::Int(0));
        let s = opened_session(&env, &f, 0.0); let o = draft_order(&env, &f, s, 1.0);
        mk(&env, "pos.order.line", &[("order_id", o.into()), ("product_id", f.pen.into()), ("qty", 1.0.into()), ("price_unit", (-1.0).into()), ("price_subtotal", (-1.0).into()), ("price_subtotal_incl", (-1.0).into()), ("is_reward_line", true.into()), ("reward_id", reward.into())]);
        assert_eq!(rd(&env, "loyalty.program", limited, "pos_order_count"), Value::Int(1));
        assert_eq!(rd(&env, "loyalty.program", limited, "total_order_count"), Value::Int(1));
        let Value::List(av) = call(&env, "pos.config", "_get_program_ids", &[f.cfg], Row::new())? else { panic!() };
        assert_eq!(av.iter().filter_map(|v| v.as_i64()).collect::<Vec<_>>(), vec![everywhere]);
        // reward lines do not count as refund lines
        let rl = children(&env, "pos.order.line", "order_id", o)?.into_iter().find(|l| flag(l, "is_reward_line")).unwrap();
        assert_eq!(call(&env, "pos.order.line", "isRefund", &[rl["id"].as_i64().unwrap()], Row::new())?, Value::Bool(false));

        // rules: unrestricted rules match any product; restricted ones list their available products; the barcode is generated once
        let rid = ids(&rec(&env, "loyalty.program", everywhere)?, "rule_ids")[0];
        assert_eq!(rd(&env, "loyalty.rule", rid, "any_product"), Value::Bool(true));
        assert_eq!(rd(&env, "loyalty.program", everywhere, "is_nominative"), Value::Bool(true));
        assert_eq!(rd(&env, "loyalty.program", limited, "is_nominative"), Value::Bool(false));
        let barcode = text(&rec(&env, "loyalty.rule", rid)?, "promo_barcode").unwrap();
        assert!(barcode.starts_with("044") && barcode.len() == 14, "{barcode}");
        orm::write(&env, "loyalty.rule", &[rid], row(&[("minimum_qty", 2.into())]))?;
        assert_eq!(text(&rec(&env, "loyalty.rule", rid)?, "promo_barcode").unwrap(), barcode);
        let hidden = mk(&env, "product.product", &[("name", "Hidden".into()), ("list_price", 1.0.into()), ("type", "consu".into())]);
        orm::write(&env, "loyalty.rule", &[rid], row(&[("product_ids", cmd6(&[f.pen, hidden]))]))?;
        assert_eq!(rd(&env, "loyalty.rule", rid, "any_product"), Value::Bool(false));
        assert_eq!(call(&env, "loyalty.rule", "_get_valid_product_ids", &[rid], Row::new())?, Value::List(vec![Value::Int(f.pen)]), "only products available in the POS");

        // reward products must be sellable before a session opens
        let free = mk(&env, "product.product", &[("name", "Free gift".into()), ("list_price", 3.0.into()), ("type", "consu".into())]);
        let gift = program(&env, "Gift", "promotion", &[], &rule, &[("reward_type", "product".into()), ("reward_product_id", free.into()), ("reward_product_ids", cmd6(&[free])), ("required_points", 1.0.into()), ("reward_product_qty", 1.0.into())]);
        let e = err_of(c, || call(&env, "pos.config", "_check_loyalty_programs", &[f.cfg], Row::new()));
        assert!(e.contains("make the following reward products available in Point of Sale") && e.contains("Program: Gift, Reward Product: `Free gift`"), "{e}");
        let ft = id_of(&rec(&env, "product.product", free)?, "product_tmpl_id").unwrap();
        orm::write(&env, "product.template", &[ft], row(&[("available_in_pos", true.into())]))?;
        call(&env, "pos.config", "_check_loyalty_programs", &[f.cfg], Row::new())?;
        orm::write(&env, "loyalty.program", &[gift], row(&[("active", false.into())]))?;
        // gift card programs follow strict shapes
        let bad_gc = program(&env, "GC bad", "gift_card", &[], &[("reward_point_mode", "unit".into()), ("reward_point_amount", 1.0.into())], &[("reward_type", "discount".into()), ("discount_mode", "per_point".into()), ("discount", 1.0.into()), ("required_points", 1.0.into())]);
        assert!(err_of(c, || call(&env, "pos.config", "_check_loyalty_programs", &[f.cfg], Row::new())).contains("Use 1 point per currency spent"));
        orm::write(&env, "loyalty.program", &[bad_gc], row(&[("rule_ids", sub(&[("reward_point_mode", "money".into()), ("reward_point_amount", 1.0.into())]))]))?;
        assert!(err_of(c, || call(&env, "pos.config", "_check_loyalty_programs", &[f.cfg], Row::new())).contains("More than one rule"));
        orm::write(&env, "loyalty.program", &[bad_gc], row(&[("active", false.into())]))?;

        // coupon codes
        let ann = mk(&env, "res.partner", &[("name", "Ann".into())]);
        let card = mk(&env, "loyalty.card", &[("program_id", everywhere.into()), ("partner_id", ann.into()), ("code", "ANN-1".into()), ("points", 5.0.into())]);
        let payload = |v: Value| match v { Value::Map(m) => m, _ => panic!() };
        let bad = payload(call(&env, "pos.config", "use_coupon_code", &[f.cfg], row(&[("code", "NOPE".into()), ("creation_date", "2026-01-01 10:00:00".into()), ("partner_id", ann.into())]))?);
        assert_eq!(bad["successful"], Value::Bool(false)); assert!(matches!(&bad["payload"], Value::Map(p) if p["error_message"] == Value::Text("This coupon is invalid (NOPE).".into())));
        // someone else's nominative card is invalid for another customer
        let stranger = payload(call(&env, "pos.config", "use_coupon_code", &[f.cfg], row(&[("code", "ANN-1".into()), ("creation_date", "2026-01-01 10:00:00".into())]))?);
        assert_eq!(stranger["successful"], Value::Bool(false));
        let ok = payload(call(&env, "pos.config", "use_coupon_code", &[f.cfg], row(&[("code", "ANN-1".into()), ("creation_date", "2026-01-01 10:00:00".into()), ("partner_id", ann.into())]))?);
        assert_eq!(ok["successful"], Value::Bool(true));
        let Value::Map(p) = &ok["payload"] else { panic!() }; assert_eq!((p["coupon_id"].clone(), p["program_id"].clone(), p["points"].as_f64(), p["has_source_order"].clone()), (Value::Int(card), Value::Int(everywhere), Some(5.0), Value::Bool(false)));
        orm::write(&env, "loyalty.card", &[card], row(&[("expiration_date", "2020-01-01".into())]))?;
        let old = payload(call(&env, "pos.config", "use_coupon_code", &[f.cfg], row(&[("code", "ANN-1".into()), ("creation_date", "2026-01-01 10:00:00".into()), ("partner_id", ann.into())]))?);
        assert!(matches!(&old["payload"], Value::Map(p) if p["error_message"] == Value::Text("This coupon is expired (ANN-1).".into())));
        orm::write(&env, "loyalty.card", &[card], row(&[("expiration_date", Value::Bool(false)), ("points", 1.0.into())]))?;
        let poor = payload(call(&env, "pos.config", "use_coupon_code", &[f.cfg], row(&[("code", "ANN-1".into()), ("creation_date", "2026-01-01 10:00:00".into()), ("partner_id", ann.into())]))?);
        assert!(matches!(&poor["payload"], Value::Map(p) if p["error_message"] == Value::Text("No reward can be claimed with this coupon.".into())));
        Ok(())
    }).unwrap();
}

#[test]
fn coupons_are_validated_and_confirmed_with_history() {
    let (reg, st) = setup(&["point_of_sale", "pos_loyalty", "account"]); let rules = rules_for(&reg);
    store::run(&st, |c| {
        let env = new_env(&reg, c, &rules); let f = fixture(&env); let s = opened_session(&env, &f, 0.0);
        let rule = [("reward_point_mode", "order".into()), ("reward_point_amount", 1.0.into())];
        let disc = [("reward_type", "discount".into()), ("discount_mode", "percent".into()), ("discount", 10.0.into()), ("required_points", 2.0.into()), ("discount_applicability", "order".into())];
        let loyal = program(&env, "Loyalty", "loyalty", &[("applies_on", "both".into())], &rule, &disc);
        let promo = program(&env, "Next order", "next_order_coupons", &[], &rule, &disc);
        let ann = f.partner;
        let card = mk(&env, "loyalty.card", &[("program_id", loyal.into()), ("partner_id", ann.into()), ("code", "C-ANN".into()), ("points", 5.0.into())]);
        // validation: spending more than the balance, unknown cards and already sold codes are refused
        let v = |changes: Row, codes: Vec<&str>| -> odoo_core::Result<Row> { match call(&env, "pos.order", "validate_coupon_programs", &[], row(&[("point_changes", Value::Map(changes)), ("new_codes", Value::List(codes.into_iter().map(|c| Value::Text(c.into())).collect()))]))? { Value::Map(m) => Ok(m), _ => panic!() } };
        assert_eq!(v(row(&[(&card.to_string(), (-2.0).into())]), vec!["NEW"])?["successful"], Value::Bool(true));
        let r = v(row(&[(&card.to_string(), (-9.0).into())]), vec![])?;
        assert_eq!(r["successful"], Value::Bool(false));
        let Value::Map(p) = &r["payload"] else { panic!() }; assert_eq!(p["message"], Value::Text("There are not enough points for the coupon: C-ANN.".into()));
        let r = v(row(&[("999", (-1.0).into())]), vec![])?; let Value::Map(p) = &r["payload"] else { panic!() }; assert_eq!(p["removed_coupons"], Value::List(vec![Value::Int(999)]));
        let r = v(row(&[]), vec!["C-ANN"])?; let Value::Map(p) = &r["payload"] else { panic!() }; assert!(matches!(&p["message"], Value::Text(m) if m.contains("already exist in the database") && m.contains("C-ANN")));

        // confirmation: the customer's card gets points added and the order's history; a new next-order coupon is minted
        let o = draft_order(&env, &f, s, 1.0);
        orm::write(&env, "pos.order", &[o], row(&[("partner_id", ann.into())]))?;
        let data = Value::Map(row(&[
            ("-1", Value::Map(row(&[("points", 3.0.into()), ("program_id", loyal.into()), ("partner_id", ann.into())]))),     // same customer + program: reuses the existing card
            ("-2", Value::Map(row(&[("points", 1.0.into()), ("program_id", promo.into()), ("code", "NEXT-1".into()), ("expiration_date", "2030-01-01".into())]))),
        ]));
        let Value::Map(res) = call(&env, "pos.order", "confirm_coupon_programs", &[o], row(&[("coupon_data", data.clone())]))? else { panic!() };
        assert_eq!(num(&rec(&env, "loyalty.card", card)?, "points"), 8.0);
        let minted = find_one(&env, "loyalty.card", term("code", "=", "NEXT-1"))?.expect("a new coupon");
        let m = rec(&env, "loyalty.card", minted)?;
        assert_eq!((num(&m, "points"), id_of(&m, "source_pos_order_id"), text(&m, "expiration_date").as_deref()), (1.0, Some(o), Some("2030-01-01")));
        let hist = children(&env, "loyalty.history", "order_id", o)?;
        assert_eq!(hist.len(), 2);
        let h = hist.iter().find(|h| id_of(h, "card_id") == Some(card)).unwrap();
        assert_eq!((num(h, "issued"), num(h, "used"), text(h, "order_model").as_deref()), (3.0, 0.0, Some("pos.order")));
        assert!(text(h, "description").unwrap().starts_with("Onsite "));
        // coupon updates only list nominative programs; new coupon info lists future-order coupons
        let Value::List(cu) = &res["coupon_updates"] else { panic!() }; assert_eq!(cu.len(), 1);
        assert!(matches!(&cu[0], Value::Map(m) if m["id"] == Value::Int(card) && m["points"].as_f64() == Some(8.0) && m["old_id"] == Value::Int(card)));
        let Value::List(info) = &res["new_coupon_info"] else { panic!() };
        assert!(matches!(&info[0], Value::Map(m) if m["code"] == Value::Text("NEXT-1".into()) && m["program_name"] == Value::Text("Next order".into())));
        let Value::List(pu) = &res["program_updates"] else { panic!() }; assert_eq!(pu.len(), 2);
        // confirming the same order again adds nothing
        call(&env, "pos.order", "confirm_coupon_programs", &[o], row(&[("coupon_data", data)]))?;
        assert_eq!(num(&rec(&env, "loyalty.card", card)?, "points"), 8.0);
        assert_eq!(children(&env, "loyalty.history", "order_id", o)?.len(), 2);
        // history lines can also be added directly, skipping unknown cards
        call(&env, "pos.order", "add_loyalty_history_lines", &[o], row(&[("coupon_data", Value::List(vec![
            Value::Map(row(&[("card_id", card.into()), ("won", 0.0.into()), ("spent", 2.0.into())])), Value::Map(row(&[("card_id", 12345.into()), ("won", 1.0.into()), ("spent", 0.0.into())]))])), ("coupon_updates", Value::List(vec![]))]))?;
        assert_eq!(children(&env, "loyalty.history", "order_id", o)?.len(), 3);
        Ok(())
    }).unwrap();
}
