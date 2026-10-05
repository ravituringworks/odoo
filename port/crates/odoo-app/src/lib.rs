//! odoo-app: backend-independent application service. One `dispatch` entry point is shared by the
//! HTTP server (Next.js) and the Tauri shell, so both transports expose identical semantics.
pub mod ai;
pub mod i18n;
pub mod portal;
pub mod security;
pub mod email;
pub mod trial;
use odoo_core::orm::{self, AllowAll, Env};
use security::Security;
use odoo_core::store::{self, Store};
use odoo_core::{ddl, Domain, OdooError, Registry, Result, Row, Rules, Value};
use odoo_modules::data::{self, Catalog};
use std::sync::{Arc, Mutex, RwLock};
use serde::Deserialize;
use serde_json::{json, Value as J};
use std::path::PathBuf;

pub struct Config {
    pub backend: String,          // "sqlite" (default); others plug in via `Store` impls
    pub url: String,              // sqlite path or connection URL
    pub modules: Vec<String>,
    pub schema_dir: PathBuf,
    pub data_dir: PathBuf,
    pub views_dir: PathBuf,
    pub i18n_dir: PathBuf,
    pub seed: bool,
    pub require_auth: bool,
    /// Enables the public free-trial signup (each signup gets its own database under `trial.dir`).
    pub trial: Option<trial::TrialConfig>,
    /// Set for trial tenants: they never use the operator's `ODOO_AI_API_KEY`.
    pub is_trial: bool,
}

/// Everything derived from the set of installed modules; replaced atomically when a module is installed.
pub struct Runtime { pub reg: Registry, pub rules: Rules, pub catalog: Catalog, pub roots: Vec<String> }

pub struct App {
    rt: RwLock<Arc<Runtime>>, install_lock: Mutex<()>,
    pub sec: Arc<Security>, pub require_auth: bool,
    pub store: Box<dyn Store>, pub schema_dir: PathBuf, pub data_dir: PathBuf, pub views_dir: PathBuf, i18n: i18n::I18n, seed: bool,
    portal_sessions: Mutex<std::collections::HashMap<String, (String, std::time::Instant)>>, login_failures: Mutex<std::collections::HashMap<String, std::collections::VecDeque<std::time::Instant>>>,
    mailer: Mutex<Arc<dyn email::Mailer>>, send_log: Mutex<std::collections::VecDeque<std::time::Instant>>,
    trial_cfg: Option<trial::TrialConfig>, tenants: RwLock<std::collections::HashMap<String, Arc<App>>>, create_lock: Mutex<std::collections::VecDeque<std::time::Instant>>, is_trial: bool,
}

const MODULES_DDL: &str = "CREATE TABLE IF NOT EXISTS odoo_modules (name VARCHAR(128) PRIMARY KEY)";
const TRIALS_DDL: &str = "CREATE TABLE IF NOT EXISTS odoo_trials (db VARCHAR(40) PRIMARY KEY, email VARCHAR(160), created_at VARCHAR(30), company VARCHAR(100), apps TEXT)";
const SETTINGS_DDL: &str = "CREATE TABLE IF NOT EXISTS odoo_settings (skey VARCHAR(128) PRIMARY KEY, svalue TEXT)";

#[derive(Deserialize, Debug)]
pub struct Request {
    pub method: String,
    #[serde(default)] pub model: String,
    #[serde(default)] pub args: Vec<J>,
    #[serde(default)] pub kwargs: serde_json::Map<String, J>,
    #[serde(default = "one")] pub uid: i64,
    #[serde(default)] pub token: Option<String>,
    #[serde(default)] pub lang: Option<String>,
    /// Trial database name (`trial-<16 hex>`); absent = the main database.
    #[serde(default)] pub db: Option<String>,
}
fn one() -> i64 { 1 }

/// Drop view nodes contributed (`_m`) by modules that are not installed, and strip the marker.
pub fn prune_uninstalled(node: &mut J, installed: &[String]) {
    let Some(o) = node.as_object_mut() else { return };
    if let Some(ch) = o.get_mut("children").and_then(|c| c.as_array_mut()) {
        ch.retain(|c| c["attrs"]["_m"].as_str().map_or(true, |m| installed.iter().any(|i| i == m)));
        ch.iter_mut().for_each(|c| prune_uninstalled(c, installed));
    }
    if let Some(a) = o.get_mut("attrs").and_then(|a| a.as_object_mut()) { a.remove("_m"); }
}

pub fn open_store(backend: &str, url: &str) -> Result<Box<dyn Store>> {
    match backend {
        "sqlite" => Ok(Box::new(odoo_sqlite::SqliteStore::open(url)?)),
        #[cfg(feature = "duckdb")]
        "duckdb" => Ok(Box::new(odoo_duckdb::DuckStore::open(url)?)),
        #[cfg(feature = "postgres")]
        "postgres" | "postgresql" => Ok(Box::new(odoo_postgres::PgStore::open(url)?)),
        #[cfg(feature = "postgres")]
        "orbit-rs" | "orbit" => Ok(Box::new(odoo_postgres::PgStore::open_orbit(url)?)),
        #[cfg(feature = "mysql")]
        "mysql" | "mariadb" => Ok(Box::new(odoo_mysql::MySqlStore::open(url)?)),
        other => Err(OdooError::Storage(format!("backend `{other}` needs its driver crate (build odoo-app with --features postgres|mysql|duckdb)"))),
    }
}

