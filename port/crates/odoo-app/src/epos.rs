//! Epson ePOS-Print (XML over HTTP) bridge. The browser sends ePOS XML commands; the server posts them to the printer's
//! `cgi-bin/epos/service.cgi`. Only printers configured on the register (`epson_printer_ip`) are reachable.
use crate::{App, Request, Runtime};
use odoo_core::{orm::{self, Env}, store, OdooError, Result};
use serde_json::{json, Value as J};

impl App {
    pub(crate) fn epos_print(&self, rt: &Runtime, req: &Request) -> Result<J> {
        let a = req.args.first().cloned().unwrap_or(J::Null);
        let (cfg_id, target, xml) = (a["config_id"].as_i64().unwrap_or(0), a["target"].as_str().unwrap_or("").trim().trim_start_matches("http://").trim_end_matches('/').to_string(), a["xml"].as_str().unwrap_or("").to_string());
        if xml.len() > 200_000 || !xml.trim_start().starts_with("<epos-print") { return Err(OdooError::User("Invalid ePOS document".into())); }
        let mut allowed = false;
        store::run(self.store.as_ref(), |c| {
            let env = Env::new(&rt.reg, c, &rt.rules, self.sec.as_ref(), req.uid);
            let cfg = orm::browse_raw(&env.sudo(), env.reg.model("pos.config")?, cfg_id)?;
            allowed = odoo_modules::pos_hw::targets(&env.sudo(), &cfg).iter().any(|(_, t, _, k)| *k == "epos" && *t == target); Ok(())
        })?;
        if !allowed { return Err(OdooError::User("That printer is not configured on this register".into())); }
        let envelope = format!("<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\"><s:Body>{xml}</s:Body></s:Envelope>");
        let http = self.terminal_http.lock().unwrap().clone();
        let (st, body) = http.send(&crate::terminal::HttpReq { method: "POST", url: format!("http://{target}/cgi-bin/epos/service.cgi?devid=local_printer&timeout=10000"), headers: vec![("Content-Type".into(), "text/xml; charset=utf-8".into()), ("SOAPAction".into(), "\"\"".into())], form: vec![], json: None, raw: Some(envelope), timeout_s: 15 })
            .map_err(|e| OdooError::User(format!("Printer unreachable ({target}): {e}")))?;
        if st >= 300 { return Err(OdooError::User(format!("The printer answered HTTP {st}"))); }
        let ok = body.contains("success=\"true\"") || body.contains("success='true'");
        if !ok { let code = body.split("code=\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or(""); return Err(OdooError::User(format!("The printer reported an error{}", if code.is_empty() { String::new() } else { format!(" ({code})") }))); }
        Ok(json!({"ok": true}))
    }
}
