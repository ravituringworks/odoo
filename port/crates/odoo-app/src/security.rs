//! Authentication, sessions, group-based ACL and record rules (ir.rule). Fails closed on rules it cannot evaluate.
use odoo_core::orm::{Access, AccessEntry, AccessTable, Env, Op, RecordRules};
use odoo_core::schema::table_of;
use odoo_core::{orm, Domain, OdooError, Registry, Result, Value};
use serde_json::Value as J;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Mutex, RwLock};

#[derive(Clone, Debug)]
pub struct RuleEntry { pub model: String, pub domain: String, pub groups: Vec<String>, pub read: bool, pub write: bool, pub create: bool, pub unlink: bool }

#[derive(Default)]
pub struct Security {
    pub acl: RwLock<AccessTable>,
    pub rules: RwLock<Vec<RuleEntry>>,
    sessions: Mutex<HashMap<String, Session>>,
}

/// One login: the user plus the companies the session works in (`company_id` = active, first of `allowed`).
#[derive(Clone, Debug)]
pub struct Session { pub uid: i64, pub company_id: i64, pub allowed: Vec<i64> }

pub const AUTH_DDL: &str = "CREATE TABLE IF NOT EXISTS odoo_auth (user_id INTEGER PRIMARY KEY, salt VARCHAR(64), hash VARCHAR(128))";

