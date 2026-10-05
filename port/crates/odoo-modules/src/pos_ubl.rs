//! UBL 2.1 (Peppol BIS Billing 3.0 shaped) e-invoice for an invoiced POS order. Amounts come from the stored order lines, so the
//! document always agrees with what the customer paid; tax rates are derived per line from the stored subtotal and total.
use crate::util::*;
use odoo_core::orm::Env;
use odoo_core::{OdooError, Result, Rules, Value};
use std::collections::BTreeMap;
use std::fmt::Write;

fn esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }
fn money(x: f64) -> String { format!("{:.2}", (x * 100.0).round() / 100.0) }
fn party(out: &mut String, tag: &str, name: &str, vat: &str, street: &str, city: &str, zip: &str, country: &str) {
    let _ = write!(out, "<cac:{tag}><cac:Party><cac:PartyName><cbc:Name>{}</cbc:Name></cac:PartyName><cac:PostalAddress><cbc:StreetName>{}</cbc:StreetName><cbc:CityName>{}</cbc:CityName><cbc:PostalZone>{}</cbc:PostalZone><cac:Country><cbc:IdentificationCode>{}</cbc:IdentificationCode></cac:Country></cac:PostalAddress>",
        esc(name), esc(street), esc(city), esc(zip), esc(if country.is_empty() { "ZZ" } else { country }));
    if !vat.is_empty() { let _ = write!(out, "<cac:PartyTaxScheme><cbc:CompanyID>{}</cbc:CompanyID><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:PartyTaxScheme>", esc(vat)); }
    let _ = write!(out, "<cac:PartyLegalEntity><cbc:RegistrationName>{}</cbc:RegistrationName></cac:PartyLegalEntity></cac:Party></cac:{tag}>", esc(name));
}

