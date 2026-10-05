//! Outbound email: provider settings, strict message validation, pure per-provider request builders/parsers (SendGrid, Mailgun,
//! Postmark, Resend, Brevo over HTTP; any server over SMTP) and a transport trait so the logic is testable without a network.
//! Secrets (API keys, SMTP password) are write-only through the API. From-address is always the configured one (no spoofing).
use odoo_core::{OdooError, Result};
use serde_json::{json, Value as J};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind { Http, Smtp }
/// One setting a provider needs; the UI renders the form from this list.
pub struct Field { pub id: &'static str, pub label: &'static str, pub secret: bool, pub options: &'static [&'static str], pub placeholder: &'static str }
pub struct Provider { pub id: &'static str, pub label: &'static str, pub kind: Kind, pub base: &'static str, pub eu_base: &'static str, pub fields: &'static [Field], pub docs: &'static str }

const KEY: Field = Field { id: "api_key", label: "API key", secret: true, options: &[], placeholder: "" };
const REGION: Field = Field { id: "region", label: "Region", secret: false, options: &["us", "eu"], placeholder: "" };
pub const PROVIDERS: &[Provider] = &[
    Provider { id: "sendgrid", label: "SendGrid", kind: Kind::Http, base: "https://api.sendgrid.com", eu_base: "https://api.eu.sendgrid.com", fields: &[KEY, REGION], docs: "Create an API key with the Mail Send permission, and verify your sender." },
    Provider { id: "mailgun", label: "Mailgun", kind: Kind::Http, base: "https://api.mailgun.net", eu_base: "https://api.eu.mailgun.net", fields: &[KEY, Field { id: "domain", label: "Sending domain", secret: false, options: &[], placeholder: "mg.example.com" }, REGION], docs: "Use the private API key and a verified sending domain." },
    Provider { id: "postmark", label: "Postmark", kind: Kind::Http, base: "https://api.postmarkapp.com", eu_base: "", fields: &[Field { id: "api_key", label: "Server token", secret: true, options: &[], placeholder: "" }], docs: "Use a Server API token; the sender signature must be confirmed." },
    Provider { id: "resend", label: "Resend", kind: Kind::Http, base: "https://api.resend.com", eu_base: "", fields: &[KEY], docs: "Create an API key; the sending domain must be verified." },
    Provider { id: "brevo", label: "Brevo (Sendinblue)", kind: Kind::Http, base: "https://api.brevo.com", eu_base: "", fields: &[KEY], docs: "Create a v3 API key; the sender must be validated." },
    Provider { id: "smtp", label: "SMTP (Gmail, Office 365, Amazon SES, …)", kind: Kind::Smtp, base: "", eu_base: "", fields: &[
        Field { id: "smtp_host", label: "SMTP host", secret: false, options: &[], placeholder: "smtp.example.com" },
        Field { id: "smtp_port", label: "Port", secret: false, options: &[], placeholder: "587" },
        Field { id: "smtp_security", label: "Security", secret: false, options: &["starttls", "ssl", "none"], placeholder: "" },
        Field { id: "smtp_user", label: "Username", secret: false, options: &[], placeholder: "" },
        Field { id: "smtp_password", label: "Password", secret: true, options: &[], placeholder: "" }], docs: "Port 587 + STARTTLS or 465 + SSL are typical. Use an app password where required." },
];
pub fn provider(id: &str) -> Option<&'static Provider> { PROVIDERS.iter().find(|p| p.id == id) }

