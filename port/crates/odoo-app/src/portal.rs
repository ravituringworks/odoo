//! Customer portal (the `/my` pages): accounts are created at trial signup; an account owns trial databases and can list, open,
//! delete and create more of them, edit its details and change its password. Sessions are separate from database sessions.
//! "Open" mints a session inside the trial for the *owner only*, so the trial's administrator password never has to be shared.
use super::*;
use std::time::{Duration, Instant};

const SESSION_TTL: Duration = Duration::from_secs(12 * 3600);
const MAX_FAILS: usize = 5;
const LOCK: Duration = Duration::from_secs(300);
pub const MAX_DATABASES: i64 = 3;

pub const PORTAL_DDL: &str = "CREATE TABLE IF NOT EXISTS odoo_portal_users (email VARCHAR(160) PRIMARY KEY, name VARCHAR(100), company VARCHAR(100), phone VARCHAR(30), country VARCHAR(2), salt VARCHAR(64), hash VARCHAR(128), created_at VARCHAR(30))";

fn text(r: &Row, k: &str) -> String { r.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string() }
fn bad<T>(m: &str) -> Result<T> { Err(OdooError::Validation(m.into())) }
fn ph(c: &dyn odoo_core::store::Conn, n: usize) -> String { c.dialect().placeholder(n) }

/// Editable account details, validated like the signup form.
pub fn parse_details(v: &J) -> Result<(String, String, String, String)> {
    let s = |k: &str| v[k].as_str().unwrap_or("").trim().to_string();
    let (name, company, phone, country) = (s("name"), s("company"), s("phone"), s("country").to_uppercase());
    if name.is_empty() || name.chars().count() > 100 { return bad("Please enter your name."); }
    if company.chars().count() > 100 { return bad("Company name is too long."); }
    if phone.len() > 30 || !phone.chars().all(|c| c.is_ascii_digit() || " +-().".contains(c)) { return bad("Please enter a valid phone number."); }
    if !(country.is_empty() || (country.len() == 2 && country.chars().all(|c| c.is_ascii_alphabetic()))) { return bad("Invalid country."); }
    Ok((name, company, phone, country))
}

impl App {
    fn portal_row(&self, email: &str) -> Result<Option<Row>> {
        store::run(self.store.as_ref(), |c| Ok(c.query(&format!("SELECT * FROM odoo_portal_users WHERE email = {}", ph(c, 1)), &[Value::Text(email.into())])?.into_iter().next()))
    }

    /// Create the account for a signup; fails if the email already has one.
    pub(crate) fn portal_create_account(&self, sg: &trial::Signup) -> Result<()> {
        if self.portal_row(&sg.email)?.is_some() { return Err(OdooError::User("An account already exists for this email address. Sign in to create another database.".into())); }
        let salt = security::random_hex(16); let hash = security::hash_password(&salt, &sg.password);
        store::run(self.store.as_ref(), |c| c.execute(&format!("INSERT INTO odoo_portal_users (email, name, company, phone, country, salt, hash, created_at) VALUES ({}, {}, {}, {}, {}, {}, {}, {})", ph(c, 1), ph(c, 2), ph(c, 3), ph(c, 4), ph(c, 5), ph(c, 6), ph(c, 7), ph(c, 8)),
            &[sg.email.clone().into(), sg.name.clone().into(), sg.company.clone().into(), sg.phone.clone().into(), sg.country.clone().into(), salt.clone().into(), hash.clone().into(), orm::now().into()]).map(|_| ()))
            .map_err(|_| OdooError::User("An account already exists for this email address. Sign in to create another database.".into()))
    }
    pub(crate) fn portal_delete_account(&self, email: &str) { let _ = store::run(self.store.as_ref(), |c| c.execute(&format!("DELETE FROM odoo_portal_users WHERE email = {}", ph(c, 1)), &[Value::Text(email.into())]).map(|_| ())); }

