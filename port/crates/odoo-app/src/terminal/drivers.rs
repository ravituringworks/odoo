//! Card-terminal providers. Every driver is request/response over the `Http` port, so each is testable against a mock.
//! Request shapes follow Odoo's own point-of-sale payment addons (pos_stripe, pos_adyen, pos_mercado_pago, pos_viva_wallet,
//! pos_razorpay, pos_paytm, pos_pine_labs), moved from the browser to the server so credentials never reach the client.
use super::*;
use base64::{engine::general_purpose::STANDARD as B64, Engine};

pub fn bearer(key: &str) -> Vec<(String, String)> { vec![("Authorization".into(), format!("Bearer {key}"))] }
pub fn basic(user: &str, pass: &str) -> Vec<(String, String)> { vec![("Authorization".into(), format!("Basic {}", B64.encode(format!("{user}:{pass}"))))] }
fn form(pairs: &[(&str, &str)]) -> Vec<(String, String)> { pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect() }
fn req(method: &'static str, url: String, headers: Vec<(String, String)>) -> HttpReq { HttpReq { method, url, headers, form: vec![], json: None, raw: None, timeout_s: 20 } }
fn jreq(method: &'static str, url: String, headers: Vec<(String, String)>, body: J) -> HttpReq { HttpReq { json: Some(body), ..req(method, url, headers) } }
fn parse(body: &str) -> J { serde_json::from_str(body).unwrap_or(J::Null) }
fn s(j: &J) -> String { match j { J::String(x) => x.clone(), J::Number(n) => n.to_string(), _ => String::new() } }
fn last4(x: &str) -> String { let v: Vec<char> = x.chars().filter(|c| c.is_ascii_digit()).collect(); v[v.len().saturating_sub(4)..].iter().collect() }
fn require(cfg: &MethodCfg, keys: &[(&str, &str)]) -> Result<()> { for (k, label) in keys { if cfg.g(k).is_empty() { return Err(OdooError::User(format!("This payment method needs {label}"))); } } Ok(()) }
fn bad(c: &Creds, cfg: &MethodCfg, msg: &str) -> String { let mut m = msg.to_string(); for k in c.secrets().iter().chain(cfg.secrets().iter()).filter(|k| k.len() > 3) { m = m.replace(k.as_str(), "***"); } m.chars().take(300).collect() }
/// ISO 4217 numeric codes for the currencies we meet most often (Viva wants the numeric code).
pub fn iso_numeric(cur: &str) -> String { match cur.to_uppercase().as_str() { "EUR" => "978", "USD" => "840", "GBP" => "826", "BGN" => "975", "CZK" => "203", "DKK" => "208", "HUF" => "348", "PLN" => "985", "RON" => "946", "SEK" => "752", "CHF" => "756", "NOK" => "578", "HRK" => "191", "INR" => "356", "BRL" => "986", "MXN" => "484", "ARS" => "032", "CLP" => "152", "COP" => "170", "PEN" => "604", "UYU" => "858", _ => "978" }.into() }

// ───────────────────────────── start ─────────────────────────────
pub fn start(http: &dyn Http, c: &Creds, cfg: &MethodCfg, id: &str, cents: i64, cur: &str, reference: &str, who: &str) -> Result<Tx> {
    if cents <= 0 { return Err(OdooError::User("Nothing to charge".into())); }
    let mut tx = Tx { provider: cfg.provider.clone(), cents, status: "pending".into(), reference: reference.into(), created: Some(Instant::now()), ..Default::default() };
    let fail = |b: &str, k: &str| OdooError::User(bad(c, cfg, &stripe_msg(b, k)));
    match cfg.provider.as_str() {
        "simulator" => {}
        "stripe" => {
            if c.stripe_key.is_empty() { return Err(OdooError::User("Stripe secret key is not set (Settings → Payment terminals)".into())); }
            require(cfg, &[("stripe_serial_number", "the Stripe reader serial number")])?;
            let reader = cfg.g("stripe_serial_number");
            let mut r = req("POST", format!("{}/v1/payment_intents", c.stripe_base), bearer(&c.stripe_key));
            r.form = form(&[("amount", &cents.to_string()), ("currency", &cur.to_lowercase()), ("payment_method_types[]", "card_present"), ("capture_method", "automatic"), ("description", reference)]);
            let (st, b) = http.send(&r)?; if st >= 300 { return Err(fail(&b, &c.stripe_key)); }
            let pi = s(&parse(&b)["id"]); if pi.is_empty() { return Err(OdooError::User("Stripe returned no payment intent".into())); }
            let mut r = req("POST", format!("{}/v1/terminal/readers/{}/process_payment_intent", c.stripe_base, reader), bearer(&c.stripe_key)); r.form = form(&[("payment_intent", &pi)]);
            let (st, b) = http.send(&r)?; if st >= 300 { return Err(fail(&b, &c.stripe_key)); }
            tx.aux.insert("pi".into(), pi); tx.aux.insert("reader".into(), reader.into());
        }
        "adyen" => {
            require(cfg, &[("adyen_api_key", "the Adyen API key"), ("adyen_terminal_identifier", "the Adyen terminal identifier")])?;
            tx.aux.insert("service".into(), id.chars().rev().take(10).collect());
        }
        "mercado_pago" => {
            require(cfg, &[("mp_bearer_token", "the Mercado Pago token"), ("mp_id_point_smart", "the terminal serial number")])?;
            let h = bearer(cfg.g("mp_bearer_token"));
            let (st, b) = http.send(&req("GET", format!("{}/point/integration-api/devices", c.mp_base), h.clone()))?;
            let dev = parse(&b)["devices"].as_array().and_then(|d| d.iter().find(|x| s(&x["id"]).contains(cfg.g("mp_id_point_smart")))).map(|x| s(&x["id"]));
            let dev = match (st, dev) { (200..=299, Some(d)) => d, (200..=299, None) => return Err(OdooError::User("The terminal serial number is not registered on Mercado Pago".into())), _ => return Err(OdooError::User("Mercado Pago rejected the token".into())) };
            let body = json!({"amount": cents, "additional_info": {"external_reference": reference, "print_on_terminal": true}});
            let (st, b) = http.send(&jreq("POST", format!("{}/point/integration-api/devices/{}/payment-intents", c.mp_base, dev), h, body))?;
            let j = parse(&b); let pi = s(&j["id"]);
            if st >= 300 || pi.is_empty() { return Err(OdooError::User(bad(c, cfg, &format!("Mercado Pago: {}", s(&j["message"]).trim().to_string().as_str().to_owned().chars().take(200).collect::<String>())))); }
            tx.aux.insert("pi".into(), pi); tx.aux.insert("dev".into(), dev);
        }
        "viva_wallet" => {
            require(cfg, &[("viva_wallet_client_id", "the Viva client id"), ("viva_wallet_client_secret", "the Viva client secret"), ("viva_wallet_terminal_id", "the Viva terminal id")])?;
            let (accounts, api) = if cfg.test { ("https://demo-accounts.vivapayments.com", "https://demo-api.vivapayments.com") } else { ("https://accounts.vivapayments.com", "https://api.vivapayments.com") };
            let (accounts, api) = (c.viva_accounts.clone().unwrap_or(accounts.into()), c.viva_api.clone().unwrap_or(api.into()));
            let token = viva_token(http, &accounts, cfg)?;
            let session = format!("{}-{}", id, id.len());
            let body = json!({"sessionId": session, "terminalId": cfg.g("viva_wallet_terminal_id"), "cashRegisterId": if who.is_empty() { "POS" } else { who }, "amount": cents, "currencyCode": iso_numeric(cur), "merchantReference": format!("{session}/{reference}"), "customerTrns": " ", "preauth": false, "maxInstalments": 0, "tipAmount": 0});
            let (st, b) = http.send(&jreq("POST", format!("{api}/ecr/v1/transactions:sale"), bearer(&token), body))?;
            if st >= 300 { return Err(OdooError::User(bad(c, cfg, &format!("Viva Wallet: {}", s(&parse(&b)["detail"]))))); }
            tx.aux.insert("session".into(), session); tx.aux.insert("api".into(), api); tx.aux.insert("token".into(), token); tx.aux.insert("register".into(), if who.is_empty() { "POS".into() } else { who.into() });
        }
        "razorpay" => {
            require(cfg, &[("razorpay_tid", "the device serial number"), ("razorpay_username", "the device username"), ("razorpay_api_key", "the API key")])?;
            let base = c.razorpay_base.clone().unwrap_or(if cfg.test { "https://demo.ezetap.com/api/3.0/p2padapter/".into() } else { "https://www.ezetap.com/api/3.0/p2padapter/".into() });
            let mode = match cfg.g("razorpay_allowed_payment_modes") { "" => "ALL".to_string(), m => m.to_uppercase() };
            let body = json!({"pushTo": {"deviceId": format!("{}|ezetap_android", cfg.g("razorpay_tid"))}, "mode": mode, "username": cfg.g("razorpay_username"), "appKey": cfg.g("razorpay_api_key"), "amount": cents as f64 / 100.0, "externalRefNumber": reference});
            let (st, b) = http.send(&jreq("POST", format!("{base}pay"), vec![], body))?; let j = parse(&b);
            if st >= 300 || !(j["success"] == true) || !j["errorCode"].is_null() { return Err(OdooError::User(bad(c, cfg, &format!("Razorpay: {}", { let m = s(&j["errorMessage"]); if m.is_empty() { "payment request failed".into() } else { m } })))); }
            tx.aux.insert("p2p".into(), s(&j["p2pRequestId"])); tx.aux.insert("base".into(), base);
        }
        "paytm" => {
            require(cfg, &[("paytm_mid", "the merchant id"), ("paytm_tid", "the terminal id"), ("paytm_merchant_key", "the merchant key")])?;
            let base = c.paytm_base.clone().unwrap_or(if cfg.test { "https://securegw-stage.paytm.in/ecr/".into() } else { "https://securegw-edc.paytm.in/ecr/".into() });
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let (tid, rid) = (format!("{}{}", id.to_uppercase().chars().take(24).collect::<String>(), "T"), format!("R{}", now % 1_000_000_000));
            let mut body = paytm_body(cfg, &tid, &rid, now);
            body.insert("transactionAmount".into(), cents.to_string());
            if cfg.g("accept_payment") != "manual" { body.insert("autoAccept".into(), "True".into()); }
            let mode = match cfg.g("allowed_payment_modes") { "" => "ALL".to_string(), m => m.to_uppercase() };
            body.insert("paymentMode".into(), mode.clone());
            let head = paytm_head(cfg, &body)?;
            let mut jb: serde_json::Map<String, J> = body.iter().map(|(k, v)| (k.clone(), J::String(v.clone()))).collect();
            let mut ext = serde_json::Map::new(); ext.insert("paymentMode".into(), J::String(mode)); if cfg.g("accept_payment") != "manual" { ext.insert("autoAccept".into(), J::String("True".into())); }
            jb.insert("merchantExtendedInfo".into(), J::Object(ext));
            let (st, b) = http.send(&jreq("POST", format!("{base}payment/request"), vec![], json!({"head": head, "body": J::Object(jb)})))?;
            let r = parse(&b)["body"]["resultInfo"].clone();
            if st >= 300 || s(&r["resultCode"]) != "A" { return Err(OdooError::User(bad(c, cfg, &format!("PayTM: {}", { let m = s(&r["resultMsg"]); if m.is_empty() { "transaction request declined".into() } else { m } })))); }
            tx.aux.insert("tid".into(), tid); tx.aux.insert("rid".into(), rid); tx.aux.insert("ts".into(), now.to_string()); tx.aux.insert("base".into(), base);
        }
        "pine_labs" => {
            require(cfg, &[("pine_labs_merchant", "the merchant id"), ("pine_labs_store", "the store id"), ("pine_labs_client", "the client id"), ("pine_labs_security_token", "the security token")])?;
            let base = c.pine_base.clone().unwrap_or(if cfg.test { "https://www.plutuscloudserviceuat.in:8201/API/CloudBasedIntegration/V1/".into() } else { "https://www.plutuscloudservice.in:8201/API/CloudBasedIntegration/V1/".into() });
            let mode = match cfg.g("pine_labs_allowed_payment_mode") { "card" => 1, "upi" => 10, _ => 0 };
            let mut body = pine_auth(cfg); body["Amount"] = json!(cents); body["TransactionNumber"] = json!(reference); body["SequenceNumber"] = json!(1); body["AllowedPaymentMode"] = json!(mode); body["AutoCancelDurationInMinutes"] = json!(10);
            let (st, b) = http.send(&jreq("POST", format!("{base}UploadBilledTransaction"), vec![], body))?; let j = parse(&b);
            if st >= 300 || j["ResponseCode"] != 0 || s(&j["ResponseMessage"]) != "APPROVED" { return Err(OdooError::User(bad(c, cfg, &format!("Pine Labs: {}", { let m = s(&j["ResponseMessage"]); if m.is_empty() { "request failed".into() } else { m } })))); }
            tx.aux.insert("plutus".into(), s(&j["PlutusTransactionReferenceID"])); tx.aux.insert("base".into(), base);
        }
        other => return Err(OdooError::User(format!("The `{other}` terminal driver is not available in this build"))),
    }
    Ok(tx)
}
fn stripe_msg(body: &str, key: &str) -> String { let m = s(&parse(body)["error"]["message"]); if m.is_empty() { "Stripe rejected the request".into() } else { m.replace(key, "***") } }
pub fn viva_token_pub(http: &dyn Http, accounts: &str, cfg: &MethodCfg) -> Result<String> { viva_token(http, accounts, cfg) }
fn viva_token(http: &dyn Http, accounts: &str, cfg: &MethodCfg) -> Result<String> {
    let mut r = req("POST", format!("{accounts}/connect/token"), basic(cfg.g("viva_wallet_client_id"), cfg.g("viva_wallet_client_secret"))); r.form = form(&[("grant_type", "client_credentials")]);
    let (_, b) = http.send(&r)?; let t = s(&parse(&b)["access_token"]);
    if t.is_empty() { Err(OdooError::User("Unable to get a Viva Wallet token — check the client id and secret".into())) } else { Ok(t) }
}
fn pine_auth(cfg: &MethodCfg) -> J { json!({"MerchantID": cfg.g("pine_labs_merchant"), "StoreID": cfg.g("pine_labs_store"), "ClientID": cfg.g("pine_labs_client"), "SecurityToken": cfg.g("pine_labs_security_token")}) }

// ── Paytm request signing: SHA-256 over the sorted values + a random salt, padded and AES-CBC encrypted with the merchant key ──
pub fn fmt_ist(epoch: u64) -> String {
    let t = epoch + 19_800; let (days, rem) = ((t / 86_400) as i64, t % 86_400);
    let z = days + 719_468; let era = z.div_euclid(146_097); let doe = z.rem_euclid(146_097); let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", if m <= 2 { y + 1 } else { y }, m, d, rem / 3600, rem % 3600 / 60, rem % 60)
}
fn paytm_body(cfg: &MethodCfg, tid: &str, rid: &str, epoch: u64) -> std::collections::BTreeMap<String, String> {
    [("paytmMid", cfg.g("paytm_mid").to_string()), ("paytmTid", cfg.g("paytm_tid").to_string()), ("transactionDateTime", fmt_ist(epoch)), ("merchantTransactionId", tid.to_string()), ("merchantReferenceNo", rid.to_string())].into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
pub fn paytm_signature(params: &std::collections::BTreeMap<String, String>, key: &str, salt: &str) -> Result<String> {
    use aes::cipher::{block_padding::NoPadding, BlockEncryptMut, KeyIvInit};
    use sha2::{Digest, Sha256};
    let mut parts: Vec<String> = params.values().map(|v| if v.eq_ignore_ascii_case("null") { String::new() } else { v.clone() }).collect();   // BTreeMap = sorted by key
    parts.push(salt.to_string());
    let hex: String = Sha256::digest(parts.join("|").as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let mut buf = format!("{hex}{salt}").into_bytes(); buf.extend(std::iter::repeat(12u8).take(12));
    while buf.len() % 16 != 0 { buf.push(0); }
    let iv = b"@@@@&&&&####$$$$"; let n = buf.len();
    macro_rules! enc { ($a:ty) => { cbc::Encryptor::<$a>::new_from_slices(key.as_bytes(), iv).map_err(|_| OdooError::User("The PayTM merchant key must be 16, 24 or 32 characters".into()))?.encrypt_padded_mut::<NoPadding>(&mut buf, n).map_err(|_| OdooError::User("PayTM signing failed".into()))?.to_vec() } }
    let out = match key.len() { 16 => enc!(aes::Aes128), 24 => enc!(aes::Aes192), 32 => enc!(aes::Aes256), _ => return Err(OdooError::User("The PayTM merchant key must be 16, 24 or 32 characters".into())) };
    Ok(B64.encode(out))
}
fn paytm_head(cfg: &MethodCfg, body: &std::collections::BTreeMap<String, String>) -> Result<J> {
    let salt: String = crate::security::random_hex(4)[..4].to_string();
    Ok(json!({"requestTimeStamp": body["transactionDateTime"], "channelId": if cfg.g("channel_id").is_empty() { "EDC" } else { cfg.g("channel_id") }, "checksum": paytm_signature(body, cfg.g("paytm_merchant_key"), &salt)?}))
}

// ───────────────────────────── poll ─────────────────────────────
pub fn poll(http: &dyn Http, c: &Creds, cfg: &MethodCfg, tx: &mut Tx) {
    if tx.status != "pending" { return; }
    let a = |k: &str| tx.aux.get(k).cloned().unwrap_or_default();
    let mut done: Option<Out> = None;
    match tx.provider.as_str() {
        "simulator" => {
            if tx.created.map_or(true, |t| t.elapsed() >= Duration::from_millis(1500)) {
                done = Some(if tx.cents % 100 == 13 { Out::fail("Card declined (simulated)") } else { Out::ok("visa", "4242", &format!("sim-{}", tx.reference)) });
            }
        }
        "stripe" => {
            let get = |url: String| http.send(&req("GET", url, bearer(&c.stripe_key)));
            if let Ok((st, b)) = get(format!("{}/v1/terminal/readers/{}", c.stripe_base, a("reader"))) { if st < 300 {
                let j = parse(&b);
                match s(&j["action"]["status"]).as_str() {
                    "failed" => done = Some(Out::fail(&bad(c, cfg, &s(&j["action"]["failure_message"])))),
                    "succeeded" => if let Ok((s2, b2)) = get(format!("{}/v1/payment_intents/{}?expand[]=latest_charge", c.stripe_base, a("pi"))) { if s2 < 300 {
                        let p = parse(&b2);
                        done = Some(if p["status"] == "succeeded" { let cp = &p["latest_charge"]["payment_method_details"]["card_present"]; Out::ok(&s(&cp["brand"]), &s(&cp["last4"]), &a("pi")) } else { Out::fail("The payment was not captured") });
                    } },
                    _ => {}
                }
            } }
        }
        "mercado_pago" => {
            let h = bearer(cfg.g("mp_bearer_token"));
            if let Ok((st, b)) = http.send(&req("GET", format!("{}/point/integration-api/payment-intents/{}", c.mp_base, a("pi")), h.clone())) { if st < 300 {
                let j = parse(&b);
                match s(&j["state"]).as_str() {
                    "CANCELED" => done = Some(Out::fail("Payment has been canceled")),
                    "FINISHED" | "PROCESSED" => {
                        let pid = s(&j["payment"]["id"]);
                        if let Ok((s2, b2)) = http.send(&req("GET", format!("{}/v1/payments/{}", c.mp_base, pid), h)) { if s2 < 300 {
                            let p = parse(&b2);
                            done = Some(if s(&p["status"]) == "approved" { Out::ok(&s(&p["payment_method_id"]), &s(&p["card"]["last_four_digits"]), &pid) } else { Out::fail("Payment has been rejected") });
                        } }
                    }
                    _ => {}   // OPEN / ON_TERMINAL / PROCESSING
                }
            } }
        }
        "viva_wallet" => {
            let get = |token: &str| http.send(&req("GET", format!("{}/ecr/v1/sessions/{}", a("api"), a("session")), bearer(token)));
            if let Ok((st, b)) = get(&a("token")) { if st < 300 && !b.trim().is_empty() {
                let j = parse(&b);
                if let Some(ok) = j.get("success").and_then(|v| v.as_bool()) {
                    done = Some(if ok { Out::ok(&s(&j["applicationLabel"]), &last4(&s(&j["primaryAccountNumberMasked"])), &s(&j["transactionId"])) } else { Out::fail(&bad(c, cfg, &{ let m = s(&j["message"]); if m.is_empty() { "The payment was declined".into() } else { m } })) });
                }
            } }
        }
        "razorpay" => {
            let body = json!({"username": cfg.g("razorpay_username"), "appKey": cfg.g("razorpay_api_key"), "origP2pRequestId": a("p2p")});
            if let Ok((st, b)) = http.send(&jreq("POST", format!("{}status", a("base")), vec![], body)) { if st < 300 {
                let j = parse(&b);
                if j["success"] == true && j["errorCode"].is_null() {
                    let (status, code) = (s(&j["status"]), s(&j["messageCode"]));
                    if status == "AUTHORIZED" && code == "P2P_DEVICE_TXN_DONE" {
                        done = Some(if s(&j["externalRefNumber"]) != tx.reference { Out::fail("Reference number mismatched") } else { Out::ok(&s(&j["paymentCardBrand"]), &s(&j["cardLastFourDigit"]), &s(&j["txnId"])) });
                    } else if status == "FAILED" || code == "P2P_DEVICE_CANCELED" { done = Some(Out::fail(&bad(c, cfg, &{ let m = s(&j["message"]); if m.is_empty() { "Razorpay transaction failed".into() } else { m } }))); }
                } else { done = Some(Out::fail(&bad(c, cfg, &{ let m = s(&j["errorMessage"]); if m.is_empty() { "Razorpay status request failed".into() } else { m } }))); }
            } }
        }
        "paytm" => {
            let (now, tid, rid) = (a("ts").parse::<u64>().unwrap_or(0), a("tid"), a("rid"));
            let body = paytm_body(cfg, &tid, &rid, now);
            if let Ok(head) = paytm_head(cfg, &body) {
                let jb: serde_json::Map<String, J> = body.iter().map(|(k, v)| (k.clone(), J::String(v.clone()))).collect();
                if let Ok((st, b)) = http.send(&jreq("POST", format!("{}V2/payment/status", a("base")), vec![], json!({"head": head, "body": J::Object(jb)}))) { if st < 300 {
                    let r = parse(&b)["body"].clone();
                    match s(&r["resultInfo"]["resultCode"]).as_str() {
                        "S" => done = Some(if s(&r["merchantReferenceNo"]) != rid { Out::fail("Reference number mismatched") } else { Out::ok(&s(&r["cardScheme"]), &last4(&s(&r["issuerMaskCardNo"])), &s(&r["merchantTransactionId"])) }),
                        "F" => done = Some(Out::fail(&bad(c, cfg, &{ let m = s(&r["resultInfo"]["resultMsg"]); if m.is_empty() { "PayTM transaction failed".into() } else { m } }))),
                        _ => {}   // "P": still pending
                    }
                } }
            }
        }
        "pine_labs" => {
            let mut body = pine_auth(cfg); body["PlutusTransactionReferenceID"] = json!(a("plutus").parse::<i64>().unwrap_or(0));
            if let Ok((st, b)) = http.send(&jreq("POST", format!("{}GetCloudBasedTxnStatus", a("base")), vec![], body)) { if st < 300 {
                let j = parse(&b);
                match j["ResponseCode"].as_i64() {
                    Some(0) => {
                        let data: std::collections::BTreeMap<String, String> = j["TransactionData"].as_array().map(|d| d.iter().map(|x| (s(&x["Tag"]), s(&x["Value"]))).collect()).unwrap_or_default();
                        if s(&j["ResponseMessage"]) == "TXN APPROVED" { done = Some(Out::ok(data.get("Card Type").map(|x| x.as_str()).unwrap_or(""), &last4(data.get("Card Number").map(|x| x.as_str()).unwrap_or("")), data.get("TransactionLogId").map(|x| x.as_str()).unwrap_or(""))); }
                        else if s(&j["ResponseMessage"]) != "TXN UPLOADED" { done = Some(Out::fail(&bad(c, cfg, &s(&j["ResponseMessage"])))); }
                    }
                    Some(1001) => {}   // in progress
                    _ => done = Some(Out::fail(&bad(c, cfg, &{ let m = s(&j["ResponseMessage"]); if m.is_empty() { "Pine Labs status request failed".into() } else { m } }))),
                }
            } }
        }
        _ => {}   // Adyen results are delivered by its worker thread
    }
    if let Some(o) = done { tx.status = o.status; tx.message = o.message; tx.brand = o.brand; tx.last4 = o.last4; tx.refref = o.reference; }
}

// ───────────────────────────── cancel ─────────────────────────────
pub fn cancel(http: &dyn Http, c: &Creds, cfg: &MethodCfg, tx: &mut Tx, id: &str) {
    if tx.status != "pending" { return; }
    let a = |k: &str| tx.aux.get(k).cloned().unwrap_or_default();
    match tx.provider.as_str() {
        "stripe" => { let _ = http.send(&req("POST", format!("{}/v1/terminal/readers/{}/cancel_action", c.stripe_base, a("reader")), bearer(&c.stripe_key))); }
        "adyen" => { let _ = http.send(&jreq("POST", format!("{}/sync", c.adyen_base), vec![("X-API-Key".into(), cfg.g("adyen_api_key").into())], adyen_abort(cfg, id, &a("service")))); }
        "mercado_pago" => { let _ = http.send(&req("DELETE", format!("{}/point/integration-api/devices/{}/payment-intents/{}", c.mp_base, a("dev"), a("pi")), bearer(cfg.g("mp_bearer_token")))); }
        "viva_wallet" => { let _ = http.send(&req("DELETE", format!("{}/ecr/v1/sessions/{}?cashRegisterId={}", a("api"), a("session"), a("register")), bearer(&a("token")))); }
        "razorpay" => { let body = json!({"pushTo": {"deviceId": format!("{}|ezetap_android", cfg.g("razorpay_tid"))}, "username": cfg.g("razorpay_username"), "appKey": cfg.g("razorpay_api_key"), "origP2pRequestId": a("p2p")}); let _ = http.send(&jreq("POST", format!("{}cancel", a("base")), vec![], body)); }
        "pine_labs" => { let mut body = pine_auth(cfg); body["Amount"] = json!(tx.cents); body["PlutusTransactionReferenceID"] = json!(a("plutus").parse::<i64>().unwrap_or(0)); body["TakeToHomeScreen"] = json!(true); body["ConfirmationRequired"] = json!(true); let _ = http.send(&jreq("POST", format!("{}CancelTransactionForced", a("base")), vec![], body)); }
        _ => {}   // simulator; PayTM has no cancel call (the customer cancels on the terminal)
    }
    tx.status = "cancelled".into(); tx.message = "Cancelled".into();
}
fn adyen_abort(cfg: &MethodCfg, id: &str, service: &str) -> J {
    json!({"SaleToPOIRequest": {"MessageHeader": {"ProtocolVersion": "3.0", "MessageClass": "Service", "MessageCategory": "Abort", "MessageType": "Request", "SaleID": "odoo-rs", "ServiceID": format!("A{}", id.chars().rev().take(9).collect::<String>()), "POIID": cfg.g("adyen_terminal_identifier")},
        "AbortRequest": {"AbortReason": "MerchantAbort", "MessageReference": {"MessageCategory": "Payment", "SaleID": "odoo-rs", "ServiceID": service}}}})
}

// ───────────────────────────── Adyen (blocking TerminalAPI) ─────────────────────────────
pub fn adyen_charge(http: &dyn Http, cfg: &MethodCfg, base: &str, service: &str, cents: i64, cur: &str, reference: &str) -> Out {
    let body = json!({"SaleToPOIRequest": {"MessageHeader": {"ProtocolVersion": "3.0", "MessageClass": "Service", "MessageCategory": "Payment", "MessageType": "Request", "SaleID": "odoo-rs", "ServiceID": service, "POIID": cfg.g("adyen_terminal_identifier")},
        "PaymentRequest": {"SaleData": {"SaleTransactionID": {"TransactionID": reference, "TimeStamp": iso_now()}}, "PaymentTransaction": {"AmountsReq": {"Currency": cur.to_uppercase(), "RequestedAmount": cents as f64 / 100.0}}}}});
    let mut r = jreq("POST", format!("{base}/sync"), vec![("X-API-Key".into(), cfg.g("adyen_api_key").into()), ("Content-Type".into(), "application/json".into())], body); r.timeout_s = 130;
    match http.send(&r) {
        Ok((st, b)) if st < 300 => parse_adyen(&b, cfg.g("adyen_api_key")),
        Ok((st, b)) => Out::fail(&format!("Adyen returned {st}: {}", b.replace(cfg.g("adyen_api_key"), "***").chars().take(200).collect::<String>())),
        Err(e) => Out::fail(&e.to_string()),
    }
}
pub fn parse_adyen(body: &str, key: &str) -> Out {
    let j = parse(body); let r = &j["SaleToPOIResponse"]["PaymentResponse"];
    if r["Response"]["Result"] == "Success" {
        let card = &r["PaymentResult"]["PaymentInstrumentData"]["CardData"]; let t = &r["POIData"]["POITransactionID"];
        Out::ok(&s(&card["PaymentBrand"]), &last4(&s(&card["MaskedPan"])), &if s(&t["TransactionID"]).is_empty() { String::new() } else { format!("{}|{}", s(&t["TransactionID"]), s(&t["TimeStamp"])) })
    } else {
        let why = { let a = s(&r["Response"]["AdditionalResponse"]); if a.is_empty() { s(&r["Response"]["ErrorCondition"]) } else { a } };
        let why = if why.is_empty() { "Payment was not completed".to_string() } else { percent_decode(&why) };
        Out::fail(&why.replace(key, "***").chars().take(300).collect::<String>())
    }
}
fn percent_decode(s: &str) -> String { let b = s.as_bytes(); let mut o = vec![]; let mut i = 0; while i < b.len() { if b[i] == b'%' && i + 2 < b.len() + 1 && i + 2 <= b.len().saturating_sub(1) { if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { o.push(v); i += 3; continue; } } o.push(b[i]); i += 1; } String::from_utf8_lossy(&o).replace('+', " ") }

// ───────────────────────────── refunds to the card ─────────────────────────────
/// Providers whose API can return money to the card by reference. Others need `manual` confirmation (refunded on the device / dashboard).
pub fn can_refund(provider: &str) -> bool { matches!(provider, "simulator" | "stripe" | "adyen" | "mercado_pago") }
pub fn refund(http: &dyn Http, c: &Creds, cfg: &MethodCfg, reference: &str, cents: i64, cur: &str) -> Result<String> {
    if cents <= 0 { return Err(OdooError::User("Nothing to refund".into())); }
    if reference.is_empty() && cfg.provider != "simulator" { return Err(OdooError::User("The original card payment has no reference to refund".into())); }
    match cfg.provider.as_str() {
        "simulator" => Ok(format!("sim-refund-{cents}")),
        "stripe" => {
            let mut r = req("POST", format!("{}/v1/refunds", c.stripe_base), bearer(&c.stripe_key)); r.form = form(&[("payment_intent", reference), ("amount", &cents.to_string())]);
            let (st, b) = http.send(&r)?; if st >= 300 { return Err(OdooError::User(bad(c, cfg, &stripe_msg(&b, &c.stripe_key)))); }
            Ok(s(&parse(&b)["id"]))
        }
        "mercado_pago" => {
            let (st, b) = http.send(&jreq("POST", format!("{}/v1/payments/{}/refunds", c.mp_base, reference), bearer(cfg.g("mp_bearer_token")), json!({"amount": cents as f64 / 100.0})))?;
            if st >= 300 { return Err(OdooError::User(bad(c, cfg, &format!("Mercado Pago: {}", s(&parse(&b)["message"]))))); }
            Ok(s(&parse(&b)["id"]))
        }
        "adyen" => {
            let (tid, ts) = reference.split_once('|').unwrap_or((reference, ""));
            let body = json!({"SaleToPOIRequest": {"MessageHeader": {"ProtocolVersion": "3.0", "MessageClass": "Service", "MessageCategory": "Reversal", "MessageType": "Request", "SaleID": "odoo-rs", "ServiceID": crate::security::random_hex(5), "POIID": cfg.g("adyen_terminal_identifier")},
                "ReversalRequest": {"OriginalPOITransaction": {"POITransactionID": {"TransactionID": tid, "TimeStamp": ts}}, "ReversalReason": "MerchantCancel", "ReversedAmount": cents as f64 / 100.0}}});
            let mut r = jreq("POST", format!("{}/sync", c.adyen_base), vec![("X-API-Key".into(), cfg.g("adyen_api_key").into())], body); r.timeout_s = 60;
            let (st, b) = http.send(&r)?; let j = parse(&b);
            if st >= 300 || j["SaleToPOIResponse"]["ReversalResponse"]["Response"]["Result"] != "Success" { return Err(OdooError::User(bad(c, cfg, &format!("Adyen refused the refund: {}", percent_decode(&s(&j["SaleToPOIResponse"]["ReversalResponse"]["Response"]["AdditionalResponse"])))))); }
            let _ = cur; Ok(format!("{tid}-reversal"))
        }
        other => Err(OdooError::User(format!("`{other}` cannot refund through its API — refund on the device or dashboard, then confirm manually"))),
    }
}
