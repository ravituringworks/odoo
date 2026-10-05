use super::*;
use std::collections::HashMap;
use std::io::{Read, Write, BufRead, BufReader};
use std::net::TcpListener;
use std::thread::JoinHandle;

fn settings(provider: &str) -> EmailSettings {
    EmailSettings { enabled: true, provider: provider.into(), from_address: "noreply@example.com".into(), from_name: "Odoo RS".into(), reply_to: "help@example.com".into(), api_key: "SECRET-KEY-123456".into(), domain: "mg.example.com".into(), region: "us".into(),
        smtp_host: "127.0.0.1".into(), smtp_port: 2525, smtp_security: "none".into(), ..Default::default() }
}
fn msg() -> Message { Message { to: vec!["ada@example.com".into(), "bob@example.com".into()], cc: vec!["cc@example.com".into()], bcc: vec!["bcc@example.com".into()], subject: "Hello Ada".into(), text: "Plain body".into(), html: "<p>HTML body</p>".into() } }

struct Captured { request_line: String, headers: HashMap<String, String>, body: String }
/// One-shot fake HTTP server: records the request, answers with the given status/headers/body.
fn serve(status: u16, extra: &str, body: &str) -> (String, JoinHandle<Captured>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap(); let url = format!("http://{}", l.local_addr().unwrap());
    let (extra, body) = (extra.to_string(), body.to_string());
    let h = std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap(); let mut r = BufReader::new(s.try_clone().unwrap());
        let mut line = String::new(); r.read_line(&mut line).unwrap(); let mut headers = HashMap::new();
        loop { let mut h = String::new(); r.read_line(&mut h).unwrap(); if h.trim().is_empty() { break; } if let Some((k, v)) = h.split_once(':') { headers.insert(k.trim().to_lowercase(), v.trim().to_string()); } }
        let n: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0); let mut buf = vec![0u8; n]; r.read_exact(&mut buf).unwrap();
        write!(s, "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        Captured { request_line: line.trim().to_string(), headers, body: String::from_utf8_lossy(&buf).to_string() }
    });
    (url, h)
}

#[test] fn addresses_are_validated_and_parsed() {
    assert!(valid_address("a.b+tag@sub.example.com")); for bad in ["", "a", "a@b", "a@@b.c", "a b@c.de", "<a@b.cd>", "a@b..cd", "a@.cd", "a@b.cd,e@f.gh", "\"x\"@b.cd"] { assert!(!valid_address(bad), "{bad}"); }
    assert_eq!(parse_addresses("Ada <Ada@Example.com>, bob@example.com; c@d.ef").unwrap(), vec!["ada@example.com", "bob@example.com", "c@d.ef"]);
    assert!(parse_addresses("ok@x.yz, nope").is_err()); assert!(parse_addresses("a@b.cd\r\nBcc: evil@x.yz").is_err(), "header injection via address list");
}
#[test] fn messages_are_validated() {
    assert!(validate_message(&msg()).is_ok());
    let with = |f: &dyn Fn(&mut Message)| { let mut m = msg(); f(&mut m); validate_message(&m).is_err() };
    assert!(with(&|m| m.to.clear())); assert!(with(&|m| m.subject = "Hi\r\nBcc: evil@x.yz".into())); assert!(with(&|m| m.subject = " ".into())); assert!(with(&|m| { m.text.clear(); m.html.clear(); }));
    assert!(with(&|m| m.to = (0..51).map(|i| format!("u{i}@example.com")).collect())); assert!(with(&|m| m.to = vec!["bad".into()])); assert!(with(&|m| m.text = "x".repeat(1_000_001)));
}
#[test] fn settings_are_validated_per_provider() {
    for p in ["sendgrid", "mailgun", "postmark", "resend", "brevo", "smtp"] { assert!(validate_settings(&settings(p)).is_ok(), "{p}"); }
    let mut s = settings("sendgrid"); s.api_key.clear(); assert!(validate_settings(&s).is_err()); s = settings("mailgun"); s.domain = "evil.com/../x".into(); assert!(validate_settings(&s).is_err());
    s = settings("sendgrid"); s.from_address = "nope".into(); assert!(validate_settings(&s).is_err()); s = settings("sendgrid"); s.from_name = "A\r\nBcc: x@y.zz".into(); assert!(validate_settings(&s).is_err());
    s = settings("smtp"); s.smtp_host = "bad host".into(); assert!(validate_settings(&s).is_err()); s = settings("smtp"); s.smtp_security = "plain".into(); assert!(validate_settings(&s).is_err()); s = settings("sendgrid"); s.base_url = "file:///x".into(); assert!(validate_settings(&s).is_err());
    assert!(validate_settings(&EmailSettings { provider: "nope".into(), ..settings("sendgrid") }).is_err());
}
#[test] fn base64_matches_rfc4648() { assert_eq!(base64(b""), ""); assert_eq!(base64(b"f"), "Zg=="); assert_eq!(base64(b"fo"), "Zm8="); assert_eq!(base64(b"foo"), "Zm9v"); assert_eq!(base64(b"api:key-1"), "YXBpOmtleS0x"); }

