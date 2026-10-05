//! Free-trial signups (the /trial page): every signup gets its own isolated database with the chosen apps installed and the
//! visitor as its administrator. Public methods: `trial_catalog`, `trial_create`. Nothing here can read or write the main database.
use odoo_core::{OdooError, Result};
use serde_json::{json, Value as J};
use std::path::PathBuf;

pub struct TrialConfig { pub dir: PathBuf, pub max_active: usize, pub days: u32 }

/// Odoo's trial app picker, mapped to this edition's installable modules. `module: None` = Enterprise-only (shown greyed out).
pub struct TrialApp { pub name: &'static str, pub category: &'static str, pub module: Option<&'static str>, pub icon: &'static str }
const fn a(name: &'static str, category: &'static str, module: Option<&'static str>, icon: &'static str) -> TrialApp { TrialApp { name, category, module, icon } }
pub const CATALOG: &[TrialApp] = &[
    a("Website", "Website", Some("website"), "🌐"), a("eCommerce", "Website", Some("website_sale"), "🛍️"), a("Blog", "Website", Some("website_blog"), "📰"),
    a("Forum", "Website", Some("website_forum"), "💬"), a("eLearning", "Website", Some("website_slides"), "🎓"), a("Events", "Website", Some("event"), "🎟️"),
    a("CRM", "Sales", Some("crm"), "🤝"), a("Sales", "Sales", Some("sale_management"), "💰"), a("Point of Sale", "Sales", Some("point_of_sale"), "🧾"),
    a("Restaurant", "Sales", Some("pos_restaurant"), "🍽️"), a("Subscriptions", "Sales", None, "🔁"), a("Rental", "Sales", None, "🔑"),
    a("Invoicing", "Finance", Some("account"), "📄"), a("Accounting", "Finance", Some("account"), "📒"), a("Expenses", "Finance", Some("hr_expense"), "💳"),
    a("Sign", "Finance", None, "✍️"), a("Equity", "Finance", None, "📈"), a("ESG", "Finance", None, "🌱"),
    a("Project", "Services", Some("project"), "📋"), a("Timesheets", "Services", Some("hr_timesheet"), "⏱️"), a("Field Service", "Services", None, "🛠️"),
    a("Helpdesk", "Services", None, "🎧"), a("Appointments", "Services", None, "📅"), a("Planning", "Services", None, "🗓️"),
    a("Documents", "Productivity", None, "🗂️"), a("Approvals", "Productivity", None, "✅"), a("Knowledge", "Productivity", None, "📚"),
    a("Inventory", "Supply Chain", Some("stock"), "📦"), a("Manufacturing", "Supply Chain", Some("mrp"), "🏭"), a("Purchase", "Supply Chain", Some("purchase"), "🛒"),
    a("Maintenance", "Supply Chain", Some("maintenance"), "🔧"), a("Quality", "Supply Chain", None, "🎯"), a("Repair", "Supply Chain", Some("repair"), "🔩"),
    a("Email Marketing", "Marketing", Some("mass_mailing"), "✉️"), a("SMS Marketing", "Marketing", Some("mass_mailing_sms"), "📱"), a("Survey", "Marketing", Some("survey"), "📝"),
    a("Social Marketing", "Marketing", None, "📣"),
    a("Employees", "Human Resources", Some("hr"), "👥"), a("Attendances", "Human Resources", Some("hr_attendance"), "🕘"), a("Recruitment", "Human Resources", Some("hr_recruitment"), "🧑‍💼"),
    a("Time Off", "Human Resources", Some("hr_holidays"), "🏖️"), a("Appraisals", "Human Resources", None, "⭐"), a("Fleet", "Human Resources", Some("fleet"), "🚗"), a("Payroll", "Human Resources", None, "💵"),
    a("Studio", "Customizations", None, "🧩"),
];

#[derive(Debug, Clone)]
pub struct Signup { pub name: String, pub email: String, pub phone: String, pub company: String, pub country: String, pub password: String, pub apps: Vec<String> }

fn s(v: &J, k: &str) -> String { v[k].as_str().unwrap_or("").trim().to_string() }

/// Pure validation of the public form: bounded sizes, a plausible email, a real password, and only catalog apps.
pub fn parse_signup(v: &J, available: &dyn Fn(&str) -> bool) -> Result<Signup> {
    let bad = |m: &str| Err(OdooError::Validation(m.into()));
    let sg = Signup { name: s(v, "name"), email: s(v, "email").to_lowercase(), phone: s(v, "phone"), company: s(v, "company"), country: s(v, "country").to_uppercase(), password: v["password"].as_str().unwrap_or("").to_string(),
        apps: v["apps"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect() };
    if sg.name.is_empty() || sg.name.chars().count() > 100 { return bad("Please enter your name."); }
    let at = sg.email.find('@');
    let ok_email = sg.email.len() <= 120 && at.map_or(false, |i| i > 0 && sg.email[i + 1..].contains('.') && !sg.email[i + 1..].starts_with('.') && !sg.email.ends_with('.')) && sg.email.chars().all(|c| c.is_ascii_graphic()) && sg.email.matches('@').count() == 1;
    if !ok_email { return bad("Please enter a valid email address."); }
    if sg.company.chars().count() > 100 { return bad("Company name is too long."); }
    if sg.phone.len() > 30 || !sg.phone.chars().all(|c| c.is_ascii_digit() || " +-().".contains(c)) { return bad("Please enter a valid phone number."); }
    if !(sg.country.is_empty() || (sg.country.len() == 2 && sg.country.chars().all(|c| c.is_ascii_alphabetic()))) { return bad("Invalid country."); }
    if sg.password.chars().count() < 8 || sg.password.len() > 200 { return bad("The password must have at least 8 characters."); }
    if sg.apps.is_empty() || sg.apps.len() > 15 { return bad("Choose between 1 and 15 apps."); }
    for app in &sg.apps { if !CATALOG.iter().any(|c| c.name == app && c.module.map_or(false, |m| available(m))) { return bad(&format!("`{app}` is not available in this edition.")); } }
    Ok(sg)
}

/// Modules to install for the chosen apps (deduplicated, `contacts` always included).
pub fn modules_for(apps: &[String]) -> Vec<String> {
    let mut out: Vec<String> = vec!["contacts".into()];
    for app in apps { if let Some(m) = CATALOG.iter().find(|c| c.name == app).and_then(|c| c.module) { if !out.iter().any(|x| x == m) { out.push(m.to_string()); } } }
    out
}

pub fn catalog_json(available: &dyn Fn(&str) -> bool) -> J {
    J::Array(CATALOG.iter().map(|c| json!({"name": c.name, "category": c.category, "icon": c.icon, "available": c.module.map_or(false, |m| available(m))})).collect())
}

/// A trial database name: `trial-` + 16 hex chars. Anything else is rejected before it can touch the filesystem.
pub fn valid_db(db: &str) -> bool { db.len() == 22 && db.starts_with("trial-") && db[6..].chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) || valid_standard_name(db) }