#[derive(Clone, Debug, Default)]
pub struct EmailSettings {
    pub enabled: bool, pub provider: String, pub from_address: String, pub from_name: String, pub reply_to: String,
    pub api_key: String, pub domain: String, pub region: String,
    pub smtp_host: String, pub smtp_port: u16, pub smtp_security: String, pub smtp_user: String, pub smtp_password: String,
    pub base_url: String, pub app_url: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Message { pub to: Vec<String>, pub cc: Vec<String>, pub bcc: Vec<String>, pub subject: String, pub text: String, pub html: String }

// ───────────────────────── validation (pure) ─────────────────────────

fn clean_line(s: &str) -> bool { !s.chars().any(|c| c == '\r' || c == '\n' || c == '\0') }

/// A bare address: one `@`, a dotted domain, no whitespace/control/angle/quote/comma characters.
pub fn valid_address(a: &str) -> bool {
    let (l, d) = match a.split_once('@') { Some(p) => p, None => return false };
    a.len() <= 254 && !l.is_empty() && l.len() <= 64 && d.contains('.') && !d.starts_with('.') && !d.ends_with('.') && !d.contains("..") && !d.contains('@')
        && a.chars().all(|c| c.is_ascii_graphic() && !"<>,;\"()[]\\".contains(c))
}

/// Parse `"Name <a@b.c>, d@e.f; g@h.i"` into bare addresses; any invalid entry rejects the whole list.
pub fn parse_addresses(s: &str) -> Result<Vec<String>> {
    let mut out = vec![];
    for part in s.split([',', ';']).map(str::trim).filter(|p| !p.is_empty()) {
        if !clean_line(part) { return Err(OdooError::Validation("invalid characters in address list".into())); }
        let addr = match (part.rfind('<'), part.rfind('>')) { (Some(a), Some(b)) if a < b && b == part.len() - 1 => part[a + 1..b].trim(), (None, None) => part, _ => return Err(OdooError::Validation(format!("invalid address `{part}`"))) };
        if !valid_address(addr) { return Err(OdooError::Validation(format!("invalid address `{addr}`"))); }
        out.push(addr.to_lowercase());
    }
    Ok(out)
}

pub fn validate_message(m: &Message) -> Result<()> {
    let bad = |s: &str| Err(OdooError::Validation(s.into()));
    if m.to.is_empty() { return bad("at least one recipient is required"); }
    if m.to.len() + m.cc.len() + m.bcc.len() > 50 { return bad("too many recipients (max 50)"); }
    for a in m.to.iter().chain(&m.cc).chain(&m.bcc) { if !valid_address(a) { return bad(&format!("invalid address `{a}`")); } }
    if m.subject.trim().is_empty() || m.subject.chars().count() > 300 || !clean_line(&m.subject) { return bad("invalid subject"); }
    if m.text.trim().is_empty() && m.html.trim().is_empty() { return bad("the message body is empty"); }
    if m.text.len() > 1_000_000 || m.html.len() > 1_000_000 { return bad("the message body is too large (max 1 MB)"); }
    Ok(())
}

pub fn validate_settings(s: &EmailSettings) -> Result<&'static Provider> {
    let bad = |m: &str| Err(OdooError::Validation(m.into()));
    let p = provider(&s.provider).ok_or_else(|| OdooError::Validation(format!("unknown email provider `{}`", s.provider)))?;
    if !valid_address(&s.from_address) { return bad("a valid From address is required"); }
    if !clean_line(&s.from_name) || s.from_name.chars().count() > 100 { return bad("invalid From name"); }
    if !s.reply_to.is_empty() && !valid_address(&s.reply_to) { return bad("invalid Reply-To address"); }
    match p.kind {
        Kind::Http => {
            if s.api_key.is_empty() { return bad("an API key is required"); }
            if p.id == "mailgun" && (s.domain.is_empty() || !s.domain.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')) { return bad("a valid Mailgun sending domain is required"); }
            if !s.base_url.is_empty() && !(s.base_url.starts_with("http://") || s.base_url.starts_with("https://")) { return bad("base URL must start with http:// or https://"); }
        }
        Kind::Smtp => {
            if s.smtp_host.is_empty() || s.smtp_host.len() > 253 || !s.smtp_host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') { return bad("a valid SMTP host is required"); }
            if s.smtp_port == 0 { return bad("a valid SMTP port is required"); }
            if !["starttls", "ssl", "none"].contains(&s.smtp_security.as_str()) { return bad("SMTP security must be starttls, ssl or none"); }
        }
    }
    Ok(p)
}

// ───────────────────────── HTTP providers (pure builders) ─────────────────────────

pub enum Body { Json(J), Form(Vec<(String, String)>) }
pub struct HttpReq { pub url: String, pub headers: Vec<(String, String)>, pub body: Body }

pub fn base64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in input.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char); out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' }); out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn endpoint(p: &Provider, s: &EmailSettings) -> String {
    if !s.base_url.is_empty() { return s.base_url.trim_end_matches('/').to_string(); }
    if s.region == "eu" && !p.eu_base.is_empty() { p.eu_base.to_string() } else { p.base.to_string() }
}
fn display(name: &str, addr: &str) -> String { if name.is_empty() { addr.to_string() } else { format!("{} <{addr}>", name.replace(['"', '<', '>'], "")) } }