#[test] fn each_provider_builds_the_documented_request() {
    let m = msg();
    let r = build_request(&settings("sendgrid"), &m).unwrap(); let Body::Json(b) = &r.body else { panic!() };
    assert_eq!(r.url, "https://api.sendgrid.com/v3/mail/send"); assert_eq!(r.headers[0], ("authorization".to_string(), "Bearer SECRET-KEY-123456".to_string()));
    assert_eq!((b["personalizations"][0]["to"][1]["email"].as_str(), b["personalizations"][0]["cc"][0]["email"].as_str(), b["personalizations"][0]["bcc"][0]["email"].as_str()), (Some("bob@example.com"), Some("cc@example.com"), Some("bcc@example.com")));
    assert_eq!((b["from"]["email"].as_str(), b["from"]["name"].as_str(), b["reply_to"]["email"].as_str(), b["subject"].as_str()), (Some("noreply@example.com"), Some("Odoo RS"), Some("help@example.com"), Some("Hello Ada")));
    assert_eq!(b["content"][0]["type"], "text/plain"); assert_eq!(b["content"][1]["type"], "text/html");
    let mut eu = settings("sendgrid"); eu.region = "eu".into(); assert!(build_request(&eu, &m).unwrap().url.starts_with("https://api.eu.sendgrid.com/"));

    let r = build_request(&settings("mailgun"), &m).unwrap(); let Body::Form(f) = &r.body else { panic!() };
    assert_eq!(r.url, "https://api.mailgun.net/v3/mg.example.com/messages"); assert_eq!(r.headers[0].1, format!("Basic {}", base64(b"api:SECRET-KEY-123456")));
    assert_eq!(f.iter().filter(|(k, _)| k == "to").count(), 2); assert!(f.contains(&("from".into(), "Odoo RS <noreply@example.com>".into())) && f.contains(&("h:Reply-To".into(), "help@example.com".into())));
    let mut eu = settings("mailgun"); eu.region = "eu".into(); assert!(build_request(&eu, &m).unwrap().url.starts_with("https://api.eu.mailgun.net/v3/mg.example.com"));

    let r = build_request(&settings("postmark"), &m).unwrap(); let Body::Json(b) = &r.body else { panic!() };
    assert_eq!(r.url, "https://api.postmarkapp.com/email"); assert!(r.headers.contains(&("x-postmark-server-token".into(), "SECRET-KEY-123456".into())));
    assert_eq!((b["To"].as_str(), b["Cc"].as_str(), b["TextBody"].as_str(), b["HtmlBody"].as_str(), b["From"].as_str()), (Some("ada@example.com,bob@example.com"), Some("cc@example.com"), Some("Plain body"), Some("<p>HTML body</p>"), Some("Odoo RS <noreply@example.com>")));

    let r = build_request(&settings("resend"), &m).unwrap(); let Body::Json(b) = &r.body else { panic!() };
    assert_eq!(r.url, "https://api.resend.com/emails"); assert_eq!((b["to"][0].as_str(), b["reply_to"].as_str(), b["from"].as_str()), (Some("ada@example.com"), Some("help@example.com"), Some("Odoo RS <noreply@example.com>")));

    let r = build_request(&settings("brevo"), &m).unwrap(); let Body::Json(b) = &r.body else { panic!() };
    assert_eq!(r.url, "https://api.brevo.com/v3/smtp/email"); assert!(r.headers.contains(&("api-key".into(), "SECRET-KEY-123456".into())));
    assert_eq!((b["sender"]["email"].as_str(), b["to"][1]["email"].as_str(), b["textContent"].as_str(), b["htmlContent"].as_str(), b["replyTo"]["email"].as_str()), (Some("noreply@example.com"), Some("bob@example.com"), Some("Plain body"), Some("<p>HTML body</p>"), Some("help@example.com")));
    // display names cannot smuggle extra headers/addresses
    let mut s = settings("resend"); s.from_name = "Evil <x@y.zz>".into(); let Body::Json(b) = build_request(&s, &m).unwrap().body else { panic!() }; assert_eq!(b["from"], "Evil x@y.zz <noreply@example.com>");
}

