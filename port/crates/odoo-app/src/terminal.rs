//! Card-terminal payments for the POS: a provider-neutral start/status/cancel/refund protocol with a built-in simulator and
//! server-driven Stripe Terminal, Adyen, Mercado Pago, Viva Wallet, Razorpay, PayTM and Pine Labs drivers (see `drivers.rs`).
//! A completed payment is written to a ledger (`ir.config_parameter`, key `pos.terminal.<tx>`); the POS only accepts a
//! terminal payment line when the matching ledger entry exists for the same method and amount, and consumes it.
use odoo_core::{OdooError, Result};
use serde_json::{json, Value as J};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

pub mod drivers;
pub use drivers::{adyen_charge, can_refund, cancel, parse_adyen, poll, refund, start};

pub struct HttpReq { pub method: &'static str, pub url: String, pub headers: Vec<(String, String)>, pub form: Vec<(String, String)>, pub json: Option<J>, pub raw: Option<String>, pub timeout_s: u64 }
pub trait Http: Send + Sync { fn send(&self, r: &HttpReq) -> Result<(u16, String)>; }

pub struct UreqHttp;
impl Http for UreqHttp {
    fn send(&self, r: &HttpReq) -> Result<(u16, String)> {
        let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(r.timeout_s)).build();
        let mut q = agent.request(r.method, &r.url);
        for (k, v) in &r.headers { q = q.set(k, v); }
        let res = if let Some(raw) = &r.raw { q.send_string(raw) } else if let Some(j) = &r.json { q.send_json(j.clone()) } else if r.method == "GET" || r.method == "DELETE" { q.call() } else { q.send_form(&r.form.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect::<Vec<_>>()) };
        match res { Ok(x) => Ok((x.status(), x.into_string().unwrap_or_default())), Err(ureq::Error::Status(c, x)) => Ok((c, x.into_string().unwrap_or_default())), Err(e) => Err(OdooError::User(format!("Payment terminal service unreachable: {e}"))) }
    }
}

/// Provider result of one poll / blocking call.
#[derive(Clone, Debug, Default)]
pub struct Out { pub status: String, pub message: String, pub brand: String, pub last4: String, pub reference: String }
impl Out {
    pub fn ok(brand: &str, last4: &str, reference: &str) -> Out { Out { status: "succeeded".into(), brand: brand.into(), last4: last4.into(), reference: reference.into(), ..Default::default() } }
    pub fn fail(msg: &str) -> Out { Out { status: "failed".into(), message: msg.into(), ..Default::default() } }
}

#[derive(Clone, Debug, Default)]
pub struct Tx { pub provider: String, pub method: i64, pub cents: i64, pub status: String, pub message: String, pub brand: String, pub last4: String, pub reference: String, pub refref: String, pub created: Option<Instant>, pub aux: BTreeMap<String, String>, pub recorded: bool }
impl Tx { pub fn view(&self, id: &str) -> J { json!({"tx": id, "status": self.status, "message": self.message, "card_brand": self.brand, "card_no": self.last4, "transaction_id": id, "provider": self.provider, "refundable": can_refund(&self.provider)}) } }

#[derive(Clone, Debug)]
pub struct Creds { pub stripe_key: String, pub stripe_base: String, pub adyen_base: String, pub mp_base: String, pub viva_accounts: Option<String>, pub viva_api: Option<String>, pub razorpay_base: Option<String>, pub paytm_base: Option<String>, pub pine_base: Option<String> }
impl Creds {
    pub fn secrets(&self) -> Vec<String> { vec![self.stripe_key.clone()] }
    pub fn new(stripe_key: &str, stripe_base: &str, adyen_base: &str) -> Creds { Creds { stripe_key: stripe_key.into(), stripe_base: stripe_base.into(), adyen_base: adyen_base.into(), mp_base: "https://api.mercadopago.com".into(), viva_accounts: None, viva_api: None, razorpay_base: None, paytm_base: None, pine_base: None } }
}