pub fn build_request(s: &EmailSettings, m: &Message) -> Result<HttpReq> {
    let p = validate_settings(s)?; validate_message(m)?;
    if p.kind != Kind::Http { return Err(OdooError::Validation("SMTP is not an HTTP provider".into())); }
    let base = endpoint(p, s);
    let hdr = |k: &str, v: &str| (k.to_string(), v.to_string());
    let objs = |v: &[String]| v.iter().map(|a| json!({"email": a})).collect::<Vec<_>>();
    Ok(match p.id {
        "sendgrid" => {
            let mut pers = json!({"to": objs(&m.to)}); if !m.cc.is_empty() { pers["cc"] = json!(objs(&m.cc)); } if !m.bcc.is_empty() { pers["bcc"] = json!(objs(&m.bcc)); }
            let mut content = vec![]; if !m.text.is_empty() { content.push(json!({"type": "text/plain", "value": m.text})); } if !m.html.is_empty() { content.push(json!({"type": "text/html", "value": m.html})); }
            let mut body = json!({"personalizations": [pers], "from": {"email": s.from_address, "name": s.from_name}, "subject": m.subject, "content": content});
            if s.from_name.is_empty() { body["from"] = json!({"email": s.from_address}); }
            if !s.reply_to.is_empty() { body["reply_to"] = json!({"email": s.reply_to}); }
            HttpReq { url: format!("{base}/v3/mail/send"), headers: vec![hdr("authorization", &format!("Bearer {}", s.api_key))], body: Body::Json(body) }
        }
        "mailgun" => {
            let mut f: Vec<(String, String)> = vec![("from".into(), display(&s.from_name, &s.from_address)), ("subject".into(), m.subject.clone())];
            for a in &m.to { f.push(("to".into(), a.clone())); } for a in &m.cc { f.push(("cc".into(), a.clone())); } for a in &m.bcc { f.push(("bcc".into(), a.clone())); }
            if !m.text.is_empty() { f.push(("text".into(), m.text.clone())); } if !m.html.is_empty() { f.push(("html".into(), m.html.clone())); }
            if !s.reply_to.is_empty() { f.push(("h:Reply-To".into(), s.reply_to.clone())); }
            HttpReq { url: format!("{base}/v3/{}/messages", s.domain), headers: vec![hdr("authorization", &format!("Basic {}", base64(format!("api:{}", s.api_key).as_bytes())))], body: Body::Form(f) }
        }
        "postmark" => {
            let mut b = json!({"From": display(&s.from_name, &s.from_address), "To": m.to.join(","), "Subject": m.subject, "MessageStream": "outbound"});
            if !m.cc.is_empty() { b["Cc"] = json!(m.cc.join(",")); } if !m.bcc.is_empty() { b["Bcc"] = json!(m.bcc.join(",")); }
            if !m.text.is_empty() { b["TextBody"] = json!(m.text); } if !m.html.is_empty() { b["HtmlBody"] = json!(m.html); }
            if !s.reply_to.is_empty() { b["ReplyTo"] = json!(s.reply_to); }
            HttpReq { url: format!("{base}/email"), headers: vec![hdr("x-postmark-server-token", &s.api_key), hdr("accept", "application/json")], body: Body::Json(b) }
        }
        "resend" => {
            let mut b = json!({"from": display(&s.from_name, &s.from_address), "to": m.to, "subject": m.subject});
            if !m.cc.is_empty() { b["cc"] = json!(m.cc); } if !m.bcc.is_empty() { b["bcc"] = json!(m.bcc); }
            if !m.text.is_empty() { b["text"] = json!(m.text); } if !m.html.is_empty() { b["html"] = json!(m.html); }
            if !s.reply_to.is_empty() { b["reply_to"] = json!(s.reply_to); }
            HttpReq { url: format!("{base}/emails"), headers: vec![hdr("authorization", &format!("Bearer {}", s.api_key))], body: Body::Json(b) }
        }
        "brevo" => {
            let mut b = json!({"sender": {"email": s.from_address, "name": s.from_name}, "to": objs(&m.to), "subject": m.subject});
            if s.from_name.is_empty() { b["sender"] = json!({"email": s.from_address}); }
            if !m.cc.is_empty() { b["cc"] = json!(objs(&m.cc)); } if !m.bcc.is_empty() { b["bcc"] = json!(objs(&m.bcc)); }
            if !m.text.is_empty() { b["textContent"] = json!(m.text); } if !m.html.is_empty() { b["htmlContent"] = json!(m.html); }
            if !s.reply_to.is_empty() { b["replyTo"] = json!({"email": s.reply_to}); }
            HttpReq { url: format!("{base}/v3/smtp/email"), headers: vec![hdr("api-key", &s.api_key), hdr("accept", "application/json")], body: Body::Json(b) }
        }
        other => return Err(OdooError::Validation(format!("provider `{other}` has no HTTP request builder"))),
    })
}