#[test] fn responses_are_parsed_and_errors_never_leak_the_key() {
    assert_eq!(parse_response("sendgrid", 202, Some("abc"), "", "k").unwrap(), "abc"); assert_eq!(parse_response("mailgun", 200, None, r#"{"id":"<m1@mg>","message":"Queued"}"#, "k").unwrap(), "<m1@mg>");
    assert_eq!(parse_response("postmark", 200, None, r#"{"MessageID":"pm-1","ErrorCode":0}"#, "k").unwrap(), "pm-1"); assert!(parse_response("postmark", 200, None, r#"{"ErrorCode":300,"Message":"Invalid email request"}"#, "k").is_err());
    assert_eq!(parse_response("resend", 200, None, r#"{"id":"re-1"}"#, "k").unwrap(), "re-1"); assert_eq!(parse_response("brevo", 201, None, r#"{"messageId":"<b1>"}"#, "k").unwrap(), "<b1>");
    let e = parse_response("sendgrid", 401, None, r#"{"errors":[{"message":"The provided authorization grant is invalid, expired, or revoked for key SECRET-KEY-123456"}]}"#, "SECRET-KEY-123456").unwrap_err().to_string();
    assert!(e.contains("401") && e.contains("invalid") && !e.contains("SECRET-KEY-123456"), "{e}");
    assert!(parse_response("resend", 422, None, r#"{"message":"The from address is not verified"}"#, "k").unwrap_err().to_string().contains("not verified"));
    assert!(parse_response("mailgun", 500, None, "<html>oops</html>", "k").unwrap_err().to_string().contains("oops"));
}

#[test] fn live_http_round_trips_for_every_http_provider() {
    for (p, status, extra, body, want_id) in [("sendgrid", 202, "X-Message-Id: sg-1\r\n", "", "sg-1"), ("mailgun", 200, "", r#"{"id":"<mg-1>","message":"Queued. Thank you."}"#, "<mg-1>"), ("postmark", 200, "", r#"{"MessageID":"pm-1","ErrorCode":0}"#, "pm-1"), ("resend", 200, "", r#"{"id":"re-1"}"#, "re-1"), ("brevo", 201, "", r#"{"messageId":"<bv-1>"}"#, "<bv-1>")] {
        let (url, h) = serve(status, extra, body); let mut s = settings(p); s.base_url = url;
        let id = LiveMailer.send(&s, &msg()).unwrap_or_else(|e| panic!("{p}: {e}")); assert_eq!(id, want_id, "{p}");
        let c = h.join().unwrap(); assert!(c.request_line.starts_with("POST /"), "{p}: {}", c.request_line);
        let path = c.request_line.split(' ').nth(1).unwrap(); assert_eq!(path, match p { "sendgrid" => "/v3/mail/send", "mailgun" => "/v3/mg.example.com/messages", "postmark" => "/email", "resend" => "/emails", _ => "/v3/smtp/email" }, "{p}");
        assert!(c.body.contains("Hello") || c.body.contains("Hello%20Ada") || c.body.contains("Hello+Ada"), "{p}: subject on the wire: {}", c.body);
        if p == "mailgun" { assert_eq!(c.headers["content-type"], "application/x-www-form-urlencoded"); assert!(c.body.matches("to=").count() >= 2); } else { assert!(c.headers["content-type"].starts_with("application/json")); }
        let auth = c.headers.get("authorization").or(c.headers.get("x-postmark-server-token")).or(c.headers.get("api-key")).expect("auth header sent"); assert!(auth.contains("SECRET-KEY-123456") || auth.starts_with("Basic "), "{p}");
    }
    // a provider error surfaces as a clean message without the key
    let (url, h) = serve(401, "", r#"{"errors":[{"message":"bad key SECRET-KEY-123456"}]}"#); let mut s = settings("sendgrid"); s.base_url = url;
    let e = LiveMailer.send(&s, &msg()).unwrap_err().to_string(); h.join().unwrap(); assert!(e.contains("401") && !e.contains("SECRET-KEY-123456"), "{e}");
    // unreachable provider
    let mut s = settings("resend"); s.base_url = "http://127.0.0.1:1".into(); assert!(LiveMailer.send(&s, &msg()).unwrap_err().to_string().contains("cannot reach"));
}

/// Scripted SMTP server: records the envelope and DATA, answers like a normal relay.
fn smtp_server() -> (u16, JoinHandle<(String, Vec<String>, String)>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap(); let port = l.local_addr().unwrap().port();
    let h = std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap(); let mut r = BufReader::new(s.try_clone().unwrap());
        s.write_all(b"220 fake ESMTP ready\r\n").unwrap();
        let (mut from, mut rcpts, mut data) = (String::new(), vec![], String::new());
        loop {
            let mut line = String::new(); if r.read_line(&mut line).unwrap() == 0 { break; } let up = line.to_uppercase();
            if up.starts_with("EHLO") || up.starts_with("HELO") { s.write_all(b"250-fake\r\n250 8BITMIME\r\n").unwrap(); }
            else if up.starts_with("MAIL FROM") { from = line.trim().to_string(); s.write_all(b"250 ok\r\n").unwrap(); }
            else if up.starts_with("RCPT TO") { rcpts.push(line.trim().to_string()); s.write_all(b"250 ok\r\n").unwrap(); }
            else if up.starts_with("DATA") { s.write_all(b"354 go\r\n").unwrap(); loop { let mut d = String::new(); r.read_line(&mut d).unwrap(); if d == ".\r\n" { break; } data.push_str(&d); } s.write_all(b"250 2.0.0 queued as FAKE123\r\n").unwrap(); }
            else if up.starts_with("QUIT") { s.write_all(b"221 bye\r\n").unwrap(); break; }
            else { s.write_all(b"250 ok\r\n").unwrap(); }
        }
        (from, rcpts, data)
    });
    (port, h)
}

#[test] fn smtp_delivers_a_multipart_message_through_a_real_connection() {
    let (port, h) = smtp_server(); let mut s = settings("smtp"); s.smtp_port = port;
    let out = LiveMailer.send(&s, &msg()).unwrap(); assert!(out.contains("FAKE123"), "{out}");
    let (from, rcpts, data) = h.join().unwrap();
    assert!(from.contains("<noreply@example.com>"), "{from}"); assert_eq!(rcpts.len(), 4, "to x2 + cc + bcc are all in the envelope: {rcpts:?}");
    assert!(rcpts.iter().any(|r| r.contains("bcc@example.com")), "bcc delivered via envelope");
    assert!(data.contains("Subject: Hello Ada") && data.contains("Plain body") && data.contains("<p>HTML body</p>") && data.contains("multipart/alternative"), "{data}");
    assert!(data.contains("Reply-To: help@example.com") && data.contains("From: \"Odoo RS\" <noreply@example.com>") || data.contains("From: Odoo RS <noreply@example.com>"), "{data}");
    assert!(!data.to_lowercase().contains("bcc:"), "the Bcc header must not appear in the message itself");
    let mut bad = settings("smtp"); bad.smtp_port = 1; assert!(LiveMailer.send(&bad, &msg()).unwrap_err().to_string().contains("SMTP error"));
}

#[test] fn html_is_flattened_to_text() { assert_eq!(html_to_text("<p>Hello&nbsp;<b>Ada</b> &amp; co</p>\n<p>Bye</p>"), "Hello Ada & co Bye"); }
#[test] fn welcome_email_is_localised_and_safe() {
    let (s, b) = welcome("fr_FR", "Ada Lovelace\r\nBcc: x@y.zz", &["CRM".into(), "Sales".into()], "https://erp.example.com");
    assert_eq!(s, "Votre base de données d’essai est prête"); assert!(b.starts_with("Bonjour Ada,") && b.contains("CRM, Sales") && b.contains("https://erp.example.com") && !b.contains('\r'), "{b}");
    assert_eq!(welcome("xx", "Bob", &["CRM".into()], "javascript:alert(1)").1.contains("javascript"), false, "only http(s) links are included");
    assert_eq!(welcome("en", "Bob", &[], "").0, "Your trial database is ready");
}