pub fn ubl(env: &Env, order: i64) -> Result<String> {
    let o = rec(env, "pos.order", order)?;
    if !matches!(text(&o, "state").as_deref(), Some("invoiced" | "paid" | "done")) { return Err(OdooError::User("Only paid orders can be exported".into())); }
    let partner = id_of(&o, "partner_id").map(|p| rec(env, "res.partner", p)).transpose()?.ok_or_else(|| OdooError::User("An e-invoice needs a customer".into()))?;
    let company = id_of(&o, "company_id").and_then(|c| rec(env, "res.company", c).ok()).or_else(|| find_one(env, "res.company", odoo_core::Domain::True).ok().flatten().and_then(|c| rec(env, "res.company", c).ok())).unwrap_or_default();
    let cur = id_of(&company, "currency_id").and_then(|c| rec(env, "res.currency", c).ok()).and_then(|c| text(&c, "name")).unwrap_or_else(|| "USD".into());
    let country = |r: &odoo_core::Row| id_of(r, "country_id").and_then(|c| rec(env, "res.country", c).ok()).and_then(|c| text(&c, "code")).unwrap_or_default();
    let credit = num(&o, "amount_total") < 0.0; let sign = if credit { -1.0 } else { 1.0 };
    let (root, code, line_tag, qty_tag) = if credit { ("CreditNote", "381", "CreditNoteLine", "CreditedQuantity") } else { ("Invoice", "380", "InvoiceLine", "InvoicedQuantity") };
    let mut x = String::new();
    let _ = write!(x, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{root} xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:{root}-2\" xmlns:cac=\"urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2\" xmlns:cbc=\"urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2\">");
    let date = text(&o, "date_order").unwrap_or_default(); let date = if date.len() >= 10 { &date[..10] } else { "1970-01-01" };
    let _ = write!(x, "<cbc:CustomizationID>urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0</cbc:CustomizationID><cbc:ProfileID>urn:fdc:peppol.eu:2017:poacc:billing:01:1.0</cbc:ProfileID><cbc:ID>{}</cbc:ID><cbc:IssueDate>{date}</cbc:IssueDate>", esc(&text(&o, "name").unwrap_or_default()));
    if !credit { let _ = write!(x, "<cbc:DueDate>{date}</cbc:DueDate>"); }
    let _ = write!(x, "<cbc:{}TypeCode>{code}</cbc:{}TypeCode><cbc:DocumentCurrencyCode>{cur}</cbc:DocumentCurrencyCode><cbc:BuyerReference>{}</cbc:BuyerReference>", root, root, esc(&text(&partner, "name").unwrap_or_default()));
    party(&mut x, "AccountingSupplierParty", &text(&company, "name").unwrap_or_default(), &text(&company, "vat").unwrap_or_default(), &text(&company, "street").unwrap_or_default(), &text(&company, "city").unwrap_or_default(), &text(&company, "zip").unwrap_or_default(), &country(&company));
    party(&mut x, "AccountingCustomerParty", &text(&partner, "name").unwrap_or_default(), &text(&partner, "vat").unwrap_or_default(), &text(&partner, "street").unwrap_or_default(), &text(&partner, "city").unwrap_or_default(), &text(&partner, "zip").unwrap_or_default(), &country(&partner));
    let mut lines = String::new(); let (mut net, mut gross) = (0.0, 0.0); let mut by_rate: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for (i, l) in children(env, "pos.order.line", "order_id", order)?.iter().enumerate() {
        let (sub, incl) = (num(l, "price_subtotal") * sign, num(l, "price_subtotal_incl") * sign);
        let qty = num(l, "qty").abs(); let tax = incl - sub;
        let rate = if sub.abs() > 0.004 { ((tax / sub) * 10_000.0).round() as i64 } else { 0 };    // hundredths of a percent
        net += sub; gross += incl; let e = by_rate.entry(rate).or_default(); e.0 += sub; e.1 += tax;
        let cat = if rate == 0 { "Z" } else { "S" }; let pct = format!("{:.2}", rate as f64 / 100.0);
        let unit = if qty > 0.0 { sub / qty } else { 0.0 };
        let _ = write!(lines, "<cac:{line_tag}><cbc:ID>{}</cbc:ID><cbc:{qty_tag} unitCode=\"C62\">{}</cbc:{qty_tag}><cbc:LineExtensionAmount currencyID=\"{cur}\">{}</cbc:LineExtensionAmount><cac:Item><cbc:Name>{}</cbc:Name><cac:ClassifiedTaxCategory><cbc:ID>{cat}</cbc:ID><cbc:Percent>{pct}</cbc:Percent><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:ClassifiedTaxCategory></cac:Item><cac:Price><cbc:PriceAmount currencyID=\"{cur}\">{}</cbc:PriceAmount></cac:Price></cac:{line_tag}>",
            i + 1, qty, money(sub), esc(&text(l, "full_product_name").unwrap_or_default()), money(unit));
    }
    let tax_total: f64 = by_rate.values().map(|v| v.1).sum();
    let _ = write!(x, "<cac:TaxTotal><cbc:TaxAmount currencyID=\"{cur}\">{}</cbc:TaxAmount>", money(tax_total));
    for (rate, (base, t)) in &by_rate { let cat = if *rate == 0 { "Z" } else { "S" }; let _ = write!(x, "<cac:TaxSubtotal><cbc:TaxableAmount currencyID=\"{cur}\">{}</cbc:TaxableAmount><cbc:TaxAmount currencyID=\"{cur}\">{}</cbc:TaxAmount><cac:TaxCategory><cbc:ID>{cat}</cbc:ID><cbc:Percent>{:.2}</cbc:Percent><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:TaxCategory></cac:TaxSubtotal>", money(*base), money(*t), *rate as f64 / 100.0); }
    let _ = write!(x, "</cac:TaxTotal><cac:LegalMonetaryTotal><cbc:LineExtensionAmount currencyID=\"{cur}\">{n}</cbc:LineExtensionAmount><cbc:TaxExclusiveAmount currencyID=\"{cur}\">{n}</cbc:TaxExclusiveAmount><cbc:TaxInclusiveAmount currencyID=\"{cur}\">{g}</cbc:TaxInclusiveAmount><cbc:PayableAmount currencyID=\"{cur}\">{g}</cbc:PayableAmount></cac:LegalMonetaryTotal>{lines}</{root}>", n = money(net), g = money(gross));
    Ok(x)
}

pub fn rules() -> Rules {
    Rules::default().action("pos.order", "ubl", |env, ids, _| Ok(Value::Text(ubl(&env.sudo(), *ids.first().ok_or_else(|| OdooError::User("Select an order".into()))?)?)))
}