/// What the payment method record says about its terminal: the provider and every credential/option field it carries.
#[derive(Clone, Debug, Default)]
pub struct MethodCfg { pub provider: String, pub f: BTreeMap<String, String>, pub test: bool }
impl MethodCfg {
    pub fn new(provider: &str, pairs: &[(&str, &str)]) -> MethodCfg { let mut m = MethodCfg { provider: provider.into(), f: pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(), test: false }; m.test = m.test_flag(); m }
    pub fn g(&self, k: &str) -> &str { self.f.get(k).map(|s| s.as_str()).unwrap_or("") }
    pub fn secrets(&self) -> Vec<String> { self.f.iter().filter(|(k, _)| ["key", "secret", "token", "password"].iter().any(|w| k.contains(w))).map(|(_, v)| v.clone()).filter(|v| !v.is_empty()).collect() }
    fn test_flag(&self) -> bool { ["adyen_test_mode", "viva_wallet_test_mode", "razorpay_test_mode", "paytm_test_mode", "pine_labs_test_mode"].iter().any(|k| matches!(self.g(k), "true" | "1")) }
}
/// Fields of `pos.payment.method` the drivers read (those absent from the registry are skipped).
pub const METHOD_FIELDS: &[&str] = &["use_payment_terminal", "stripe_serial_number", "adyen_api_key", "adyen_terminal_identifier", "adyen_test_mode", "mp_bearer_token", "mp_id_point_smart",
    "viva_wallet_client_id", "viva_wallet_client_secret", "viva_wallet_terminal_id", "viva_wallet_test_mode", "razorpay_tid", "razorpay_username", "razorpay_api_key", "razorpay_test_mode", "razorpay_allowed_payment_modes",
    "paytm_mid", "paytm_tid", "paytm_merchant_key", "paytm_test_mode", "channel_id", "accept_payment", "allowed_payment_modes", "pine_labs_merchant", "pine_labs_store", "pine_labs_client", "pine_labs_security_token", "pine_labs_allowed_payment_mode", "pine_labs_test_mode"];

pub fn iso_now() -> String { let s = odoo_core::orm::now(); format!("{}T{}Z", &s[..10], &s[11..19]) }

// ───────────── App integration ─────────────
use crate::{security::random_hex, App, Request, Runtime};
use odoo_core::{orm::{self, Env}, store, Value};
use std::sync::{Arc, Mutex};

/// One in-flight terminal transaction (+ the worker slot for providers whose API blocks).
pub struct Slot { pub tx: Tx, pub cfg: MethodCfg, pub worker: Option<Arc<Mutex<Option<Out>>>> }

impl App {
    fn settings_str(&self, k: &str, d: &str) -> String { self.settings_map().ok().and_then(|m| m.get(k).cloned()).filter(|v| !v.is_empty()).unwrap_or_else(|| d.to_string()) }
    fn terminal_creds(&self, test: bool) -> Creds {
        let mut c = Creds::new(&self.settings_str("pos.stripe_secret_key", ""), self.settings_str("pos.stripe_base_url", "https://api.stripe.com").trim_end_matches('/'),
            self.settings_str("pos.adyen_base_url", if test { "https://terminal-api-test.adyen.com" } else { "https://terminal-api-live.adyen.com" }).trim_end_matches('/'));
        let o = |k: &str| Some(self.settings_str(k, "")).filter(|v| !v.is_empty());
        c.mp_base = self.settings_str("pos.mp_base_url", "https://api.mercadopago.com").trim_end_matches('/').into();
        c.viva_accounts = o("pos.viva_accounts_url"); c.viva_api = o("pos.viva_api_url"); c.razorpay_base = o("pos.razorpay_base_url"); c.paytm_base = o("pos.paytm_base_url"); c.pine_base = o("pos.pine_labs_base_url"); c
    }
    pub(crate) fn terminal_view(&self) -> J { json!({"stripe_has_key": !self.settings_str("pos.stripe_secret_key", "").is_empty(), "stripe_base_url": self.settings_str("pos.stripe_base_url", ""), "adyen_base_url": self.settings_str("pos.adyen_base_url", ""), "pine_labs_base_url": self.settings_str("pos.pine_labs_base_url", "")}) }