pub(crate) fn random_hex(n: usize) -> String {
    use std::io::Read;
    let mut b = vec![0u8; n];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        for (i, x) in b.iter_mut().enumerate() { *x = ((t >> ((i % 16) * 8)) as u8) ^ (i as u8).wrapping_mul(31); }
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn hash_password(salt: &str, pw: &str) -> String {
    let mut h = Sha256::digest(format!("{salt}:{pw}").as_bytes()).to_vec();
    for _ in 0..30_000 { let mut d = Sha256::new(); d.update(&h); d.update(salt.as_bytes()); h = d.finalize().to_vec(); }
    h.iter().map(|x| format!("{x:02x}")).collect()
}
fn ct_eq(a: &str, b: &str) -> bool { a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0 }

pub fn set_password(env: &Env, uid: i64, pw: &str) -> Result<()> {
    let salt = random_hex(16);
    let h = hash_password(&salt, pw);
    env.conn.execute(&format!("DELETE FROM odoo_auth WHERE user_id = {}", env.conn.dialect().placeholder(1)), &[Value::Int(uid)])?;
    let (p1, p2, p3) = (env.conn.dialect().placeholder(1), env.conn.dialect().placeholder(2), env.conn.dialect().placeholder(3));
    env.conn.execute(&format!("INSERT INTO odoo_auth (user_id, salt, hash) VALUES ({p1}, {p2}, {p3})"), &[Value::Int(uid), Value::Text(salt), Value::Text(h)])?;
    Ok(())
}

impl Security {
    pub fn new(reg: &Registry, dir: &Path, modules: &[String]) -> Security {
        Security { acl: RwLock::new(load_acl(reg, dir, modules)), rules: RwLock::new(load_rules(reg, dir, modules)), sessions: Mutex::new(HashMap::new()) }
    }

    /// Re-read ACL entries and record rules after modules were installed; sessions and group membership are kept.
    pub fn reload(&self, reg: &Registry, dir: &Path, modules: &[String]) {
        let fresh = load_acl(reg, dir, modules);
        { let mut a = self.acl.write().unwrap(); a.entries = fresh.entries; }
        *self.rules.write().unwrap() = load_rules(reg, dir, modules);
    }

    /// Verify credentials and open a session; returns (token, uid).
    pub fn login(&self, env: &Env, login: &str, pw: &str) -> Result<Option<(String, i64)>> {
        let e = env.sudo();
        let ph = e.conn.dialect().placeholder(1);
        let u = e.conn.query(&format!("SELECT a.user_id AS uid, a.salt AS salt, a.hash AS hash FROM odoo_auth a JOIN res_users u ON u.id = a.user_id WHERE u.login = {ph} AND u.active <> {}", e.conn.dialect().bool_lit(false)), &[Value::Text(login.into())])?;
        let Some(r) = u.first() else { let _ = hash_password("x", pw); return Ok(None) };  // dummy hash to blunt timing oracle
        let (uid, salt, hash) = (r["uid"].as_i64().unwrap_or(0), r["salt"].as_str().unwrap_or(""), r["hash"].as_str().unwrap_or(""));
        if !ct_eq(&hash_password(salt, pw), hash) { return Ok(None); }
        let tok = random_hex(32);
        let (home, _) = self.user_companies(env, uid)?;
        self.sessions.lock().unwrap().insert(tok.clone(), Session { uid, company_id: home, allowed: vec![home] });
        self.refresh_groups(env)?;
        Ok(Some((tok, uid)))
    }
    /// Open a session for `uid` without credentials. Only for callers that have already proven ownership (the portal's "Open database").
    pub fn open_session(&self, uid: i64) -> String { let tok = random_hex(32); self.sessions.lock().unwrap().insert(tok.clone(), Session { uid, company_id: 1, allowed: vec![1] }); tok }
    pub fn uid_of(&self, token: &str) -> Option<i64> { self.sessions.lock().unwrap().get(token).map(|s| s.uid) }
    pub fn session(&self, token: &str) -> Option<Session> { self.sessions.lock().unwrap().get(token).cloned() }

    /// (default company, every company the user may switch to): `res.users.company_id` plus `res.users.company_ids`.
    pub fn user_companies(&self, env: &Env, uid: i64) -> Result<(i64, Vec<i64>)> {
        let e = env.sudo();
        let ph = e.conn.dialect().placeholder(1);
        let home = e.conn.query(&format!("SELECT company_id AS c FROM res_users WHERE id = {ph}"), &[Value::Int(uid)])?.first().and_then(|r| r["c"].as_i64()).unwrap_or(1);
        let mut all = vec![home];
        if let Some((rt, c1, c2)) = e.reg.field("res.users", "company_ids").ok().and_then(|f| f.relation_table_name("res.users", "company_ids", e.reg)) {
            for r in e.conn.query(&format!("SELECT {c2} AS c FROM {rt} WHERE {c1} = {ph}"), &[Value::Int(uid)])? { if let Some(c) = r["c"].as_i64() { if !all.contains(&c) { all.push(c); } } }
        }
        Ok((home, all))
    }

    /// Choose the companies a session works in; the first id becomes the active company. Every id must be one of the user's own.
    pub fn switch_company(&self, env: &Env, token: &str, ids: &[i64]) -> Result<Session> {
        let cur = self.session(token).ok_or_else(|| OdooError::AccessDenied { op: "session".into(), model: "res.company".into(), uid: 0 })?;
        let (_, mine) = self.user_companies(env, cur.uid)?;
        if ids.is_empty() { return Err(OdooError::User("Select at least one company.".into())); }
        if let Some(bad) = ids.iter().find(|i| !mine.contains(i)) { return Err(OdooError::User(format!("You are not allowed to access company {bad}."))); }
        let mut allowed: Vec<i64> = vec![]; for i in ids { if !allowed.contains(i) { allowed.push(*i); } }
        let s = Session { uid: cur.uid, company_id: allowed[0], allowed };
        self.sessions.lock().unwrap().insert(token.to_string(), s.clone());
        Ok(s)
    }
    pub fn logout(&self, token: &str) { self.sessions.lock().unwrap().remove(token); }

    /// Reload user -> group-xmlid membership from res.users.groups_id + ir.model.data.
    pub fn refresh_groups(&self, env: &Env) -> Result<()> {
        let e = env.sudo();
        let Ok(f) = e.reg.field("res.users", "groups_id") else { return Ok(()) };
        let Some((rt, c1, c2)) = f.relation_table_name("res.users", "groups_id", e.reg) else { return Ok(()) };
        let names: BTreeMap<i64, String> = e.conn.query("SELECT module, name, res_id FROM ir_model_data WHERE model = 'res.groups'", &[])?.iter().filter_map(|r| Some((r["res_id"].as_i64()?, format!("{}.{}", r["module"].as_str()?, r["name"].as_str()?)))).collect();
        let mut ug: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new(); let mut ugi: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
        for r in e.conn.query(&format!("SELECT {c1} AS u, {c2} AS g FROM {rt}"), &[])? { if let (Some(u), Some(gid)) = (r["u"].as_i64(), r["g"].as_i64()) { ugi.entry(u).or_default().push(gid); if let Some(g) = names.get(&gid) { ug.entry(u).or_default().insert(g.clone()); } } }
        { let mut a = self.acl.write().unwrap(); a.user_groups = ug; a.user_group_ids = ugi; }
        Ok(())
    }
    /// The superuser or a member of `base.group_system`.
    pub fn is_admin(&self, uid: i64) -> bool { uid == 1 || self.groups_of(uid).contains("base.group_system") }
    fn groups_of(&self, uid: i64) -> BTreeSet<String> { self.acl.read().unwrap().user_groups.get(&uid).cloned().unwrap_or_default() }
}

impl Access for Security { fn check(&self, uid: i64, model: &str, op: Op) -> bool { self.acl.read().unwrap().check(uid, model, op) } }

fn qualify(module: &str, x: &str) -> String { if x.contains('.') { x.to_string() } else { format!("{module}.{x}") } }

fn load_acl(reg: &Registry, dir: &Path, modules: &[String]) -> AccessTable {
    let by_table: BTreeMap<String, String> = reg.models.keys().map(|n| (table_of(n), n.clone())).collect();
    let mut t = AccessTable::default();
    for m in modules {
        let Some(d) = std::fs::read_to_string(dir.join(format!("{m}.json"))).ok().and_then(|s| serde_json::from_str::<J>(&s).ok()) else { continue };
        for a in d["access"].as_array().into_iter().flatten() {
            let Some(x) = a["model_xmlid"].as_str() else { continue };
            let Some(model) = by_table.get(x.rsplit('.').next().unwrap_or(x).trim_start_matches("model_")) else { continue };
            t.entries.push(AccessEntry { model: model.clone(), group: a["group"].as_str().map(|g| qualify(m, g)), read: a["read"].as_bool().unwrap_or(false), write: a["write"].as_bool().unwrap_or(false), create: a["create"].as_bool().unwrap_or(false), unlink: a["unlink"].as_bool().unwrap_or(false) });
        }
    }
    t
}

fn truthy_perm(v: &J) -> bool { match v { J::Null => true, J::Bool(b) => *b, J::String(s) => matches!(s.as_str(), "1" | "True" | "true"), J::Number(n) => n.as_i64() != Some(0), _ => true } }

fn load_rules(reg: &Registry, dir: &Path, modules: &[String]) -> Vec<RuleEntry> {
    let by_table: BTreeMap<String, String> = reg.models.keys().map(|n| (table_of(n), n.clone())).collect();
    let mut out = vec![];
    for m in modules {
        let Some(d) = std::fs::read_to_string(dir.join(format!("{m}.json"))).ok().and_then(|s| serde_json::from_str::<J>(&s).ok()) else { continue };
        for r in d["records"].as_array().into_iter().flatten().filter(|r| r["model"] == "ir.rule") {
            let v = &r["values"];
            let Some(mx) = v["model_id"]["$ref"].as_str() else { continue };
            let Some(model) = by_table.get(mx.rsplit('.').next().unwrap_or(mx).trim_start_matches("model_")) else { continue };
            let mut groups = vec![];
            for g in v["groups"].as_array().into_iter().flatten() { if let Some(x) = g.as_array().and_then(|a| a.get(1)).and_then(|x| x["$ref"].as_str()) { groups.push(qualify(m, x)); } else if let Some(x) = g.as_array().and_then(|a| a.get(2)).and_then(|a| a.as_array()) { for y in x { if let Some(s) = y["$ref"].as_str() { groups.push(qualify(m, s)); } } } }
            out.push(RuleEntry { model: model.clone(), domain: v["domain_force"].as_str().unwrap_or("[(1,'=',1)]").to_string(), groups, read: truthy_perm(&v["perm_read"]), write: truthy_perm(&v["perm_write"]), create: truthy_perm(&v["perm_create"]), unlink: truthy_perm(&v["perm_unlink"]) });
        }
    }
    out
}

/// Evaluate an Odoo domain_force string for `uid` using a safe textual subset; None => cannot evaluate.
pub fn eval_domain(src: &str, uid: i64) -> Option<Domain> { eval_domain_with(src, uid, &[]) }

pub fn eval_domain_with(src: &str, uid: i64, group_ids: &[i64]) -> Option<Domain> { eval_domain_in(src, uid, group_ids, 1, &[1]) }

/// As `eval_domain_with`, for a session working in `active` (`user.company_id`) with `companies` allowed (`company_ids`).
pub fn eval_domain_in(src: &str, uid: i64, group_ids: &[i64], active: i64, companies: &[i64]) -> Option<Domain> {
    let gids = format!("[{}]", group_ids.iter().map(|g| g.to_string()).collect::<Vec<_>>().join(","));
    let cids = format!("[{}]", companies.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","));
    let cids_false = format!("[{}{}False]", companies.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","), if companies.is_empty() { "" } else { ", " });
    let active = active.to_string();
    let src = src.replace("company_ids + [False]", &cids_false).replace("user.groups_id.ids", &gids);
    let src = src.as_str();
    // substitute only outside string literals (field names like 'company_id' must survive)
    let mut out = String::new(); let mut quote: Option<char> = None; let mut seg = String::new();
    let flush = |seg: &mut String, out: &mut String| {
        let s = seg.replace("user.company_id.id", &active).replace("user.company_ids.ids", &cids).replace("user.id", &uid.to_string())
            .replace("company_ids", &cids).replace("company_id", &active)
            .replace('(', "[").replace(')', "]").replace("True", "true").replace("False", "false").replace("None", "null");
        out.push_str(&s); seg.clear();
    };
    for ch in src.trim().chars() {
        match quote {
            None if ch == '\'' || ch == '"' => { flush(&mut seg, &mut out); quote = Some(ch); out.push('"'); }
            None => seg.push(ch),
            Some(q) if ch == q => { quote = None; out.push('"'); }
            Some(_) => out.push(ch),
        }
    }
    flush(&mut seg, &mut out);
    let j: J = serde_json::from_str(&out.replace(",]", "]")).ok()?;
    Domain::parse(&j).ok()
}