impl App {
    pub fn open(cfg: &Config) -> Result<App> {
        let st = open_store(&cfg.backend, &cfg.url)?;
        store::run(st.as_ref(), |c| { c.execute(security::AUTH_DDL, &[])?; c.execute(MODULES_DDL, &[])?; c.execute(SETTINGS_DDL, &[])?; c.execute(TRIALS_DDL, &[])?; c.execute(portal::PORTAL_DDL, &[]).map(|_| ()) })?;
        // databases created before the portal existed lack these columns (ADD COLUMN failures are ignored)
        store::apply_upgrade(st.as_ref(), &["ALTER TABLE odoo_trials ADD COLUMN company VARCHAR(100)".to_string(), "ALTER TABLE odoo_trials ADD COLUMN apps TEXT".to_string()])?;
        // installed roots live in the database; the config list only bootstraps a fresh one
        let mut roots: Vec<String> = store::run(st.as_ref(), |c| Ok(c.query("SELECT name FROM odoo_modules ORDER BY name", &[])?.iter().filter_map(|r| r["name"].as_str().map(String::from)).collect()))?;
        let fresh_db = roots.is_empty();
        if fresh_db { roots = cfg.modules.clone(); }
        let rootrefs: Vec<&str> = roots.iter().map(|s| s.as_str()).collect();
        let reg = Registry::load_dir(&cfg.schema_dir, &rootrefs)?;
        let rules = odoo_modules::rules_for(&reg);
        let installed = store::run(st.as_ref(), |c| Ok(c.query("SELECT 1 FROM res_partner LIMIT 1", &[]).is_ok()))?;
        if !installed {
            let plan = ddl::install_plan(&reg, st.dialect());
            store::apply_plan(st.as_ref(), &plan)?;
            store::run(st.as_ref(), |c| { let env = Env::new(&reg, c, &rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env)?; if cfg.seed { data::seed(&env, &cfg.data_dir, &reg.modules)?; }
                let pw = std::env::var("ODOO_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".into());
                security::set_password(&env, 1, &pw)?; Ok(()) })?;
        }
        // heal drift between the code's schema and an older database (new models/fields after an update)
        if installed { store::sync_schema(st.as_ref(), &reg)?; }
        if fresh_db { store::run(st.as_ref(), |c| { for r in &roots { let p = c.dialect().placeholder(1); c.execute(&format!("INSERT INTO odoo_modules (name) VALUES ({p})"), &[Value::Text(r.clone())])?; } Ok(()) })?; }
        let catalog = Catalog::load(&cfg.data_dir, &reg.modules);
        let sec = Arc::new(Security::new(&reg, &cfg.data_dir, &reg.modules));
        store::run(st.as_ref(), |c| { let env = Env::new(&reg, c, &rules, sec.as_ref(), 1); sec.refresh_groups(&env) })?;
        let app = App { rt: RwLock::new(Arc::new(Runtime { reg, rules, catalog, roots })), install_lock: Mutex::new(()), sec, require_auth: cfg.require_auth, store: st, schema_dir: cfg.schema_dir.clone(), data_dir: cfg.data_dir.clone(), views_dir: cfg.views_dir.clone(), i18n: i18n::I18n::new(cfg.i18n_dir.clone()), seed: cfg.seed,
            portal_sessions: Mutex::new(Default::default()), login_failures: Mutex::new(Default::default()),
            mailer: Mutex::new(Arc::new(email::LiveMailer)), send_log: Mutex::new(Default::default()),
            trial_cfg: cfg.trial.as_ref().map(|c| trial::TrialConfig { dir: c.dir.clone(), max_active: c.max_active, days: c.days }), tenants: RwLock::new(Default::default()), create_lock: Mutex::new(Default::default()), is_trial: cfg.is_trial };
        app.purge_expired_trials();
        Ok(app)
    }

    // ── settings (key/value in the database; the API key is write-only through the API) ──
    fn settings_map(&self) -> Result<std::collections::BTreeMap<String, String>> {
        store::run(self.store.as_ref(), |c| Ok(c.query("SELECT skey, svalue FROM odoo_settings", &[])?.iter().filter_map(|r| Some((r["skey"].as_str()?.to_string(), r["svalue"].as_str().unwrap_or("").to_string()))).collect()))
    }
    pub fn ai_settings(&self) -> Result<ai::Settings> {
        let m = self.settings_map()?;
        let g = |k: &str, d: &str| m.get(k).cloned().unwrap_or_else(|| d.to_string());
        let key = if self.is_trial { g("ai.api_key", "") } else { std::env::var("ODOO_AI_API_KEY").ok().filter(|k| !k.is_empty()).unwrap_or_else(|| g("ai.api_key", "")) };   // trial tenants never borrow the operator's key
        let provider = g("ai.provider", ai::DEFAULT_PROVIDER);
        let model = m.get("ai.model").filter(|v| !v.is_empty()).cloned().unwrap_or_else(|| ai::default_model(&provider).to_string());
        Ok(ai::Settings { enabled: g("ai.enabled", "false") == "true", provider, model, base_url: g("ai.base_url", ""), api_key: key,
            temperature: g("ai.temperature", "0.2").parse().unwrap_or(0.2), system_prompt: g("ai.system_prompt", ""), max_rows: g("ai.max_rows", "25").parse().unwrap_or(25).clamp(1, 200) })
    }
    fn save_settings(&self, vals: &serde_json::Map<String, J>) -> Result<()> {
        const ALLOWED: &[&str] = &["ai.enabled", "ai.provider", "ai.model", "ai.base_url", "ai.api_key", "ai.temperature", "ai.system_prompt", "ai.max_rows",
            "email.enabled", "email.provider", "email.from_address", "email.from_name", "email.reply_to", "email.api_key", "email.domain", "email.region", "email.smtp_host", "email.smtp_port", "email.smtp_security", "email.smtp_user", "email.smtp_password", "email.base_url", "email.app_url"];
        store::run(self.store.as_ref(), |c| {
            for (k, v) in vals {
                if !ALLOWED.contains(&k.as_str()) { return Err(OdooError::Validation(format!("unknown setting `{k}`"))); }
                let s = match v { J::String(s) => s.clone(), J::Bool(b) => b.to_string(), J::Number(n) => n.to_string(), _ => continue };
                if (k == "ai.api_key" || k == "email.api_key" || k == "email.smtp_password") && s.is_empty() { continue; }   // empty = keep the saved key; use "__clear__" to remove it
                let s = if s == "__clear__" { String::new() } else { s };
                if (k == "ai.base_url" || k == "email.base_url" || k == "email.app_url") && !s.is_empty() && !(s.starts_with("http://") || s.starts_with("https://")) { return Err(OdooError::Validation("base URL must start with http:// or https://".into())); }
                let (p1, p2) = (c.dialect().placeholder(1), c.dialect().placeholder(2));
                c.execute(&format!("DELETE FROM odoo_settings WHERE skey = {p1}"), &[Value::Text(k.clone())])?;
                c.execute(&format!("INSERT INTO odoo_settings (skey, svalue) VALUES ({p1}, {p2})"), &[Value::Text(k.clone()), Value::Text(s)])?;
            }
            Ok(())
        })
    }
    pub fn email_settings(&self) -> Result<email::EmailSettings> {
        let m = self.settings_map()?;
        let g = |k: &str, d: &str| m.get(k).cloned().unwrap_or_else(|| d.to_string());
        let env = |name: &str, stored: String| if self.is_trial { stored } else { std::env::var(name).ok().filter(|v| !v.is_empty()).unwrap_or(stored) };   // trial tenants never borrow the operator's credentials
        Ok(email::EmailSettings { enabled: g("email.enabled", "false") == "true", provider: g("email.provider", "sendgrid"), from_address: g("email.from_address", ""), from_name: g("email.from_name", ""), reply_to: g("email.reply_to", ""),
            api_key: env("ODOO_EMAIL_API_KEY", g("email.api_key", "")), domain: g("email.domain", ""), region: g("email.region", "us"), smtp_host: g("email.smtp_host", ""), smtp_port: g("email.smtp_port", "587").parse().unwrap_or(587),
            smtp_security: g("email.smtp_security", "starttls"), smtp_user: g("email.smtp_user", ""), smtp_password: env("ODOO_SMTP_PASSWORD", g("email.smtp_password", "")), base_url: g("email.base_url", ""), app_url: g("email.app_url", "") })
    }

