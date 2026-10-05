//! odoo-modules: ported business rules per Odoo module, assembled against the installed registry.
pub mod account;
pub mod bootstrap;
pub mod crm;
pub mod hr;
pub mod project;
pub mod base;
pub mod data;
pub mod pos;
pub mod pos_hw;
pub mod pos_kiosk;
pub mod pos_loyalty;
pub mod pos_loyalty_methods;
pub mod pos_methods;
pub mod pos_misc_methods;
pub mod pos_order_methods;
pub mod pos_session_methods;
pub mod pos_post;
pub mod pos_report_methods;
pub mod pos_restaurant;
pub mod pos_sale_methods;
pub mod pos_shop;
pub mod pos_ubl;
pub mod purchase;
pub mod sale;
pub mod stock;
pub mod tax;
pub mod util;

use odoo_core::{Registry, Rules};

/// Compose the rule set for exactly the models present in `reg` (mirrors Odoo's per-module installation).
pub fn rules_for(reg: &Registry) -> Rules {
    let has = |m: &str| reg.models.contains_key(m);
    let mut r = Rules::default();
    if has("account.move") { r = r.merge(account::rules()); }
    if has("sale.order") { r = r.merge(sale::rules()); }
    if has("stock.picking") { r = r.merge(stock::rules()); }
    if has("purchase.order") { r = r.merge(purchase::rules()); }
    if has("res.country") { r = r.merge(base::country_rules()); }
    if has("res.partner") { r = r.merge(base::partner_rules()); }
    if has("res.users") && has("ir.model.data") { r = r.merge(base::rules()); }
    if has("pos.payment.method") { r = r.merge(pos::rules()); }
    if has("pos.session") && has("pos.order") { r = r.merge(pos_methods::rules()); }
    if has("hr.leave") { r = r.merge(hr::rules()); }
    if has("project.task") { r = r.merge(project::rules()); }
    if has("crm.lead") { r = r.merge(crm::rules()); }
    r
}