impl Security {
    /// Record-rule domain for `uid`; `co` = (active company, allowed companies) of the session.
    fn rule_domain(&self, uid: i64, model: &str, op: Op, co: Option<(i64, Vec<i64>)>) -> Option<Domain> {
        let (active, cids) = co.clone().unwrap_or((1, vec![1]));
        let applies = |r: &&RuleEntry| r.model == model && match op { Op::Read => r.read, Op::Write => r.write, Op::Create => r.create, Op::Unlink => r.unlink };
        let mine = self.groups_of(uid);
        let gids = self.acl.read().unwrap().user_group_ids.get(&uid).cloned().unwrap_or_default();
        let mut global = Domain::True; let mut group_doms: Vec<Domain> = vec![];
        let all = self.rules.read().unwrap();
        for r in all.iter().filter(applies) {
            // the superuser skips ordinary rules, but still only sees the companies it has switched to
            if uid == 1 && !(co.is_some() && r.groups.is_empty() && r.domain.contains("company_id")) { continue; }
            let d = eval_domain_in(&r.domain, uid, &gids, active, &cids).unwrap_or(Domain::False); // fail closed
            if r.groups.is_empty() { global = global.and(d); }
            else if r.groups.iter().any(|g| mine.contains(g)) { group_doms.push(d); }
        }
        let res = if group_doms.is_empty() { global } else { global.and(Domain::Or(group_doms)) };
        if res == Domain::True { None } else { Some(res) }
    }
}