    fn method_cfg(&self, rt: &Runtime, req: &Request, id: i64) -> Result<MethodCfg> {
        let fields: Vec<String> = METHOD_FIELDS.iter().filter(|f| rt.reg.field("pos.payment.method", f).is_ok()).map(|f| f.to_string()).collect();
        let mut out = MethodCfg::default();
        store::run(self.store.as_ref(), |c| {
            let env = Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), req.uid);
            let r = orm::read(&env, "pos.payment.method", &[id], &fields)?; let r = r.first().ok_or_else(|| OdooError::User("Unknown payment method".into()))?;
            for (k, v) in r { let sv = match v { Value::Text(t) => t.clone(), Value::Bool(b) => b.to_string(), Value::Int(i) => i.to_string(), _ => String::new() }; if !sv.is_empty() && sv != "false" || k.ends_with("_mode") { out.f.insert(k.clone(), sv); } }
            Ok(())
        })?;
        out.provider = out.g("use_payment_terminal").to_string(); out.test = out.test_flag();
        if out.provider.is_empty() { return Err(OdooError::User("This payment method has no terminal configured".into())); }
        Ok(out)
    }

    /// Write a ledger entry (`method|cents|brand|last4|ref`) the POS requires before accepting a terminal payment line.
    fn ledger_write(&self, rt: &Runtime, id: &str, method: i64, cents: i64, brand: &str, last4: &str, reference: &str) -> Result<()> {
        store::run(self.store.as_ref(), |c| {
            let env = Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), 1).sudo();
            let mut v = odoo_core::Row::new(); v.insert("key".into(), Value::Text(format!("pos.terminal.{id}"))); v.insert("value".into(), Value::Text(format!("{method}|{cents}|{brand}|{last4}|{reference}")));
            orm::create(&env, "ir.config_parameter", v).map(|_| ())
        })
    }
    fn terminal_record(&self, rt: &Runtime, id: &str, slot: &mut Slot) -> Result<()> {
        if slot.tx.recorded || slot.tx.status != "succeeded" { return Ok(()); }
        self.ledger_write(rt, id, slot.tx.method, slot.tx.cents, &slot.tx.brand, &slot.tx.last4, &slot.tx.refref)?; slot.tx.recorded = true; Ok(())
    }

    pub(crate) fn terminal_dispatch(&self, rt: &Runtime, req: &Request) -> Result<J> {
        let http = self.terminal_http.lock().unwrap().clone();
        let a = req.args.first().cloned().unwrap_or(J::Null);
        match req.method.as_str() {
            "terminal_start" => {
                let mid = a["method_id"].as_i64().ok_or_else(|| OdooError::User("Select a payment method".into()))?;
                let cents = (a["amount"].as_f64().unwrap_or(0.0) * 100.0).round() as i64; let cur = a["currency"].as_str().unwrap_or("usd").to_string(); let reference = a["reference"].as_str().unwrap_or("POS").chars().take(60).collect::<String>();
                let who = a["cashier"].as_str().unwrap_or("").chars().take(40).collect::<String>();
                let cfg = self.method_cfg(rt, req, mid)?; let id = random_hex(16); let creds = self.terminal_creds(cfg.test);
                let mut tx = start(http.as_ref(), &creds, &cfg, &id, cents, &cur, &reference, &who)?; tx.method = mid;
                let mut worker = None;
                if cfg.provider == "adyen" {
                    let slot: Arc<Mutex<Option<Out>>> = Arc::new(Mutex::new(None)); worker = Some(slot.clone());
                    let (h, c2, base, svc) = (http.clone(), cfg.clone(), creds.adyen_base.clone(), tx.aux.get("service").cloned().unwrap_or_default());
                    std::thread::spawn(move || { let r = adyen_charge(h.as_ref(), &c2, &base, &svc, cents, &cur, &reference); *slot.lock().unwrap() = Some(r); });
                }
                let v = tx.view(&id); self.terminals.lock().unwrap().insert(id, Slot { tx, cfg, worker }); Ok(v)
            }
            "terminal_status" | "terminal_cancel" => {
                let id = a["tx"].as_str().unwrap_or("").to_string();
                let mut map = self.terminals.lock().unwrap();
                let slot = map.get_mut(&id).ok_or_else(|| OdooError::User("Unknown transaction".into()))?;
                let creds = self.terminal_creds(slot.cfg.test);
                if req.method == "terminal_cancel" { let cfg = slot.cfg.clone(); cancel(http.as_ref(), &creds, &cfg, &mut slot.tx, &id); }
                else {
                    let cfg = slot.cfg.clone(); poll(http.as_ref(), &creds, &cfg, &mut slot.tx);
                    if let Some(w) = &slot.worker { if let Some(o) = w.lock().unwrap().clone() { if slot.tx.status == "pending" { slot.tx.status = o.status; slot.tx.message = o.message; slot.tx.brand = o.brand; slot.tx.last4 = o.last4; slot.tx.refref = o.reference; } } }
                    self.terminal_record(rt, &id, slot)?;
                }
                let v = slot.tx.view(&id);
                if map.len() > 500 { let keep: Vec<String> = map.iter().filter(|(_, s)| s.tx.created.map_or(false, |t| t.elapsed() < Duration::from_secs(3600))).map(|(k, _)| k.clone()).collect(); map.retain(|k, _| keep.contains(k)); }
                Ok(v)
            }
            // Return money for a card payment: through the provider API when it has one, otherwise confirmed manually (done on the device/dashboard).
            "terminal_refund" => {
                let pid = a["payment_id"].as_i64().ok_or_else(|| OdooError::User("Select the payment to refund".into()))?;
                let cents = (a["amount"].as_f64().unwrap_or(0.0).abs() * 100.0).round() as i64; let manual = a["manual"].as_bool().unwrap_or(false); let cur = a["currency"].as_str().unwrap_or("usd").to_string();
                let (mid, reference, original) = { let mut r = (0i64, String::new(), 0i64);
                    store::run(self.store.as_ref(), |c| { let env = Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), req.uid);
                        let p = orm::read(&env, "pos.payment", &[pid], &["payment_method_id".into(), "payment_ref_no".into(), "amount".into()])?; let p = p.first().ok_or_else(|| OdooError::User("Unknown payment".into()))?;
                        r = (match p.get("payment_method_id") { Some(Value::Int(i)) => *i, Some(Value::List(l)) => l.first().and_then(|v| v.as_i64()).unwrap_or(0), _ => 0 }, p.get("payment_ref_no").and_then(|v| v.as_str()).unwrap_or("").to_string(), (p.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0).round() as i64); Ok(()) })?; r };
                if cents > original { return Err(OdooError::User("Cannot refund more than was paid".into())); }
                let cfg = self.method_cfg(rt, req, mid)?; let creds = self.terminal_creds(cfg.test);
                let (brand, last4, rref) = if manual {
                    if can_refund(&cfg.provider) { return Err(OdooError::User("This terminal refunds through its API — manual confirmation is not allowed".into())); }
                    (String::new(), String::new(), "manual".to_string())
                } else { (String::new(), String::new(), refund(http.as_ref(), &creds, &cfg, &reference, cents, &cur)?) };
                let id = random_hex(16); self.ledger_write(rt, &id, mid, -cents, &brand, &last4, &rref)?;
                Ok(json!({"tx": id, "status": "succeeded", "transaction_id": id, "provider": cfg.provider, "manual": manual}))
            }
            // Cheap credential check shown in Settings / the payment method form.
            "terminal_test" => {
                let mid = a["method_id"].as_i64().ok_or_else(|| OdooError::User("Select a payment method".into()))?;
                let cfg = self.method_cfg(rt, req, mid)?; let creds = self.terminal_creds(cfg.test);
                match cfg.provider.as_str() {
                    "simulator" => Ok(json!({"ok": true, "message": "Simulator ready"})),
                    "stripe" => {
                        if creds.stripe_key.is_empty() { return Ok(json!({"ok": false, "message": "Stripe secret key is not set"})); }
                        let (st, b) = http.send(&HttpReq { method: "GET", url: format!("{}/v1/terminal/readers?limit=100", creds.stripe_base), headers: drivers::bearer(&creds.stripe_key), form: vec![], json: None, raw: None, timeout_s: 15 })?;
                        if st >= 300 { return Ok(json!({"ok": false, "message": "Stripe rejected the key"})); }
                        let j: J = serde_json::from_str(&b).unwrap_or(J::Null); let readers: Vec<J> = j["data"].as_array().cloned().unwrap_or_default();
                        let want = cfg.g("stripe_serial_number");
                        let found = readers.iter().find(|r| r["serial_number"] == want || r["id"] == want);
                        Ok(json!({"ok": found.is_some(), "message": match found { Some(r) => format!("Reader {} is {}", r["label"].as_str().unwrap_or(want), r["status"].as_str().unwrap_or("?")), None => format!("Key accepted, but no reader matches {want} ({} readers on the account)", readers.len()) }}))
                    }
                    "mercado_pago" => { let (st, b) = http.send(&HttpReq { method: "GET", url: format!("{}/point/integration-api/devices", creds.mp_base), headers: drivers::bearer(cfg.g("mp_bearer_token")), form: vec![], json: None, raw: None, timeout_s: 15 })?;
                        let found = serde_json::from_str::<J>(&b).ok().and_then(|j| j["devices"].as_array().map(|d| d.iter().any(|x| x["id"].as_str().map_or(false, |i| i.contains(cfg.g("mp_id_point_smart")))))).unwrap_or(false);
                        Ok(json!({"ok": st < 300 && found, "message": if st >= 300 { "Mercado Pago rejected the token".to_string() } else if found { "Terminal found".into() } else { "Token accepted, terminal serial not found".into() }})) }
                    "viva_wallet" => { let (accounts, _) = if cfg.test { ("https://demo-accounts.vivapayments.com", "") } else { ("https://accounts.vivapayments.com", "") };
                        let r = drivers::viva_token_pub(http.as_ref(), creds.viva_accounts.as_deref().unwrap_or(accounts), &cfg); Ok(json!({"ok": r.is_ok(), "message": r.map(|_| "Credentials accepted".to_string()).unwrap_or_else(|e| e.to_string())})) }
                    p => Ok(json!({"ok": true, "message": format!("`{p}` has no read-only test call; use a small test payment")})),
                }
            }
            _ => Err(OdooError::User("unknown terminal method".into())),
        }
    }
}

#[cfg(test)]
mod tests;