/// Interpret a provider response: the provider's message id on success, a readable (secret-free) error otherwise.
pub fn parse_response(provider_id: &str, status: u16, id_header: Option<&str>, body: &str, secret: &str) -> Result<String> {
    let j: J = serde_json::from_str(body).unwrap_or(J::Null);
    if !(200..300).contains(&status) {
        let msg = match provider_id {
            "sendgrid" => j["errors"][0]["message"].as_str(), "postmark" => j["Message"].as_str(), _ => j["message"].as_str().or_else(|| j["error"].as_str()),
        }.map(String::from).unwrap_or_else(|| body.chars().take(200).collect());
        let msg = if secret.len() >= 6 { msg.replace(secret, "***") } else { msg };
        return Err(OdooError::User(format!("{provider_id} rejected the message ({status}): {msg}")));
    }
    Ok(match provider_id {
        "sendgrid" => id_header.unwrap_or("accepted").to_string(),
        "mailgun" | "resend" => j["id"].as_str().unwrap_or("accepted").to_string(),
        "postmark" => { if j["ErrorCode"].as_i64().unwrap_or(0) != 0 { return Err(OdooError::User(format!("postmark: {}", j["Message"].as_str().unwrap_or("error")))); } j["MessageID"].as_str().unwrap_or("accepted").to_string() }
        "brevo" => j["messageId"].as_str().unwrap_or("accepted").to_string(),
        _ => "accepted".into(),
    })
}

// ───────────────────────── transport ─────────────────────────

pub trait Mailer: Send + Sync { fn send(&self, s: &EmailSettings, m: &Message) -> Result<String>; }

pub struct LiveMailer;
impl Mailer for LiveMailer {
    fn send(&self, s: &EmailSettings, m: &Message) -> Result<String> {
        let p = validate_settings(s)?; validate_message(m)?;
        match p.kind { Kind::Http => send_http(p, s, m), Kind::Smtp => send_smtp(s, m) }
    }
}

fn send_http(p: &Provider, s: &EmailSettings, m: &Message) -> Result<String> {
    let rq = build_request(s, m)?;
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(30)).build();
    let mut r = agent.post(&rq.url);
    for (k, v) in &rq.headers { r = r.set(k, v); }
    let res = match rq.body {
        Body::Json(j) => r.send_json(j),
        Body::Form(f) => { let pairs: Vec<(&str, &str)> = f.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect(); r.send_form(&pairs) }
    };
    let (status, id, body) = match res {
        Ok(resp) => { let id = resp.header("x-message-id").map(String::from); (resp.status(), id, resp.into_string().unwrap_or_default()) }
        Err(ureq::Error::Status(code, resp)) => (code, None, resp.into_string().unwrap_or_default()),
        Err(e) => return Err(OdooError::User(format!("cannot reach {}: {}", p.label, e.to_string().replace(&s.api_key, "***")))),
    };
    parse_response(p.id, status, id.as_deref(), &body, &s.api_key)
}

pub fn build_smtp_message(s: &EmailSettings, m: &Message) -> Result<lettre::Message> {
    use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
    let mb = |a: &str| a.parse::<Mailbox>().map_err(|e| OdooError::Validation(format!("invalid address `{a}`: {e}")));
    let from: Mailbox = if s.from_name.is_empty() { mb(&s.from_address)? } else { Mailbox::new(Some(s.from_name.replace(['"', '<', '>'], "")), s.from_address.parse().map_err(|e| OdooError::Validation(format!("invalid From address: {e}")))?) };
    let mut b = lettre::Message::builder().from(from).subject(m.subject.clone());
    for a in &m.to { b = b.to(mb(a)?); } for a in &m.cc { b = b.cc(mb(a)?); } for a in &m.bcc { b = b.bcc(mb(a)?); }
    if !s.reply_to.is_empty() { b = b.reply_to(mb(&s.reply_to)?); }
    let built = match (m.text.is_empty(), m.html.is_empty()) {
        (false, false) => b.multipart(MultiPart::alternative_plain_html(m.text.clone(), m.html.clone())),
        (true, false) => b.header(ContentType::TEXT_HTML).body(m.html.clone()),
        _ => b.header(ContentType::TEXT_PLAIN).body(m.text.clone()),
    };
    built.map_err(|e| OdooError::Validation(format!("cannot build the message: {e}")))
        .map(|x| { let _ = SinglePart::builder(); x })
}