impl RecordRules for Security {
    fn domain(&self, uid: i64, model: &str, op: Op) -> Option<Domain> { self.rule_domain(uid, model, op, None) }
    fn domain_ctx(&self, uid: i64, model: &str, op: Op, ctx: &BTreeMap<String, Value>) -> Option<Domain> {
        let allowed: Vec<i64> = match ctx.get("allowed_company_ids") { Some(Value::List(l)) => l.iter().filter_map(|v| v.as_i64()).collect(), _ => vec![] };
        let co = if allowed.is_empty() { None } else { Some((ctx.get("company_id").and_then(|v| v.as_i64()).unwrap_or(allowed[0]), allowed)) };
        self.rule_domain(uid, model, op, co)
    }
}

pub fn _unused(_: &orm::Rules) {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn evals_common_rules() {
        assert!(matches!(eval_domain("[('company_id', 'in', company_ids)]", 5), Some(Domain::Term(f, _, _)) if f == "company_id"));
        assert!(matches!(eval_domain("[('user_id','=',user.id)]", 7), Some(Domain::Term(_, _, Value::Int(7)))));
        assert!(matches!(eval_domain("['|',('user_id','=',user.id),('user_id','=',False)]", 7), Some(Domain::Or(_))));
        assert!(eval_domain("[('x','=',some_func())]", 1).is_none());
    }
    #[test] fn password_roundtrip() { let h = hash_password("s", "pw"); assert!(ct_eq(&h, &hash_password("s", "pw"))); assert!(!ct_eq(&h, &hash_password("s", "px"))); }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    #[test] fn groups_and_company_list_rules() {
        assert!(matches!(eval_domain_with("['|', ('create_uid', '=', user.id), ('group_ids', 'in', user.groups_id.ids)]", 9, &[4, 7]), Some(Domain::Or(_))));
        assert!(eval_domain_with("['|', ('company_id', 'in', company_ids + [False]), ('company_id', 'parent_of', company_ids)]", 9, &[]).is_some());
    }
    #[test] fn company_rules_follow_the_session() {
        let d = eval_domain_in("['|', ('company_id', '=', False), ('company_id', 'in', company_ids)]", 5, &[], 2, &[2, 3]).unwrap();
        assert_eq!(format!("{d:?}"), format!("{:?}", eval_domain_in("['|', ('company_id', '=', False), ('company_id', 'in', [2, 3])]", 5, &[], 1, &[1]).unwrap()));
        assert!(matches!(eval_domain_in("[('company_id', '=', user.company_id.id)]", 5, &[], 3, &[3]), Some(Domain::Term(_, _, Value::Int(3)))));
        assert!(eval_domain_in("[('company_id', 'in', company_ids + [False])]", 5, &[], 2, &[2, 3]).is_some());
    }
}