    pub fn set_mailer(&self, m: Arc<dyn email::Mailer>) { *self.mailer.lock().unwrap() = m; }

    /// Send one message with the configured provider: enabled check, settings validation, send-rate limit (30/min), then the transport.
    /// Reads settings itself, so never call this from inside a store transaction (single-connection stores would deadlock);
    /// use `send_email_with` there.
    pub fn send_email(&self, msg: &email::Message) -> Result<String> { let s = self.email_settings()?; self.send_email_with(&s, msg) }

    pub fn send_email_with(&self, s: &email::EmailSettings, msg: &email::Message) -> Result<String> {
        if !s.enabled { return Err(OdooError::User("Email sending is disabled. An administrator can configure it in Settings → Email.".into())); }
        email::validate_settings(s)?; email::validate_message(msg)?;
        { let mut log = self.send_log.lock().unwrap(); let now = std::time::Instant::now(); log.retain(|t| now.duration_since(*t).as_secs() < 60); if log.len() >= 30 { return Err(OdooError::User("Sending too many emails; please wait a minute.".into())); } log.push_back(now); }
        let mailer = self.mailer.lock().unwrap().clone();
        mailer.send(s, msg)
    }

    /// Process `mail.mail` records: send the outgoing ones with the caller's own access rights and record the outcome.
    fn send_mail_records(&self, rt: &Runtime, req: &Request, ids: Option<Vec<i64>>) -> Result<J> {
        let mut report = vec![];
        let settings = self.email_settings()?;   // read BEFORE opening the transaction below
        let results: Vec<(i64, std::result::Result<String, String>)> = {
            let mut collected = vec![];
            store::run(self.store.as_ref(), |c| {
                let env = self.session_env(Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), req.uid).with_record_rules(self.sec.as_ref()), &req);
                let ids = match &ids { Some(v) => v.clone(), None => orm::search(&env, "mail.mail", &Domain::Term("state".into(), "=".into(), Value::Text("outgoing".into())), Some("id"), Some(50), 0)? };
                let fields: Vec<String> = ["email_to", "email_cc", "subject", "body_html", "state"].iter().map(|s| s.to_string()).collect();
                for r in orm::read(&env, "mail.mail", &ids, &fields)? {
                    let id = r["id"].as_i64().unwrap_or(0);
                    if r.get("state").and_then(|v| v.as_str()) != Some("outgoing") { continue; }
                    let txt = |k: &str| r.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let outcome = (|| -> Result<String> {
                        let html = txt("body_html");
                        let msg = email::Message { to: email::parse_addresses(&txt("email_to"))?, cc: email::parse_addresses(&txt("email_cc"))?, bcc: vec![], subject: txt("subject"), text: email::html_to_text(&html), html };
                        self.send_email_with(&settings, &msg)
                    })();
                    collected.push((id, outcome.map_err(|e| e.to_string())));
                }
                Ok(())
            })?;
            collected
        };
        store::run(self.store.as_ref(), |c| {
            let env = self.session_env(Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), req.uid).with_record_rules(self.sec.as_ref()), &req);
            for (id, r) in &results {
                let (state, why) = match r { Ok(_) => ("sent", String::new()), Err(e) => ("exception", e.chars().take(250).collect::<String>()) };
                let mut vals = Row::new(); vals.insert("state".into(), state.into()); vals.insert("failure_reason".into(), if why.is_empty() { Value::Null } else { why.clone().into() });
                orm::write(&env, "mail.mail", &[*id], vals)?;
                report.push(json!({"id": id, "state": state, "error": if why.is_empty() { J::Null } else { json!(why) }}));
            }
            Ok(())
        })?;
        Ok(J::Array(report))
    }

    fn settings_view(&self, rt: &Runtime) -> Result<J> {
        let s = self.ai_settings()?;
        Ok(json!({
            "ai": {"enabled": s.enabled, "provider": s.provider, "model": s.model, "base_url": s.base_url, "has_key": !s.api_key.is_empty(), "temperature": s.temperature, "system_prompt": s.system_prompt, "max_rows": s.max_rows},
            "providers": ai::PROVIDERS.iter().map(|p| json!({"id": p.id, "label": p.label, "base_url": p.base_url, "needs_key": p.needs_key})).collect::<Vec<_>>(),
            "email": self.email_view(),
            "email_providers": email::PROVIDERS.iter().map(|p| json!({"id": p.id, "label": p.label, "kind": if p.kind == email::Kind::Smtp { "smtp" } else { "http" }, "docs": p.docs, "fields": p.fields.iter().map(|f| json!({"id": f.id, "label": f.label, "secret": f.secret, "options": f.options, "placeholder": f.placeholder})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "system": {"backend": self.store.dialect().name(), "modules": rt.reg.modules.len(), "models": rt.reg.models.len(), "version": env!("CARGO_PKG_VERSION"), "auth": self.require_auth},
        }))
    }

    /// Email settings for the UI: secrets are reported only as booleans.
    fn email_view(&self) -> J {
        match self.email_settings() { Ok(s) => json!({"enabled": s.enabled, "provider": s.provider, "from_address": s.from_address, "from_name": s.from_name, "reply_to": s.reply_to, "domain": s.domain, "region": s.region, "smtp_host": s.smtp_host, "smtp_port": s.smtp_port, "smtp_security": s.smtp_security, "smtp_user": s.smtp_user, "base_url": s.base_url, "app_url": s.app_url, "has_api_key": !s.api_key.is_empty(), "has_smtp_password": !s.smtp_password.is_empty()}), Err(_) => J::Null }
    }

    /// Tool execution for the assistant: everything goes through `dispatch` with the caller's own credentials.
    fn ai_tool(&self, rt: &Runtime, base: &Request, max_rows: usize, c: &ai::ToolCall) -> Result<J> {
        let a = &c.args; let model = a["model"].as_str().unwrap_or("");
        let call = |method: &str, args: Vec<J>, kwargs: serde_json::Map<String, J>| self.dispatch(Request { method: method.into(), model: model.into(), args, kwargs, uid: base.uid, token: base.token.clone(), lang: base.lang.clone(), db: None });
        let domain = a.get("domain").cloned().filter(|d| d.is_array()).unwrap_or(json!([]));
        match c.name.as_str() {
            "list_models" => { let q = a["query"].as_str().unwrap_or("").to_lowercase(); Ok(J::Array(rt.reg.models.values().filter(|m| m.has_table() && (m.name.contains(&q) || m.description.as_deref().unwrap_or("").to_lowercase().contains(&q))).take(20).map(|m| json!({"model": m.name, "name": m.description})).collect())) }
            "describe_model" => {
                let f = call("fields_get", vec![], Default::default())?;
                Ok(J::Array(f.as_object().into_iter().flatten().filter(|(_, v)| v["store"] == true || v["relation"].is_string()).take(120).map(|(k, v)| json!({"name": k, "type": v["type"], "label": v["string"], "relation": v["relation"], "selection": v["selection"].as_array().filter(|s| !s.is_empty()).map(|s| s.iter().take(12).map(|p| p[0].clone()).collect::<Vec<_>>())})).collect()))
            }
            "search_read" => { let limit = a["limit"].as_u64().unwrap_or(max_rows as u64).min(max_rows as u64); let mut kw = serde_json::Map::new(); kw.insert("domain".into(), domain); kw.insert("fields".into(), a.get("fields").cloned().unwrap_or(json!([]))); kw.insert("limit".into(), json!(limit)); if let Some(o) = a.get("order") { kw.insert("order".into(), o.clone()); } call("search_read", vec![], kw) }
            "count" => call("search_count", vec![domain], Default::default()),
            "read_group" => call("read_group", vec![domain, a.get("fields").cloned().unwrap_or(json!([])), a["groupby"].clone()], Default::default()),
            other => Err(OdooError::User(format!("unknown tool `{other}`"))),
        }
    }

    fn ai_chat(&self, rt: &Runtime, req: &Request, llm: &dyn ai::Llm) -> Result<J> {
        let s = self.ai_settings()?;
        if !s.enabled { return Err(OdooError::User("The AI assistant is disabled. An administrator can enable it in Settings → AI.".into())); }
        if s.model.is_empty() { return Err(OdooError::User("No model configured. An administrator can set one in Settings → AI.".into())); }
        let history = ai::history_from_json(&req.args.first().cloned().unwrap_or(J::Null));
        if history.is_empty() { return Err(OdooError::Validation("no messages".into())); }
        let ctx = req.kwargs.get("context").cloned().unwrap_or(J::Null);
        // Screen context is pre-loaded as synthetic tool results (untrusted data, same as any tool output): the model sees the
        // model's fields and the open record without having to guess names.
        let mut history = history;
        if let Some(model) = ctx["model"].as_str().filter(|m| rt.reg.models.contains_key(*m)) {
            let mut calls = vec![ai::ToolCall { id: "ctx_fields".into(), name: "describe_model".into(), args: json!({"model": model}) }];
            if let Some(id) = ctx["id"].as_i64() { calls.push(ai::ToolCall { id: "ctx_record".into(), name: "search_read".into(), args: json!({"model": model, "domain": [["id", "=", id]], "limit": 1}) }); }
            let mut results = vec![];
            for c in &calls { if let Ok(out) = self.ai_tool(rt, req, s.max_rows, c) { let mut content = out.to_string(); if content.len() > 6000 { content.truncate(6000); content.push_str("…[truncated]"); } results.push(ai::Msg::Tool { id: c.id.clone(), name: c.name.clone(), content }); } }
            if results.len() == calls.len() { let last = history.pop(); history.push(ai::Msg::Assistant { text: String::new(), calls }); history.extend(results); history.extend(last); }
        }
        let today = orm::today();
        let system = ai::system_prompt(&s, req.lang.as_deref().unwrap_or("en_US"), &today, &ctx);
        ai::run_chat(llm, &s, &system, history, &mut |c| match self.ai_tool(rt, req, s.max_rows, c) {
            // actionable errors: when the model names a field/model that does not exist, tell it what does
            Err(e) if e.to_string().contains("unknown field") || e.to_string().contains("unknown model") => {
                let model = c.args["model"].as_str().unwrap_or("");
                let valid: Vec<String> = rt.reg.models.get(model).map(|m| m.fields.iter().filter(|(_, f)| f.is_stored() || f.is_x2many()).map(|(n, _)| n.clone()).take(80).collect()).unwrap_or_default();
                Ok(json!({"error": e.to_string(), "hint": if valid.is_empty() { "call list_models to find the right model name".to_string() } else { format!("valid fields on {model}: {}", valid.join(", ")) }}))
            }
            other => other,
        })
    }

    /// Same as the `ai_chat` request but with an injected model (tests, alternative transports).
    pub fn ai_chat_with(&self, req: &Request, llm: &dyn ai::Llm) -> Result<J> { let rt = self.runtime(); self.ai_chat(&rt, req, llm) }

    // ── free trials: one isolated database per signup ──
    fn module_available(&self, m: &str) -> bool {
        std::fs::read_to_string(self.schema_dir.join("_manifest.json")).ok().and_then(|s| serde_json::from_str::<J>(&s).ok()).map_or(false, |man| man["modules"].get(m).is_some())
    }

    fn trial_path(&self, db: &str) -> Result<PathBuf> {
        let cfg = self.trial_cfg.as_ref().ok_or_else(|| OdooError::User("Free trials are not enabled on this server.".into()))?;
        if !trial::valid_db(db) { return Err(OdooError::AccessDenied { op: "connect".into(), model: "database".into(), uid: 0 }); }
        Ok(cfg.dir.join(format!("{db}.sqlite")))
    }

    /// The tenant application for a trial database (opened lazily, e.g. after a server restart).
    fn tenant(&self, db: &str) -> Result<Arc<App>> {
        if let Some(t) = self.tenants.read().unwrap().get(db) { return Ok(t.clone()); }
        let path = self.trial_path(db)?;
        if !path.exists() { return Err(OdooError::User("This trial database does not exist (it may have expired).".into())); }
        let app = Arc::new(self.open_tenant(&path, vec!["contacts".into()])?);
        Ok(self.tenants.write().unwrap().entry(db.to_string()).or_insert(app).clone())
    }

    fn open_tenant(&self, path: &std::path::Path, modules: Vec<String>) -> Result<App> {
        App::open(&Config { backend: "sqlite".into(), url: path.to_string_lossy().into(), modules, schema_dir: self.schema_dir.clone(), data_dir: self.data_dir.clone(), views_dir: self.views_dir.clone(), i18n_dir: self.i18n_dir_path(), seed: true, require_auth: true, trial: None, is_trial: true })
    }
    fn i18n_dir_path(&self) -> PathBuf { self.i18n.dir().to_path_buf() }

    pub fn trial_catalog(&self) -> Result<J> {
        if self.trial_cfg.is_none() { return Err(OdooError::User("Free trials are not enabled on this server.".into())); }
        Ok(trial::catalog_json(&|m| self.module_available(m)))
    }

    /// Public signup: validate, create the portal account, provision the first trial database, return a portal session.
    pub fn trial_create(&self, form: &J, lang: &str) -> Result<J> {
        if self.trial_cfg.is_none() { return Err(OdooError::User("Free trials are not enabled on this server.".into())); }
        let sg = trial::parse_signup(form, &|m| self.module_available(m))?;
        self.portal_create_account(&sg)?;
        match self.provision_trial(&sg) {
            Ok((db, token)) => Ok(json!({"portal_token": self.portal_new_session(&sg.email), "db": db, "token": token, "uid": 1, "login": sg.email, "apps": sg.apps, "welcome_email": self.send_welcome(&sg, lang)})),
            Err(e) => { self.portal_delete_account(&sg.email); Err(e) }   // leave nothing behind
        }
    }

    /// Provision one isolated trial database for `sg` (limits enforced here): returns (db name, administrator session token).
    pub(crate) fn provision_trial(&self, sg: &trial::Signup) -> Result<(String, String)> {
        let cfg = self.trial_cfg.as_ref().ok_or_else(|| OdooError::User("Free trials are not enabled on this server.".into()))?;
        // serialise provisioning (it is the expensive operation) and rate-limit it
        let mut recent = self.create_lock.lock().map_err(|_| OdooError::Storage("lock poisoned".into()))?;
        let now = std::time::Instant::now();
        recent.retain(|t| now.duration_since(*t).as_secs() < 60);
        if recent.len() >= 10 { return Err(OdooError::User("Too many signups right now. Please try again in a minute.".into())); }
        let count = store::run(self.store.as_ref(), |c| Ok(c.query("SELECT COUNT(*) AS n FROM odoo_trials", &[])?.first().and_then(|r| r["n"].as_i64()).unwrap_or(0)))?;
        if count as usize >= cfg.max_active { return Err(OdooError::User("Trial capacity has been reached. Please try again later.".into())); }
        recent.push_back(now);
        let db = format!("trial-{}", security::random_hex(8));
        std::fs::create_dir_all(&cfg.dir).map_err(|e| OdooError::Storage(e.to_string()))?;
        let path = self.trial_path(&db)?;
        store::run(self.store.as_ref(), |c| { let p: Vec<String> = (1..=5).map(|i| c.dialect().placeholder(i)).collect(); c.execute(&format!("INSERT INTO odoo_trials (db, email, created_at, company, apps) VALUES ({}, {}, {}, {}, {})", p[0], p[1], p[2], p[3], p[4]), &[Value::Text(db.clone()), Value::Text(sg.email.clone()), Value::Text(orm::now()), Value::Text(sg.company.clone()), Value::Text(json!(sg.apps).to_string())]).map(|_| ()) })?;
        let built = self.open_tenant(&path, trial::modules_for(&sg.apps)).and_then(|app| { let token = provision(&app, sg)?; Ok((app, token)) });
        match built {
            Ok((app, token)) => { self.tenants.write().unwrap().insert(db.clone(), Arc::new(app)); Ok((db, token)) }
            Err(e) => { self.delete_trial(&db); Err(e) }
        }
    }

    /// Remove a trial database completely (files, registry entry, open tenant).
    pub(crate) fn delete_trial(&self, db: &str) {
        let Some(cfg) = &self.trial_cfg else { return };
        if !trial::valid_db(db) { return; }
        self.tenants.write().unwrap().remove(db);
        for ext in ["sqlite", "sqlite-wal", "sqlite-shm"] { let _ = std::fs::remove_file(cfg.dir.join(format!("{db}.{ext}"))); }
        let _ = store::run(self.store.as_ref(), |c| c.execute(&format!("DELETE FROM odoo_trials WHERE db = {}", c.dialect().placeholder(1)), &[Value::Text(db.to_string())]).map(|_| ()));
    }

    /// Best-effort welcome email through the operator's configured provider; never blocks or fails the signup.
    fn send_welcome(&self, sg: &trial::Signup, lang: &str) -> &'static str {
        let Ok(s) = self.email_settings() else { return "not_configured" };
        if !s.enabled || email::validate_settings(&s).is_err() { return "not_configured"; }
        let (subject, text) = email::welcome(lang, &sg.name, &sg.apps, &s.app_url);
        match self.send_email(&email::Message { to: vec![sg.email.clone()], subject, text, ..Default::default() }) { Ok(_) => "sent", Err(_) => "failed" }
    }

    /// Delete trial databases older than the configured lifetime.
    pub fn purge_expired_trials(&self) {
        let Some(cfg) = &self.trial_cfg else { return };
        let cutoff = orm::shift_date(&orm::today(), -(cfg.days as i64));
        let old: Vec<String> = store::run(self.store.as_ref(), |c| { let p = c.dialect().placeholder(1); Ok(c.query(&format!("SELECT db FROM odoo_trials WHERE created_at < {p}"), &[Value::Text(cutoff.clone())])?.iter().filter_map(|r| r["db"].as_str().map(String::from)).collect()) }).unwrap_or_default();
        for db in old { self.delete_trial(&db); }
    }

    pub fn runtime(&self) -> Arc<Runtime> { self.rt.read().unwrap().clone() }

    /// Modules available to install (from the extracted manifest) with their install state.
    pub fn apps_list(&self) -> Result<J> {
        let man: J = serde_json::from_str(&std::fs::read_to_string(self.schema_dir.join("_manifest.json")).map_err(|e| OdooError::Storage(e.to_string()))?).map_err(|e| OdooError::Storage(e.to_string()))?;
        let rt = self.runtime();
        let installed: std::collections::BTreeSet<&String> = rt.reg.modules.iter().collect();
        let mods = man["modules"].as_object().cloned().unwrap_or_default();
        let mut out: Vec<J> = mods.iter().map(|(name, m)| json!({
            "name": name, "title": m["title"].as_str().unwrap_or(name), "summary": m["summary"], "category": m["category"], "application": m["application"],
            "depends": m["depends"], "installed": installed.contains(name), "models": m["models"],
            "missing_deps": m["depends"].as_array().into_iter().flatten().filter_map(|d| d.as_str()).filter(|d| !mods.contains_key(*d)).collect::<Vec<_>>(),
        })).collect();
        out.sort_by(|a, b| (b["application"].as_bool(), a["title"].as_str()).partial_cmp(&(a["application"].as_bool(), b["title"].as_str())).unwrap_or(std::cmp::Ordering::Equal));
        Ok(J::Array(out))
    }

    /// Install modules (and their dependencies) into the live database: schema upgrade, seed data, ACL/rules/menus reload.
    pub fn install(&self, names: &[String]) -> Result<Vec<String>> {
        let _g = self.install_lock.lock().map_err(|_| OdooError::Storage("install lock poisoned".into()))?;
        let old = self.runtime();
        let man: J = serde_json::from_str(&std::fs::read_to_string(self.schema_dir.join("_manifest.json")).map_err(|e| OdooError::Storage(e.to_string()))?).map_err(|e| OdooError::Storage(e.to_string()))?;
        for n in names { if man["modules"].get(n).is_none() { return Err(OdooError::User(format!("unknown module `{n}`"))); } }
        let mut roots = old.roots.clone();
        for n in names { if !roots.contains(n) { roots.push(n.clone()); } }
        let rootrefs: Vec<&str> = roots.iter().map(|s| s.as_str()).collect();
        let reg = Registry::load_dir(&self.schema_dir, &rootrefs)?;
        let added: Vec<String> = reg.modules.iter().filter(|m| !old.reg.modules.contains(m)).cloned().collect();
        if added.is_empty() { return Ok(vec![]); }
        let plan = ddl::upgrade_plan(&old.reg, &reg, self.store.dialect());
        store::apply_upgrade(self.store.as_ref(), &plan)?;
        let rules = odoo_modules::rules_for(&reg);
        if self.seed {
            store::run(self.store.as_ref(), |c| { let env = Env::new(&reg, c, &rules, &AllowAll, 1); odoo_modules::bootstrap::run(&env)?; data::seed(&env, &self.data_dir, &added)?; Ok(()) })?;
        }
        store::run(self.store.as_ref(), |c| { for n in names { let p = c.dialect().placeholder(1); let exists = c.query(&format!("SELECT 1 FROM odoo_modules WHERE name = {p}"), &[Value::Text(n.clone())])?; if exists.is_empty() { c.execute(&format!("INSERT INTO odoo_modules (name) VALUES ({p})"), &[Value::Text(n.clone())])?; } } Ok(()) })?;
        self.sec.reload(&reg, &self.data_dir, &reg.modules);
        let catalog = Catalog::load(&self.data_dir, &reg.modules);
        store::run(self.store.as_ref(), |c| { let env = Env::new(&reg, c, &rules, self.sec.as_ref(), 1); self.sec.refresh_groups(&env) })?;
        *self.rt.write().unwrap() = Arc::new(Runtime { reg, rules, catalog, roots });
        Ok(added)
    }

    /// Scope an environment to the companies its session has switched to (`company_id` = active, `allowed_company_ids`).
    fn session_env<'a>(&self, env: Env<'a>, req: &Request) -> Env<'a> {
        match req.token.as_deref().and_then(|t| self.sec.session(t)) {
            Some(s) => env.with_ctx("company_id", Value::Int(s.company_id)).with_ctx("allowed_company_ids", Value::List(s.allowed.iter().map(|c| Value::Int(*c)).collect())),
            None => env,
        }
    }

    /// The companies the caller may switch between, and the ones its session currently works in.
    fn company_state(&self, rt: &Runtime, req: &Request) -> Result<J> {
        let Some(sess) = req.token.as_deref().and_then(|t| self.sec.session(t)) else { return Ok(json!({"current": J::Null, "allowed": [], "companies": []})) };
        let mut out = J::Null;
        store::run(self.store.as_ref(), |c| {
            let env = Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), 1);
            let (_, mine) = self.sec.user_companies(&env, sess.uid)?;
            let rows = orm::read(&env, "res.company", &mine, &["name".to_string()])?;
            let list: Vec<J> = mine.iter().map(|id| json!({"id": id, "name": rows.iter().find(|r| r["id"].as_i64() == Some(*id)).and_then(|r| r["name"].as_str()).unwrap_or("")})).collect();
            out = json!({"current": sess.company_id, "allowed": sess.allowed, "companies": list}); Ok(())
        })?;
        Ok(out)
    }

    pub fn dispatch(&self, mut req: Request) -> Result<J> {
        if req.method.starts_with("portal_") { return self.portal_dispatch(&req); }   // portal sessions are separate from database sessions
        // a request for a trial database is served entirely by that tenant (its own data, users and sessions)
        if let Some(db) = req.db.take() { return self.tenant(&db)?.dispatch(req); }
        match req.method.as_str() { "trial_catalog" => return self.trial_catalog(), "trial_create" => return self.trial_create(&req.args.first().cloned().unwrap_or(J::Null), req.lang.as_deref().unwrap_or("en")), _ => {} }
        let rt = self.runtime();
        match req.method.as_str() {
            "login" => {
                let (l, p) = (req.args.first().and_then(|a| a.as_str()).unwrap_or("").to_string(), req.args.get(1).and_then(|a| a.as_str()).unwrap_or("").to_string());
                let mut out = J::Null;
                store::run(self.store.as_ref(), |c| { let env = Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), 1); out = match self.sec.login(&env, &l, &p)? { Some((t, u)) => json!({"token": t, "uid": u}), None => J::Null }; Ok(()) })?;
                return if out.is_null() { Err(OdooError::User("Wrong login/password".into())) } else { Ok(out) };
            }
            "logout" => { if let Some(t) = &req.token { self.sec.logout(t); } return Ok(J::Bool(true)); }
            "whoami" => { return Ok(match req.token.as_deref().and_then(|t| self.sec.uid_of(t)) { Some(u) => json!({"uid": u}), None => if self.require_auth { J::Null } else { json!({"uid": 1}) } }); }
            _ => {}
        }
        if self.require_auth {
            req.uid = req.token.as_deref().and_then(|t| self.sec.uid_of(t)).ok_or_else(|| OdooError::AccessDenied { op: "session".into(), model: req.model.clone(), uid: 0 })?;
        } else if req.token.is_some() { if let Some(u) = req.token.as_deref().and_then(|t| self.sec.uid_of(t)) { req.uid = u; } }
        match req.method.as_str() {
            "apps_list" => return self.apps_list(),
            "companies" => return self.company_state(&rt, &req),
            "switch_company" => {
                let ids: Vec<i64> = req.args.first().and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|v| v.as_i64()).collect()).unwrap_or_default();
                let tok = req.token.clone().unwrap_or_default();
                store::run(self.store.as_ref(), |c| { let env = Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), 1); self.sec.switch_company(&env, &tok, &ids).map(|_| ()) })?;
                return self.company_state(&rt, &req);
            }
            "email_test" | "send_mail" => {
                if req.uid != 1 { return Err(OdooError::AccessDenied { op: "send".into(), model: "email".into(), uid: req.uid }); }
                let o = req.args.first().cloned().unwrap_or(J::Null);
                let list = |k: &str| email::parse_addresses(o[k].as_str().unwrap_or(""));
                let msg = if req.method == "email_test" {
                    email::Message { to: list("to")?, subject: "Odoo RS test email".into(), text: "This is a test email from Odoo RS. If you can read this, outgoing email is configured correctly.".into(), ..Default::default() }
                } else { email::Message { to: list("to")?, cc: list("cc")?, bcc: list("bcc")?, subject: o["subject"].as_str().unwrap_or("").to_string(), text: o["text"].as_str().unwrap_or("").to_string(), html: o["html"].as_str().unwrap_or("").to_string() } };
                return Ok(json!({"ok": true, "id": self.send_email(&msg)?}));
            }
            "process_email_queue" => { if req.uid != 1 { return Err(OdooError::AccessDenied { op: "send".into(), model: "mail.mail".into(), uid: req.uid }); } return self.send_mail_records(&rt, &req, None); }
            "send" if req.model == "mail.mail" => { let ids: Vec<i64> = req.args.first().and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|i| i.as_i64()).collect()).unwrap_or_default(); return self.send_mail_records(&rt, &req, Some(ids)); }
            "settings_get" => return self.settings_view(&rt),
            "settings_set" => {
                if req.uid != 1 { return Err(OdooError::AccessDenied { op: "write".into(), model: "settings".into(), uid: req.uid }); }
                let m = req.args.first().and_then(|a| a.as_object()).cloned().ok_or_else(|| OdooError::Validation("settings object expected".into()))?;
                self.save_settings(&m)?; return self.settings_view(&rt);
            }
            "ai_test" => {
                if req.uid != 1 { return Err(OdooError::AccessDenied { op: "write".into(), model: "settings".into(), uid: req.uid }); }
                let s = self.ai_settings()?;
                let r = ai::Llm::complete(&ai::HttpLlm, "Reply with the single word OK.", &[ai::Msg::User("ping".into())], &json!([]), &s)?;
                return Ok(json!({"ok": true, "reply": r.text.chars().take(80).collect::<String>()}));
            }
            "ai_models" => {
                if req.uid != 1 { return Err(OdooError::AccessDenied { op: "read".into(), model: "settings".into(), uid: req.uid }); }
                let mut s = self.ai_settings()?;
                // allow probing a not-yet-saved provider/base URL/key from the form
                if let Some(o) = req.args.first().and_then(|a| a.as_object()) { if let Some(p) = o.get("provider").and_then(|v| v.as_str()) { s.provider = p.into(); } if let Some(b) = o.get("base_url").and_then(|v| v.as_str()) { s.base_url = b.into(); } if let Some(k) = o.get("api_key").and_then(|v| v.as_str()).filter(|k| !k.is_empty()) { s.api_key = k.into(); } }
                return Ok(json!(ai::HttpLlm::list_models(&s)?));
            }
            "ai_chat" => return self.ai_chat(&rt, &req, &ai::HttpLlm),
            "install_module" => {
                if req.uid != 1 { return Err(OdooError::AccessDenied { op: "install".into(), model: "ir.module.module".into(), uid: req.uid }); }
                let names: Vec<String> = req.args.iter().filter_map(|a| a.as_str().map(String::from)).collect();
                let added = self.install(&names)?;
                return Ok(json!({"installed": added}));
            }
            "menus" => { let mut tree = rt.catalog.menu_tree(&rt.reg); if let Some(c) = req.lang.as_deref().and_then(|l| self.i18n.get(l)) { c.menus(&mut tree); } return Ok(tree); }
            "models" => return Ok(J::Array(rt.reg.models.values().filter(|m| m.has_table()).map(|m| json!({"model": m.name, "name": m.description, "module": m.module})).collect())),
            "fields_get" => {
                let mut f = orm::fields_get(&rt.reg, &req.model)?;
                if let Some(c) = req.lang.as_deref().and_then(|l| self.i18n.get(l)) { let parents = rt.reg.models.get(&req.model).map(|m| m.inherit.clone()).unwrap_or_default(); c.fields_get(&req.model, &parents, &mut f); }
                return Ok(f);
            }
            "action" => { let k = req.args.first().and_then(|a| a.as_str()).unwrap_or(""); return Ok(rt.catalog.actions.get(k).cloned().unwrap_or(J::Null)); }
            "get_view" => {
                // merged arch (form|list|kanban|search) for the model, or null -> UI falls back to a generated view
                if req.model.contains(['/', '\\']) || req.model.contains("..") { return Err(OdooError::Validation("bad model name".into())); }
                let kind = req.args.first().and_then(|a| a.as_str()).unwrap_or("form");
                let v: J = std::fs::read_to_string(self.views_dir.join(format!("{}.json", req.model))).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(J::Null);
                let mut out = v.get(kind).cloned().unwrap_or(J::Null);
                prune_uninstalled(&mut out, &rt.reg.modules);
                if let Some(c) = req.lang.as_deref().and_then(|l| self.i18n.get(l)) { c.view(&mut out); }
                return Ok(out);
            }
            "ported" => { let mut v: Vec<&str> = rt.rules.actions.keys().filter(|(m, _)| *m == req.model).map(|(_, n)| n.as_str()).collect(); v.extend(["copy", "exists", "name_create"]); if rt.reg.model(&req.model).map_or(false, |m| m.fields.get("active").map_or(false, |f| f.is_stored())) { v.extend(["action_archive", "action_unarchive", "toggle_active"]); } v.sort(); v.dedup(); return Ok(json!(v)); }
            "methods" => { let m = rt.reg.model(&req.model)?; return Ok(J::Array(m.methods.iter().filter(|x| x.action).map(|x| json!(x.name)).collect())); }
            _ => {}
        }
        let mut out = J::Null;
        store::run(self.store.as_ref(), |c| {
            let env = self.session_env(Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), req.uid).with_record_rules(self.sec.as_ref()), &req);
            out = self.call(&env, &req)?;
            if req.model == "res.users" || req.model == "res.groups" { self.sec.refresh_groups(&env)?; }
            Ok(())
        })?;
        Ok(out)
    }

    fn call(&self, env: &Env, req: &Request) -> Result<J> {
        let a = |i: usize| req.args.get(i).cloned().unwrap_or(J::Null);
        let kw = |k: &str| req.kwargs.get(k).cloned();
        let dom = |v: J| if v.is_null() { Ok(Domain::True) } else { Domain::parse(&v) };
        let ids = |v: J| -> Vec<i64> { v.as_array().map(|x| x.iter().filter_map(|i| i.as_i64()).collect()).unwrap_or_default() };
        let strs = |v: J| -> Vec<String> { v.as_array().map(|x| x.iter().filter_map(|i| i.as_str().map(String::from)).collect()).unwrap_or_default() };
        let vals = |v: J| -> Row { match Value::from_json(&v) { Value::Map(m) => m.into_iter().collect(), _ => Row::new() } };
        let rows = |r: Vec<Row>| J::Array(r.iter().map(|x| Value::Map(x.clone().into_iter().collect()).to_json()).collect());
        let m = req.model.as_str();
        Ok(match req.method.as_str() {
            "search" => json!(orm::search(env, m, &dom(a(0))?, kw("order").and_then(|o| o.as_str().map(String::from)).as_deref(), kw("limit").and_then(|l| l.as_u64()).map(|l| l as usize), kw("offset").and_then(|l| l.as_u64()).unwrap_or(0) as usize)?),
            "search_count" => json!(orm::search_count(env, m, &dom(a(0))?)?),
            "search_read" => rows(orm::search_read(env, m, &dom(kw("domain").unwrap_or(a(0)))?, &strs(kw("fields").unwrap_or(a(1))), kw("order").and_then(|o| o.as_str().map(String::from)).as_deref(), kw("limit").and_then(|l| l.as_u64()).map(|l| l as usize), kw("offset").and_then(|l| l.as_u64()).unwrap_or(0) as usize)?),
            "read" => rows(orm::read(env, m, &ids(a(0)), &strs(a(1)))?),
            "create" => match a(0) { J::Array(list) => json!(list.into_iter().map(|v| orm::create(env, m, vals(v))).collect::<Result<Vec<_>>>()?), v => json!(orm::create(env, m, vals(v))?) },
            "write" => { orm::write(env, m, &ids(a(0)), vals(a(1)))?; json!(true) }
            "change_password" if m == "res.users" => { let pw = a(1).as_str().unwrap_or("").to_string(); if pw.len() < 8 { return Err(OdooError::User("password must be at least 8 characters".into())); } let uid = a(0).as_i64().unwrap_or(0); if env.uid != 1 && env.uid != uid { return Err(OdooError::AccessDenied { op: "write".into(), model: m.into(), uid: env.uid }); } security::set_password(&env, uid, &pw)?; json!(true) }
            "default_get" => rows(vec![orm::default_get(env, m, &strs(a(0)))?]).as_array().and_then(|x| x.first().cloned()).unwrap_or(J::Null),
            "onchange" => Value::Map(orm::onchange(env, m, vals(a(0)))?.into_iter().collect()).to_json(),
            "unlink" => { orm::unlink(env, m, &ids(a(0)))?; json!(true) }
            "name_search" => json!(orm::name_search(env, m, a(0).as_str().unwrap_or(""), kw("limit").and_then(|l| l.as_u64()).unwrap_or(8) as usize)?),
            "read_group" => rows(orm::read_group(env, m, &dom(a(0))?, &strs(a(1)), a(2).as_str().unwrap_or("id"), kw("orderby").and_then(|o| o.as_str().map(String::from)).as_deref())?),
            other => orm::call(env, m, other, &ids(a(0)), &vals(kw("kwargs").unwrap_or(a(1))))?.to_json(),
        })
    }
}