fn send_smtp(s: &EmailSettings, m: &Message) -> Result<String> {
    use lettre::{transport::smtp::authentication::Credentials, SmtpTransport, Transport};
    let msg = build_smtp_message(s, m)?;
    let conn = |e: lettre::transport::smtp::Error| OdooError::User(format!("SMTP error ({}:{}): {}", s.smtp_host, s.smtp_port, e.to_string().replace(&s.smtp_password, "***")));
    let mut b = match s.smtp_security.as_str() { "ssl" => SmtpTransport::relay(&s.smtp_host).map_err(conn)?, "starttls" => SmtpTransport::starttls_relay(&s.smtp_host).map_err(conn)?, _ => SmtpTransport::builder_dangerous(&s.smtp_host) };
    b = b.port(s.smtp_port).timeout(Some(std::time::Duration::from_secs(30)));
    if !s.smtp_user.is_empty() { b = b.credentials(Credentials::new(s.smtp_user.clone(), s.smtp_password.clone())); }
    let resp = b.build().send(&msg).map_err(conn)?;
    let id = resp.message().next().map(|l| l.to_string()).unwrap_or_else(|| "accepted".into());
    Ok(id)
}

/// Plain-text alternative derived from HTML (tags stripped, common entities decoded, whitespace collapsed).
pub fn html_to_text(html: &str) -> String {
    let (mut out, mut in_tag) = (String::new(), false);
    for c in html.chars() { match c { '<' => { in_tag = true; out.push(' ') } '>' => in_tag = false, _ if !in_tag => out.push(c), _ => {} } }
    let out = out.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Welcome email for a new trial: (subject, text) localised for the UI languages; unknown languages get English.
pub fn welcome(lang: &str, name: &str, apps: &[String], app_url: &str) -> (String, String) {
    let l = lang.split(['_', '-']).next().unwrap_or("en");
    let (subject, hello, body, link, bye) = match l {
        "es" => ("Tu base de datos de prueba está lista", "Hola", "Tu base de datos de prueba gratuita está lista con estas aplicaciones", "Ábrela aquí", "El equipo de Odoo RS"),
        "fr" => ("Votre base de données d’essai est prête", "Bonjour", "Votre base de données d’essai gratuite est prête avec ces applications", "Ouvrez-la ici", "L’équipe Odoo RS"),
        "de" => ("Ihre Testdatenbank ist bereit", "Hallo", "Ihre kostenlose Testdatenbank ist mit diesen Apps bereit", "Hier öffnen", "Ihr Odoo-RS-Team"),
        "pt" => ("Seu banco de dados de teste está pronto", "Olá", "Seu banco de dados de teste gratuito está pronto com estes aplicativos", "Abra aqui", "A equipe Odoo RS"),
        "it" => ("Il tuo database di prova è pronto", "Ciao", "Il tuo database di prova gratuito è pronto con queste app", "Aprilo qui", "Il team Odoo RS"),
        "ru" => ("Ваша пробная база данных готова", "Здравствуйте", "Ваша бесплатная пробная база данных готова со следующими приложениями", "Открыть", "Команда Odoo RS"),
        "hi" => ("आपका ट्रायल डेटाबेस तैयार है", "नमस्ते", "आपका निःशुल्क ट्रायल डेटाबेस इन ऐप्स के साथ तैयार है", "यहाँ खोलें", "Odoo RS टीम"),
        "ar" => ("قاعدة بياناتك التجريبية جاهزة", "مرحبًا", "قاعدة بياناتك التجريبية المجانية جاهزة مع هذه التطبيقات", "افتحها هنا", "فريق Odoo RS"),
        "zh" => ("你的试用数据库已就绪", "你好", "你的免费试用数据库已就绪，包含以下应用", "在此打开", "Odoo RS 团队"),
        "ja" => ("トライアル用データベースの準備ができました", "こんにちは", "無料トライアル用データベースが次のアプリで準備できました", "ここから開く", "Odoo RS チーム"),
        _ => ("Your trial database is ready", "Hello", "Your free trial database is ready with these apps", "Open it here", "The Odoo RS team"),
    };
    let first = name.split_whitespace().next().unwrap_or(name).replace(['\r', '\n'], " ");
    let mut text = format!("{hello} {first},\n\n{body}: {}.\n", apps.join(", "));
    if app_url.starts_with("http://") || app_url.starts_with("https://") { text.push_str(&format!("\n{link}: {app_url}\n")); }
    text.push_str(&format!("\n{bye}\n"));
    (subject.to_string(), text)
}

#[cfg(test)]
mod tests;
