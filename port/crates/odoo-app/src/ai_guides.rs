//! Setup knowledge for the assistant's `setup_guide` tool: how to get an app working, with live checks against the caller's own
//! database (so the plan says what is already done) and per-country notes. Pure data; evaluation lives in `App::setup_guide`.

pub enum Check { None, Module(&'static str), Count(&'static str, &'static str), CompanyCountry, CompanyCurrency }
pub struct Step { pub title: &'static str, pub how: &'static str, pub check: Check }
pub struct Guide { pub topic: &'static str, pub title: &'static str, pub aliases: &'static [&'static str], pub modules: &'static [&'static str], pub steps: &'static [Step] }
pub struct Region { pub code: &'static str, pub name: &'static str, pub currency: &'static str, pub tax: &'static str, pub localization: &'static str, pub notes: &'static [&'static str] }

const fn s(title: &'static str, how: &'static str, check: Check) -> Step { Step { title, how, check } }

pub const GUIDES: &[Guide] = &[
    Guide { topic: "ecommerce", title: "Set up an online store", aliases: &["e-commerce", "online store", "online shop", "webshop", "web shop", "shop", "store", "website_sale", "storefront", "sell online"], modules: &["website_sale"], steps: &[
        s("Install the Website and eCommerce apps", "Apps → install `website_sale` (it brings Website, Sales, Delivery and Payment). Use propose_action kind \"install\" with values {\"module\": \"website_sale\"} if it is missing.", Check::Module("website_sale")),
        s("Set the company country, currency and address", "Edit the company (`res.company`): name, address, country, currency and logo. The country decides taxes, address format and the default checkout countries.", Check::CompanyCountry),
        s("Check the company currency", "The currency is what prices are shown in. Change it before creating products.", Check::CompanyCurrency),
        s("Install the country's accounting localization", "Install the `l10n_<country>` module (chart of accounts and taxes such as GST/VAT), then confirm at least one sales tax exists.", Check::Count("account.tax", "[[\"type_tax_use\",\"=\",\"sale\"]]")),
        s("Create and publish products", "Create products (`product.template`) with \"Can be sold\" ticked, a price, a category and an image, then set \"Is published\" so they appear in the shop.", Check::Count("product.template", "[[\"sale_ok\",\"=\",true],[\"website_published\",\"=\",true]]")),
        s("Enable a payment provider", "Add and enable at least one payment provider (`payment.provider`, e.g. Stripe or PayPal). Use test mode until you are ready to take real payments.", Check::Count("payment.provider", "[[\"state\",\"in\",[\"enabled\",\"test\"]]]")),
        s("Add shipping methods", "Create delivery methods (`delivery.carrier`): fixed price, free over a threshold, or a carrier integration, with the countries they serve.", Check::Count("delivery.carrier", "[]")),
        s("Configure the website", "Set the website name, domain, languages, pricelists and whether prices show with or without tax; add terms and a privacy page.", Check::Count("website", "[]")),
        s("Place a test order and go live", "Open the storefront at /storefront/, add a product to the cart, check out with a test payment, then confirm the order and delivery appear in Sales and Inventory.", Check::None),
    ] },
    Guide { topic: "pos", title: "Set up a point of sale", aliases: &["point of sale", "cash register", "till", "retail", "restaurant", "point_of_sale"], modules: &["point_of_sale"], steps: &[
        s("Install Point of Sale", "Apps → install `point_of_sale` (and `pos_restaurant` for tables and kitchen).", Check::Module("point_of_sale")),
        s("Create products available in POS", "Products need \"Available in POS\" ticked and a price.", Check::Count("product.template", "[[\"available_in_pos\",\"=\",true]]")),
        s("Configure a register", "Create a POS configuration (`pos.config`): name, payment methods, receipt header, optional pricelist and loyalty.", Check::Count("pos.config", "[]")),
        s("Add payment methods", "Cash, card and customer account methods (`pos.payment.method`); link a terminal for card readers.", Check::Count("pos.payment.method", "[]")),
        s("Open a session and sell", "Open /pos/, choose the register, and take a test sale before opening to customers.", Check::None),
    ] },
    Guide { topic: "sales", title: "Set up quotations and sales orders", aliases: &["quotation", "quotations", "sales order", "sale", "selling", "sale_management", "invoice customers"], modules: &["sale_management"], steps: &[
        s("Install Sales", "Apps → install `sale_management`.", Check::Module("sale_management")),
        s("Create your products and services", "Products (`product.template`) with prices and taxes.", Check::Count("product.template", "[[\"sale_ok\",\"=\",true]]")),
        s("Add customers", "Customers are partners (`res.partner`).", Check::Count("res.partner", "[[\"customer_rank\",\">\",0]]")),
        s("Make a quotation and confirm it", "Create a `sale.order`, add lines, confirm, then create the invoice.", Check::Count("sale.order", "[]")),
    ] },
    Guide { topic: "inventory", title: "Set up inventory and warehouses", aliases: &["stock", "warehouse", "warehouses", "stock management"], modules: &["stock"], steps: &[
        s("Install Inventory", "Apps → install `stock`.", Check::Module("stock")),
        s("Configure the warehouse", "Check the default warehouse (`stock.warehouse`): name, short code, routes (receive/deliver in 1, 2 or 3 steps).", Check::Count("stock.warehouse", "[]")),
        s("Make products storable", "Set the product type to goods with stock tracking, and optional lots or serial numbers.", Check::Count("product.template", "[[\"is_storable\",\"=\",true]]")),
        s("Set starting quantities and reordering rules", "Adjust on-hand quantities and add reordering rules (`stock.warehouse.orderpoint`).", Check::None),
    ] },
    Guide { topic: "accounting", title: "Set up accounting", aliases: &["bookkeeping", "invoicing", "taxes", "chart of accounts", "account"], modules: &["account"], steps: &[
        s("Install Accounting", "Apps → install `account`.", Check::Module("account")),
        s("Install the country localization", "Install `l10n_<country>` for the chart of accounts and taxes.", Check::Count("account.tax", "[]")),
        s("Set company country and currency", "Edit the company (`res.company`).", Check::CompanyCountry),
        s("Add bank journals", "Create bank and cash journals (`account.journal`) and import statements to reconcile.", Check::Count("account.journal", "[[\"type\",\"in\",[\"bank\",\"cash\"]]]")),
        s("Create the first invoice", "Create a customer invoice (`account.move`, type out_invoice) and register a payment.", Check::Count("account.move", "[[\"move_type\",\"=\",\"out_invoice\"]]")),
    ] },
    Guide { topic: "crm", title: "Set up CRM and a sales pipeline", aliases: &["leads", "pipeline", "opportunities", "customers relationship"], modules: &["crm"], steps: &[
        s("Install CRM", "Apps → install `crm`.", Check::Module("crm")),
        s("Define pipeline stages", "Review the stages (`crm.stage`) and rename them to match your sales process.", Check::Count("crm.stage", "[]")),
        s("Add sales teams and salespeople", "Create sales teams (`crm.team`) and assign users.", Check::Count("crm.team", "[]")),
        s("Capture leads", "Create leads or opportunities (`crm.lead`); website forms and emails can create them automatically.", Check::Count("crm.lead", "[]")),
    ] },
    Guide { topic: "manufacturing", title: "Set up manufacturing", aliases: &["production", "mrp", "bill of materials", "bom", "factory"], modules: &["mrp"], steps: &[
        s("Install Manufacturing", "Apps → install `mrp` (it needs Inventory).", Check::Module("mrp")),
        s("Create components and finished products", "Products for raw materials and for what you make.", Check::Count("product.template", "[]")),
        s("Define bills of materials", "A `mrp.bom` lists components and quantities for each finished product.", Check::Count("mrp.bom", "[]")),
        s("Create a manufacturing order", "Create and confirm a `mrp.production`, then produce.", Check::Count("mrp.production", "[]")),
    ] },
    Guide { topic: "project", title: "Set up projects and tasks", aliases: &["projects", "tasks", "timesheets", "task management"], modules: &["project"], steps: &[
        s("Install Project", "Apps → install `project` (and `hr_timesheet` to log time).", Check::Module("project")),
        s("Create a project", "A `project.project` with its stages.", Check::Count("project.project", "[]")),
        s("Add tasks", "Create tasks (`project.task`), assign people and deadlines.", Check::Count("project.task", "[]")),
    ] },
];

pub const REGIONS: &[Region] = &[
    Region { code: "AU", name: "Australia", currency: "AUD", tax: "GST 10%", localization: "l10n_au", notes: &[
        "Set the company country to Australia and the currency to AUD before creating products.",
        "Install `l10n_au` for the Australian chart of accounts and GST taxes; the ABN goes in the company's Tax ID.",
        "Prices shown to consumers are normally GST-inclusive: set the website to show prices tax-included.",
        "Addresses use a state (NSW, VIC, QLD, WA, SA, TAS, ACT, NT) and a 4-digit postcode.",
        "Shipping: set up delivery methods for Australia Post, Sendle or flat-rate/free-over-threshold shipping, with the country restricted to Australia (and New Zealand if you ship there).",
        "Payments: enable Stripe and/or PayPal as payment providers; start in test mode.",
        "GST registration is only mandatory above AUD 75,000 annual turnover; confirm your obligations with an accountant."] },
    Region { code: "NZ", name: "New Zealand", currency: "NZD", tax: "GST 15%", localization: "l10n_nz", notes: &["Set currency NZD; install `l10n_nz` for GST 15% and the NZ chart of accounts.", "Prices to consumers are GST-inclusive."] },
    Region { code: "GB", name: "United Kingdom", currency: "GBP", tax: "VAT 20%", localization: "l10n_uk", notes: &["Set currency GBP; install `l10n_uk` for VAT and the UK chart of accounts.", "Put the VAT number in the company's Tax ID; consumer prices are VAT-inclusive."] },
    Region { code: "US", name: "United States", currency: "USD", tax: "state sales tax", localization: "l10n_us", notes: &["Set currency USD; sales tax varies by state, so create taxes per state you collect in (or use a tax service).", "Prices are usually shown tax-excluded."] },
    Region { code: "CA", name: "Canada", currency: "CAD", tax: "GST/HST/PST", localization: "l10n_ca", notes: &["Set currency CAD; install `l10n_ca` for GST/HST/PST by province."] },
    Region { code: "DE", name: "Germany", currency: "EUR", tax: "MwSt 19%", localization: "l10n_de", notes: &["Set currency EUR; install `l10n_de`; consumer prices are VAT-inclusive; add legal pages (Impressum, AGB, Widerruf)."] },
    Region { code: "FR", name: "France", currency: "EUR", tax: "TVA 20%", localization: "l10n_fr", notes: &["Set currency EUR; install `l10n_fr`; consumer prices are VAT-inclusive; add mentions légales."] },
    Region { code: "IN", name: "India", currency: "INR", tax: "GST", localization: "l10n_in", notes: &["Set currency INR; install `l10n_in` for GST; the GSTIN goes in the company's Tax ID."] },
];

/// The guide for a topic or any of its aliases (case-insensitive; a longer free-text topic matches if it contains an alias).
pub fn find(topic: &str) -> Option<&'static Guide> {
    let t = topic.trim().to_lowercase();
    if t.is_empty() { return None; }
    GUIDES.iter().find(|g| g.topic == t || g.aliases.iter().any(|a| *a == t))
        .or_else(|| GUIDES.iter().find(|g| t.contains(g.topic) || g.aliases.iter().any(|a| t.contains(a))))
}
pub fn region(code: &str) -> Option<&'static Region> {
    let c = code.trim();
    REGIONS.iter().find(|r| r.code.eq_ignore_ascii_case(c) || r.name.eq_ignore_ascii_case(c))
}
pub fn topics() -> Vec<&'static str> { GUIDES.iter().map(|g| g.topic).collect() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn topics_and_aliases_resolve() {
        assert_eq!(find("e-commerce").unwrap().topic, "ecommerce"); assert_eq!(find("Online Store").unwrap().topic, "ecommerce");
        assert_eq!(find("how do I set up an e-commerce store").unwrap().topic, "ecommerce", "free text containing an alias");
        assert_eq!(find("pos").unwrap().topic, "pos"); assert!(find("").is_none() && find("quantum").is_none());
        assert_eq!(region("au").unwrap().currency, "AUD"); assert_eq!(region("Australia").unwrap().localization, "l10n_au"); assert!(region("zz").is_none());
    }
    #[test] fn every_guide_has_steps_and_valid_domains() {
        for g in GUIDES { assert!(g.steps.len() >= 3, "{}", g.topic); for st in g.steps { if let Check::Count(_, d) = st.check { assert!(serde_json::from_str::<serde_json::Value>(d).map_or(false, |v| v.is_array()), "{} {}", g.topic, d); } } }
    }
}