    pub(crate) fn portal_new_session(&self, email: &str) -> String {
        let tok = security::random_hex(32); let now = Instant::now();
        let mut s = self.portal_sessions.lock().unwrap(); s.retain(|_, (_, t)| now.duration_since(*t) < SESSION_TTL); s.insert(tok.clone(), (email.to_string(), now)); tok
    }
    fn portal_email(&self, token: Option<&str>) -> Result<String> {
        let denied = || OdooError::AccessDenied { op: "session".into(), model: "portal".into(), uid: 0 };
        let tok = token.ok_or_else(denied)?; let now = Instant::now();
        let mut s = self.portal_sessions.lock().unwrap();
        match s.get(tok) { Some((e, t)) if now.duration_since(*t) < SESSION_TTL => Ok(e.clone()), Some(_) => { s.remove(tok); Err(denied()) } None => Err(denied()) }
    }

    /// Credentials -> portal session. Failed attempts are throttled per account (5 per 5 minutes) and cost the same time for unknown emails.
    pub fn portal_login(&self, email: &str, pw: &str) -> Result<J> {
        let email = email.trim().to_lowercase(); let now = Instant::now();
        { let mut f = self.login_failures.lock().unwrap(); let q = f.entry(email.clone()).or_default(); q.retain(|t| now.duration_since(*t) < LOCK);
          if q.len() >= MAX_FAILS { return Err(OdooError::User("Too many failed attempts. Please try again in a few minutes.".into())); } }
        let row = self.portal_row(&email)?;
        let ok = match &row { Some(r) => security::hash_password(&text(r, "salt"), pw) == text(r, "hash"), None => { let _ = security::hash_password("x", pw); false } };
        if !ok { self.login_failures.lock().unwrap().entry(email).or_default().push_back(now); return Err(OdooError::User("Wrong email or password.".into())); }
        self.login_failures.lock().unwrap().remove(&email);
        let r = row.unwrap();
        Ok(json!({"token": self.portal_new_session(&email), "email": email, "name": text(&r, "name")}))
    }

    fn owned_trials(&self, email: &str) -> Result<Vec<Row>> {
        store::run(self.store.as_ref(), |c| c.query(&format!("SELECT db, company, apps, created_at FROM odoo_trials WHERE email = {} ORDER BY created_at DESC", ph(c, 1)), &[Value::Text(email.into())]))
    }
    fn owns(&self, email: &str, db: &str) -> Result<bool> {
        store::run(self.store.as_ref(), |c| Ok(!c.query(&format!("SELECT 1 FROM odoo_trials WHERE db = {} AND email = {}", ph(c, 1), ph(c, 2)), &[db.into(), email.into()])?.is_empty()))
    }

    fn portal_me(&self, email: &str) -> Result<J> {
        let r = self.portal_row(email)?.ok_or_else(|| OdooError::AccessDenied { op: "session".into(), model: "portal".into(), uid: 0 })?;
        let days = self.trial_cfg.as_ref().map_or(15, |c| c.days as i64); let today = orm::epoch_days(&orm::today());
        let dbs: Vec<J> = self.owned_trials(email)?.iter().map(|t| {
            let created = text(t, "created_at"); let left = (days - (today - orm::epoch_days(&created))).max(0);
            json!({"db": text(t, "db"), "company": text(t, "company"), "apps": serde_json::from_str::<J>(&text(t, "apps")).unwrap_or(json!([])), "created_at": created, "days_left": left})
        }).collect();
        Ok(json!({"email": email, "name": text(&r, "name"), "company": text(&r, "company"), "phone": text(&r, "phone"), "country": text(&r, "country"), "created_at": text(&r, "created_at"), "databases": dbs, "max_databases": MAX_DATABASES, "trial_days": days}))
    }