/// An administrator-created (non-trial) database name: 3-30 chars of `a-z 0-9 _`, starting with a letter, never `trial…` or a reserved word.
pub fn valid_standard_name(db: &str) -> bool {
    (3..=30).contains(&db.len()) && db.chars().next().map_or(false, |c| c.is_ascii_lowercase()) && db.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !db.starts_with("trial") && !["main", "backups", "postgres", "template", "odoo"].contains(&db)
}

/// Modules that must not be installed when the given catalog apps are disallowed (a module still needed by an allowed app stays available).
pub fn denied_modules(disallowed: &[String]) -> Vec<String> {
    let module = |n: &str| CATALOG.iter().find(|c| c.name == n).and_then(|c| c.module);
    let kept: Vec<&str> = CATALOG.iter().filter(|c| !disallowed.iter().any(|d| d == c.name)).filter_map(|c| c.module).collect();
    let mut out: Vec<String> = vec![];
    for d in disallowed { if let Some(m) = module(d) { if !kept.contains(&m) && !out.iter().any(|x| x == m) { out.push(m.to_string()); } } }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn all(_: &str) -> bool { true }
    fn good() -> J { json!({"name": "Ada Lovelace", "email": "Ada@Example.com", "phone": "+1 (555) 010-2030", "company": "Analytical Engines", "country": "gb", "password": "correct horse", "apps": ["CRM", "Sales"]}) }
    #[test] fn accepts_a_valid_signup_and_normalises_it() {
        let s = parse_signup(&good(), &all).unwrap();
        assert_eq!((s.email.as_str(), s.country.as_str()), ("ada@example.com", "GB")); assert_eq!(modules_for(&s.apps), vec!["contacts", "crm", "sale_management"]);
    }
    #[test] fn rejects_bad_input() {
        let with = |k: &str, v: J| { let mut g = good(); g[k] = v; parse_signup(&g, &all).is_err() };
        assert!(with("email", json!("nope"))); assert!(with("email", json!("a@b"))); assert!(with("email", json!("a b@c.com"))); assert!(with("email", json!("a@@b.com")));
        assert!(with("password", json!("short"))); assert!(with("name", json!("  "))); assert!(with("name", json!("x".repeat(101))));
        assert!(with("phone", json!("<script>"))); assert!(with("country", json!("../../"))); assert!(with("apps", json!([]))); assert!(with("apps", json!(["Studio"])));   // enterprise-only
        assert!(with("apps", json!(["CRM; DROP TABLE"]))); assert!(with("apps", json!(vec!["Sales"; 16])));
        assert!(parse_signup(&good(), &|m| m != "crm").is_err(), "an app whose module is not installable here is refused");
    }
    #[test] fn accounting_and_invoicing_share_a_module() { assert_eq!(modules_for(&["Invoicing".into(), "Accounting".into()]), vec!["contacts", "account"]); }
    #[test] fn db_names_cannot_escape_the_trial_directory() {
        assert!(valid_db("trial-0123456789abcdef")); assert!(!valid_db("trial-../../etc/passwd")); assert!(!valid_db("../trial-0123456789abcdef")); assert!(!valid_db("trial-0123456789ABCDEF")); assert!(!valid_db("main")); assert!(!valid_db(""));
        assert!(valid_db("acme_prod") && !valid_db("Acme") && !valid_db("../acme") && !valid_db("trial_x") && !valid_db("ab") && !valid_db("backups") && !valid_db("a/b") && !valid_db("9lives"));
    }
    #[test] fn disallowing_an_app_keeps_modules_other_apps_need() {
        assert_eq!(denied_modules(&["CRM".into()]), vec!["crm"]);
        assert!(denied_modules(&["Invoicing".into()]).is_empty(), "Accounting still needs the account module");
        assert_eq!(denied_modules(&["Invoicing".into(), "Accounting".into()]), vec!["account"]);
        assert!(denied_modules(&["Studio".into(), "nope".into()]).is_empty());
    }
    #[test] fn catalog_marks_enterprise_only_apps() {
        let c = catalog_json(&all); let find = |n: &str| c.as_array().unwrap().iter().find(|x| x["name"] == n).unwrap().clone();
        assert_eq!(find("CRM")["available"], true); assert_eq!(find("Studio")["available"], false);
    }
}
