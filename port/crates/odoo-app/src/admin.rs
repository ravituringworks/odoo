//! Operator console (`admin_*` methods): every database on the server (the main one, trials, administrator-created ones) with its
//! lifecycle (backup, restore, archive, delete) and per-account app restrictions. Only the administrator (uid 1 / `base.group_system`)
//! of the main database may call these; tenants refuse them. Backups are SQLite snapshots in `<trial dir>/backups`.
use super::*;
use std::path::PathBuf;

fn text(r: &Row, k: &str) -> String { r.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string() }
fn ph(c: &dyn odoo_core::store::Conn, n: usize) -> String { c.dialect().placeholder(n) }
fn user<T>(m: &str) -> Result<T> { Err(OdooError::User(m.into())) }
fn io(e: std::io::Error) -> OdooError { OdooError::Storage(e.to_string()) }
fn names(v: &J) -> Vec<String> { v.as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect() }

/// `<db>__<yyyymmdd>-<hhmmss>-<4 hex>.sqlite`: the only file names a backup can have, so a name from the client can never point elsewhere.
fn is_backup_of(db: &str, file: &str) -> bool {
    let Some(rest) = file.strip_prefix(&format!("{db}__")).and_then(|r| r.strip_suffix(".sqlite")) else { return false };
    let p: Vec<&str> = rest.split('-').collect();
    p.len() == 3 && p[0].len() == 8 && p[1].len() == 6 && p[2].len() == 4 && p[..2].iter().all(|x| x.chars().all(|c| c.is_ascii_digit())) && p[2].chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

impl App {
    /// Registry status of a database: None = not registered.
    pub(crate) fn db_status(&self, db: &str) -> Result<Option<String>> {
        store::run(self.store.as_ref(), |c| Ok(c.query(&format!("SELECT status FROM odoo_trials WHERE db = {}", ph(c, 1)), &[db.into()])?.first().map(|r| { let s = text(r, "status"); if s.is_empty() { "active".to_string() } else { s } })))
    }
    /// Catalog app names an account may not use.
    pub(crate) fn account_disallowed(&self, email: &str) -> Result<Vec<String>> {
        if email.is_empty() { return Ok(vec![]); }
        store::run(self.store.as_ref(), |c| Ok(c.query(&format!("SELECT disallowed FROM odoo_portal_users WHERE email = {}", ph(c, 1)), &[email.into()])?.first().map(|r| names(&serde_json::from_str::<J>(&text(r, "disallowed")).unwrap_or(J::Null))).unwrap_or_default()))
    }
    /// Modules a database may not install: those of its owner's disallowed apps.
    pub(crate) fn denied_for_db(&self, db: &str) -> Vec<String> {
        let owner = store::run(self.store.as_ref(), |c| Ok(c.query(&format!("SELECT email FROM odoo_trials WHERE db = {}", ph(c, 1)), &[db.into()])?.first().map(|r| text(r, "email")).unwrap_or_default())).unwrap_or_default();
        trial::denied_modules(&self.account_disallowed(&owner).unwrap_or_default())
    }

    fn backup_dir(&self) -> Result<PathBuf> {
        let cfg = self.trial_cfg.as_ref().ok_or_else(|| OdooError::User("Backups need the trials directory to be configured on this server.".into()))?;
        let d = cfg.dir.join("backups"); std::fs::create_dir_all(&d).map_err(io)?; Ok(d)
    }
    fn backups_of(&self, db: &str) -> Result<Vec<J>> {
        let dir = self.backup_dir()?; let mut out = vec![];
        for e in std::fs::read_dir(&dir).map_err(io)?.flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            if is_backup_of(db, &f) { let rest = &f[db.len() + 2..]; out.push(json!({"file": f, "size": e.metadata().map(|m| m.len()).unwrap_or(0), "created_at": format!("{}-{}-{} {}:{}:{}", &rest[0..4], &rest[4..6], &rest[6..8], &rest[9..11], &rest[11..13], &rest[13..15])})); }
        }
        out.sort_by(|a, b| b["file"].as_str().cmp(&a["file"].as_str())); Ok(out)
    }
    /// Run `f` on the database's application: the main one, an open tenant, or (archived / not yet opened) a short-lived one.
    fn with_db<R>(&self, db: &str, f: impl FnOnce(&App) -> Result<R>) -> Result<R> {
        if db == "main" { return f(self); }
        if !trial::valid_db(db) || self.db_status(db)?.is_none() { return user("Unknown database."); }
        if let Some(t) = self.tenants.read().unwrap().get(db).cloned() { return f(&t); }
        let path = self.trial_path(db)?;
        if !path.exists() { return user("The database file is missing."); }
        f(&self.open_tenant(&path, vec!["contacts".into()])?)
    }
    fn make_backup(&self, db: &str) -> Result<J> {
        let dir = self.backup_dir()?;
        let stamp = orm::now().replace(['-', ':'], "").replace(' ', "-");
        let file = format!("{db}__{stamp}-{}.sqlite", &security::random_hex(2));
        self.with_db(db, |a| a.store.snapshot(&dir.join(&file)))?;
        let size = std::fs::metadata(dir.join(&file)).map(|m| m.len()).unwrap_or(0);
        Ok(json!({"db": db, "file": file, "size": size, "created_at": orm::now()}))
    }

    fn admin_db_list(&self) -> Result<J> {
        let rows = store::run(self.store.as_ref(), |c| c.query("SELECT db, email, company, apps, created_at, kind, status FROM odoo_trials ORDER BY created_at DESC", &[]))?;
        let days = self.trial_cfg.as_ref().map_or(15, |c| c.days as i64); let today = orm::epoch_days(&orm::today());
        let size = |db: &str| self.trial_path(db).ok().map(|p| ["", "-wal"].iter().map(|x| std::fs::metadata(format!("{}{x}", p.display())).map(|m| m.len()).unwrap_or(0)).sum::<u64>());
        let backups = |db: &str| self.backups_of(db).unwrap_or_default();
        let entry = |db: String, kind: String, status: String, email: String, company: String, apps: J, created: String, size: Option<u64>| {
            let b = backups(&db);
            let left = if kind == "trial" && !created.is_empty() { json!((days - (today - orm::epoch_days(&created))).max(0)) } else { J::Null };
            json!({"db": db, "kind": kind, "status": status, "owner": email, "company": company, "apps": apps, "created_at": created, "days_left": left, "size": size, "backups": b.len(), "last_backup": b.first().and_then(|x| x["created_at"].as_str()).map(String::from), "open_tenant": self.tenants.read().unwrap().contains_key(&db)})
        };
        let mut out = vec![entry("main".into(), "main".into(), "active".into(), String::new(), String::new(), json!([]), String::new(), None)];
        for r in &rows {
            let (kind, status) = (text(r, "kind"), text(r, "status")); let db = text(r, "db");
            let sz = size(&db);
            out.push(entry(db, if kind.is_empty() { "trial".into() } else { kind }, if status.is_empty() { "active".into() } else { status }, text(r, "email"), text(r, "company"), serde_json::from_str(&text(r, "apps")).unwrap_or(json!([])), text(r, "created_at"), sz));
        }
        Ok(J::Array(out))
    }

    /// Replace a database's contents with one of its backups. The current state is backed up first and put back if the restored file does not open.
    fn admin_db_restore(&self, db: &str, file: &str) -> Result<J> {
        if db == "main" { return user("The main database cannot be restored while the server is running: stop the server and replace its file."); }
        if !is_backup_of(db, file) { return user("That is not a backup of this database."); }
        let src = self.backup_dir()?.join(file);
        let mut head = [0u8; 16];
        { use std::io::Read; std::fs::File::open(&src).map_err(|_| OdooError::User("The backup file is missing.".into()))?.read_exact(&mut head).map_err(io)?; }
        if &head != b"SQLite format 3\0" { return user("The backup file is not a valid database."); }
        let safety = self.make_backup(db)?;   // pre-restore safety copy
        let path = self.trial_path(db)?;
        self.tenants.write().unwrap().remove(db);
        let tmp = path.with_extension("restoring");
        std::fs::copy(&src, &tmp).map_err(io)?;
        let old = path.with_extension("replaced");
        let _ = std::fs::rename(&path, &old);
        for ext in ["sqlite-wal", "sqlite-shm"] { let _ = std::fs::remove_file(path.with_extension(ext)); }
        std::fs::rename(&tmp, &path).map_err(io)?;
        match self.tenant(db).and_then(|t| t.runtime().reg.model("res.users").map(|_| ())) {
            Ok(()) => { let _ = std::fs::remove_file(&old); Ok(json!({"db": db, "restored": file, "safety_backup": safety["file"]})) }
            Err(e) => {   // roll back to what was there
                self.tenants.write().unwrap().remove(db);
                let _ = std::fs::remove_file(&path); let _ = std::fs::rename(&old, &path);
                Err(OdooError::User(format!("The backup could not be opened ({e}); the database was left unchanged.")))
            }
        }
    }

    fn admin_account_list(&self) -> Result<J> {
        let (users, dbs) = store::run(self.store.as_ref(), |c| Ok((c.query("SELECT email, name, company, created_at, disallowed FROM odoo_portal_users ORDER BY created_at DESC", &[])?, c.query("SELECT email, db FROM odoo_trials", &[])?)))?;
        Ok(J::Array(users.iter().map(|u| { let email = text(u, "email");
            json!({"email": email, "name": text(u, "name"), "company": text(u, "company"), "created_at": text(u, "created_at"), "databases": dbs.iter().filter(|d| text(d, "email") == email).count(), "disallowed": names(&serde_json::from_str::<J>(&text(u, "disallowed")).unwrap_or(J::Null))}) }).collect()))
    }

    fn admin_account_set_apps(&self, email: &str, disallowed: &J) -> Result<J> {
        let mut list = names(disallowed); list.sort(); list.dedup();
        if let Some(bad) = list.iter().find(|a| !trial::CATALOG.iter().any(|c| c.name == a.as_str())) { return user(&format!("`{bad}` is not an app.")); }
        let n = store::run(self.store.as_ref(), |c| c.execute(&format!("UPDATE odoo_portal_users SET disallowed = {} WHERE email = {}", ph(c, 1), ph(c, 2)), &[json!(list).to_string().into(), email.into()]))?;
        if n == 0 { return user("Unknown account."); }
        let denied = trial::denied_modules(&list);
        let owned: Vec<String> = store::run(self.store.as_ref(), |c| Ok(c.query(&format!("SELECT db FROM odoo_trials WHERE email = {}", ph(c, 1)), &[email.into()])?.iter().map(|r| text(r, "db")).collect()))?;
        for db in owned { if let Some(t) = self.tenants.read().unwrap().get(&db) { *t.denied_modules.write().unwrap() = denied.clone(); } }
        Ok(json!({"email": email, "disallowed": list}))
    }

    /// Dispatch for `admin_*` methods.
    pub(crate) fn admin_dispatch(&self, req: &Request) -> Result<J> {
        let denied = || OdooError::AccessDenied { op: "administer".into(), model: "database".into(), uid: req.uid };
        if self.is_trial || !self.sec.is_admin(req.uid) { return Err(denied()); }
        let arg = |i: usize| req.args.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let a0 = req.args.first().cloned().unwrap_or(J::Null);
        let registered = |db: &str| -> Result<()> { if db == "main" { return user("The main database cannot be archived, deleted or restored while the server is running."); } if trial::valid_db(db) && self.db_status(db)?.is_some() { Ok(()) } else { user("Unknown database.") } };
        match req.method.as_str() {
            "admin_db_list" => self.admin_db_list(),
            "admin_db_backup" => { let db = arg(0); if db != "main" { registered(&db)?; } self.make_backup(&db) }
            "admin_db_backups" => { let db = arg(0); if db != "main" { registered(&db)?; } Ok(J::Array(self.backups_of(&db)?)) }
            "admin_db_backup_delete" => {
                let (db, file) = (arg(0), arg(1));
                if !is_backup_of(&db, &file) { return user("That is not a backup of this database."); }
                std::fs::remove_file(self.backup_dir()?.join(&file)).map_err(|_| OdooError::User("The backup file is missing.".into()))?; Ok(J::Bool(true))
            }
            "admin_db_restore" => { let db = arg(0); registered(&db)?; if arg(2) != db { return user("Type the database name to confirm the restore."); } self.admin_db_restore(&db, &arg(1)) }
            "admin_db_archive" => {
                let db = arg(0); registered(&db)?;
                let on = req.args.get(1).and_then(|v| v.as_bool()).unwrap_or(true);
                store::run(self.store.as_ref(), |c| c.execute(&format!("UPDATE odoo_trials SET status = {} WHERE db = {}", ph(c, 1), ph(c, 2)), &[(if on { "archived" } else { "active" }).into(), db.clone().into()]))?;
                if on { self.tenants.write().unwrap().remove(&db); }   // closes the database; requests are refused until it is unarchived
                Ok(json!({"db": db, "status": if on { "archived" } else { "active" }}))
            }
            "admin_db_delete" => {
                let db = arg(0); registered(&db)?;
                if arg(1) != db { return user("Type the database name to confirm the deletion."); }
                self.delete_trial(&db);
                if !req.args.get(2).and_then(|v| v.as_bool()).unwrap_or(false) { for b in self.backups_of(&db)? { if let Some(f) = b["file"].as_str() { let _ = std::fs::remove_file(self.backup_dir()?.join(f)); } } }
                Ok(J::Bool(true))
            }
            "admin_db_open" => {
                let db = arg(0); registered(&db)?;
                Ok(json!({"db": db, "token": self.tenant(&db)?.sec.open_session(1)}))
            }
            "admin_db_create" => {
                let name = a0["name"].as_str().unwrap_or("").trim().to_string();
                let name = if name.is_empty() { format!("db_{}", security::random_hex(4)) } else { name };
                if !trial::valid_standard_name(&name) { return user("Database names use 3-30 lowercase letters, digits or underscores, start with a letter, and cannot start with `trial`."); }
                let owner = a0["email"].as_str().unwrap_or("").trim().to_lowercase();
                if !owner.is_empty() && self.account_row_exists(&owner)? == false { return user("That email has no account. Leave it empty for an unowned database."); }
                let form = json!({"name": "Administrator", "email": if owner.is_empty() { format!("admin@{name}.local") } else { owner.clone() }, "phone": "", "company": a0["company"], "country": "", "password": security::random_hex(12), "apps": a0["apps"]});
                let sg = trial::parse_signup(&form, &|m| self.module_available(m))?;
                let (db, _) = self.provision_db(&sg, "standard", Some(&name))?;
                Ok(json!({"db": db, "kind": "standard", "owner": owner}))
            }
            "admin_account_list" => self.admin_account_list(),
            "admin_account_set_apps" => self.admin_account_set_apps(&arg(0), req.args.get(1).unwrap_or(&J::Null)),
            other => user(&format!("unknown admin method `{other}`")),
        }
    }

    fn account_row_exists(&self, email: &str) -> Result<bool> {
        store::run(self.store.as_ref(), |c| Ok(!c.query(&format!("SELECT 1 FROM odoo_portal_users WHERE email = {}", ph(c, 1)), &[email.into()])?.is_empty()))
    }
}
