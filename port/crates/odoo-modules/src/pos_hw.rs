//! POS hardware bridge: raw ESC/POS over TCP (port 9100) to receipt / kitchen printers configured on the register.
//! Only addresses configured on the register or its printers can be reached, so this can never be used as a general TCP relay.
use crate::util::*;
use odoo_core::orm::Env;
use odoo_core::{OdooError, Result, Row, Rules, Value};
use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

fn norm(a: &str) -> Option<String> {
    let a = a.trim().trim_start_matches("tcp://").trim_end_matches('/'); if a.is_empty() { return None; }
    Some(if a.rsplit_once(':').map_or(false, |(_, p)| p.parse::<u16>().is_ok()) { a.to_string() } else { format!("{a}:9100") })
}
/// Every printer address the register may talk to: (label, host:port, product categories).
pub fn targets(env: &Env, cfg: &Row) -> Vec<(String, String, Vec<i64>)> {
    let mut out = vec![];
    let mut add = |label: String, v: Option<String>, cats: Vec<i64>| if let Some(a) = v.as_deref().and_then(norm) { if !out.iter().any(|(_, x, _): &(String, String, Vec<i64>)| *x == a) { out.push((label, a, cats)); } };
    for k in ["epson_printer_ip", "proxy_ip"] { add("Receipt printer".into(), text(cfg, k), vec![]); }
    for id in ids(cfg, "printer_ids") {
        if let Ok(p) = rec(env, "pos.printer", id) { let cats = ids(&p, "product_categories_ids"); for k in ["epson_printer_ip", "proxy_ip"] { add(text(&p, "name").unwrap_or_default(), text(&p, k), cats.clone()); } }
    }
    out
}
fn unhex(s: &str) -> Result<Vec<u8>> {
    if s.len() % 2 != 0 || s.len() > 2_000_000 { return Err(OdooError::User("Invalid print data".into())); }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| OdooError::User("Invalid print data".into()))).collect()
}

pub fn rules() -> Rules {
    Rules::default().action("pos.config", "print_raw", |env, ids, kw| {
        let e = env.sudo(); let cfg = rec(&e, "pos.config", *ids.first().unwrap_or(&0))?;
        let want = norm(&text(kw, "target").unwrap_or_default()).ok_or_else(|| OdooError::User("No printer selected".into()))?;
        if !targets(&e, &cfg).iter().any(|(_, a, _)| *a == want) { return Err(OdooError::User("That printer is not configured on this register".into())); }
        let bytes = unhex(&text(kw, "data").unwrap_or_default())?;
        let addr = want.to_socket_addrs().map_err(|e| OdooError::User(format!("Cannot resolve the printer address: {e}")))?.next().ok_or_else(|| OdooError::User("Cannot resolve the printer address".into()))?;
        let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| OdooError::User(format!("Printer unreachable ({want}): {e}")))?;
        s.set_write_timeout(Some(Duration::from_secs(5))).ok();
        s.write_all(&bytes).and_then(|_| s.flush()).map_err(|e| OdooError::User(format!("Printing failed: {e}")))?;
        Ok(Value::Int(bytes.len() as i64))
    })
}