    /// Dispatch for `portal_*` methods (token = portal session, never a database session).
    pub(crate) fn portal_dispatch(&self, req: &Request) -> Result<J> {
        let a0 = req.args.first().cloned().unwrap_or(J::Null);
        if req.method == "portal_login" { return self.portal_login(req.args.first().and_then(|v| v.as_str()).unwrap_or(""), req.args.get(1).and_then(|v| v.as_str()).unwrap_or("")); }
        let email = self.portal_email(req.token.as_deref())?;
        match req.method.as_str() {
            "portal_me" => self.portal_me(&email),
            "portal_logout" => { if let Some(t) = &req.token { self.portal_sessions.lock().unwrap().remove(t); } Ok(J::Bool(true)) }
            "portal_update" => {
                let (name, company, phone, country) = parse_details(&a0)?;
                store::run(self.store.as_ref(), |c| c.execute(&format!("UPDATE odoo_portal_users SET name = {}, company = {}, phone = {}, country = {} WHERE email = {}", ph(c, 1), ph(c, 2), ph(c, 3), ph(c, 4), ph(c, 5)), &[name.clone().into(), company.clone().into(), phone.clone().into(), country.clone().into(), email.clone().into()]).map(|_| ()))?;
                self.portal_me(&email)
            }
            "portal_change_password" => {
                let (old, new) = (a0["old"].as_str().unwrap_or(""), a0["new"].as_str().unwrap_or(""));
                let r = self.portal_row(&email)?.ok_or_else(|| OdooError::User("account not found".into()))?;
                if security::hash_password(&text(&r, "salt"), old) != text(&r, "hash") { return Err(OdooError::User("The current password is incorrect.".into())); }
                if new.chars().count() < 8 || new.len() > 200 { return bad("The new password must have at least 8 characters."); }
                let salt = security::random_hex(16); let hash = security::hash_password(&salt, new);
                store::run(self.store.as_ref(), |c| c.execute(&format!("UPDATE odoo_portal_users SET salt = {}, hash = {} WHERE email = {}", ph(c, 1), ph(c, 2), ph(c, 3)), &[salt.clone().into(), hash.clone().into(), email.clone().into()]).map(|_| ()))?;
                Ok(J::Bool(true))
            }
            "portal_open" => {
                let db = a0.as_str().unwrap_or("");
                if !trial::valid_db(db) || !self.owns(&email, db)? { return Err(OdooError::AccessDenied { op: "open".into(), model: "database".into(), uid: 0 }); }
                let tenant = self.tenant(db)?;
                Ok(json!({"db": db, "token": tenant.sec.open_session(1)}))
            }
            "portal_delete" => {
                let db = a0.as_str().unwrap_or("");
                if !trial::valid_db(db) || !self.owns(&email, db)? { return Err(OdooError::AccessDenied { op: "delete".into(), model: "database".into(), uid: 0 }); }
                self.delete_trial(db); Ok(J::Bool(true))
            }
            "portal_new_trial" => {
                if self.owned_trials(&email)?.len() as i64 >= MAX_DATABASES { return Err(OdooError::User(format!("You can have at most {MAX_DATABASES} trial databases. Delete one to create another."))); }
                let r = self.portal_row(&email)?.ok_or_else(|| OdooError::User("account not found".into()))?;
                let pick = |k: &str| { let v = a0[k].as_str().unwrap_or("").trim(); if v.is_empty() { text(&r, k) } else { v.to_string() } };
                // the trial's own administrator password is random: the owner reaches it through "Open", never by typing it
                let form = json!({"name": text(&r, "name"), "email": email, "phone": text(&r, "phone"), "company": pick("company"), "country": pick("country"), "password": security::random_hex(12), "apps": a0["apps"]});
                let sg = trial::parse_signup(&form, &|m| self.module_available(m))?;
                let (db, _tenant_token) = self.provision_trial(&sg)?;
                Ok(json!({"db": db, "apps": sg.apps, "welcome_email": self.send_welcome(&sg, req.lang.as_deref().unwrap_or("en"))}))
            }
            other => Err(OdooError::User(format!("unknown portal method `{other}`"))),
        }
    }
}