/// Make the signup the administrator of their fresh database and return a session token.
fn provision(app: &App, sg: &trial::Signup) -> Result<String> {
    let rt = app.runtime();
    store::run(app.store.as_ref(), |c| {
        let env = Env::new(&rt.reg, c, &rt.rules, &AllowAll, 1);
        let row = |p: &[(&str, Value)]| -> Row { p.iter().map(|(k, v)| (k.to_string(), v.clone())).collect() };
        orm::write(&env, "res.users", &[1], row(&[("login", sg.email.clone().into()), ("name", sg.name.clone().into())]))?;
        let pid = orm::read(&env, "res.users", &[1], &["partner_id".to_string()])?.first().and_then(|r| match r.get("partner_id") { Some(Value::List(l)) => l.first().and_then(|v| v.as_i64()), _ => None });
        if let Some(pid) = pid {
            let mut v = row(&[("email", sg.email.clone().into())]); if !sg.phone.is_empty() { v.insert("phone".into(), sg.phone.clone().into()); }
            orm::write(&env, "res.partner", &[pid], v)?;
        }
        orm::write(&env, "res.company", &[1], row(&[("name", if sg.company.is_empty() { sg.name.clone() } else { sg.company.clone() }.into())]))?;
        // a company's address lives on its partner record (res.company.country_id is computed from it)
        if !sg.country.is_empty() {
            let cpid = orm::read(&env, "res.company", &[1], &["partner_id".to_string()])?.first().and_then(|r| match r.get("partner_id") { Some(Value::List(l)) => l.first().and_then(|v| v.as_i64()), _ => None });
            if let (Some(cpid), Some(cid)) = (cpid, orm::search(&env, "res.country", &Domain::Term("code".into(), "=ilike".into(), Value::Text(sg.country.clone())), None, Some(1), 0)?.first()) {
                orm::write(&env, "res.partner", &[cpid], row(&[("country_id", (*cid).into())]))?;
            }
        }
        security::set_password(&env, 1, &sg.password)
    })?;
    let mut token = None;
    store::run(app.store.as_ref(), |c| { let env = Env::new(&rt.reg, c, &rt.rules, app.sec.as_ref(), 1); token = app.sec.login(&env, &sg.email, &sg.password)?.map(|(t, _)| t); Ok(()) })?;
    token.ok_or_else(|| OdooError::Storage("could not open a session for the new database".into()))
}
