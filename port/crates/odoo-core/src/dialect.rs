//! SQL dialect abstraction: everything backend-specific lives here, so the ORM
//! and DDL/query builders stay pure and portable.
use crate::schema::FieldDef;

pub trait Dialect: Send + Sync {
    fn name(&self) -> &'static str;
    fn quote(&self, ident: &str) -> String { format!("\"{}\"", ident.replace('"', "\"\"")) }
    /// 1-based bind placeholder.
    fn placeholder(&self, _n: usize) -> String { "?".into() }
    fn column_type(&self, f: &FieldDef) -> String;
    fn pk_column(&self) -> &'static str;
    fn now(&self) -> &'static str { "CURRENT_TIMESTAMP" }
    fn ilike(&self, col: &str, ph: &str) -> String { format!("{col} ILIKE {ph}") }
    fn supports_returning(&self) -> bool { true }
    /// FK constraints must be added after all tables exist (cyclic schema).
    fn alter_fk(&self) -> bool { true }
    fn bool_lit(&self, v: bool) -> &'static str { if v { "TRUE" } else { "FALSE" } }
    fn concat(&self, parts: &[String]) -> String { parts.join(" || ") }
    /// Statement(s) required before `CREATE TABLE` (e.g. sequences).
    fn pre_table(&self, _table: &str) -> Option<String> { None }
    fn pk_for(&self, _table: &str) -> String { self.pk_column().to_string() }
    /// `SELECT table_name, column_name` of every existing table in the current schema (for self-healing schema sync).
    fn columns_query(&self) -> &'static str { "SELECT table_name, column_name FROM information_schema.columns WHERE table_schema = current_schema()" }
}

fn base_type(f: &FieldDef, text: &str, int: &str, real: &str, blob: &str, boolean: &str, json: &str) -> String {
    match f.ty.as_str() {
        "Integer" | "Many2one" | "Many2oneReference" => int.into(),
        "Float" | "Monetary" => real.into(),
        "Boolean" => boolean.into(),
        "Binary" | "Image" => blob.into(),
        "Json" | "Properties" => json.into(),
        "Date" => if text == "TEXT" { "TEXT".into() } else { "DATE".into() },
        "Datetime" => if text == "TEXT" { "TEXT".into() } else { "TIMESTAMP".into() },
        _ => text.into(),
    }
}

pub struct Sqlite;
impl Dialect for Sqlite {
    fn name(&self) -> &'static str { "sqlite" }
    fn columns_query(&self) -> &'static str { "SELECT m.name AS table_name, p.name AS column_name FROM sqlite_master m JOIN pragma_table_info(m.name) p WHERE m.type = 'table'" }
    fn column_type(&self, f: &FieldDef) -> String { base_type(f, "TEXT", "INTEGER", "REAL", "BLOB", "INTEGER", "TEXT") }
    fn pk_column(&self) -> &'static str { "INTEGER PRIMARY KEY AUTOINCREMENT" }
    fn now(&self) -> &'static str { "datetime('now')" }
    fn ilike(&self, col: &str, ph: &str) -> String { format!("{col} LIKE {ph} ESCAPE '\\'") } // sqlite LIKE is ASCII case-insensitive
    fn alter_fk(&self) -> bool { false }
    fn bool_lit(&self, v: bool) -> &'static str { if v { "1" } else { "0" } }
}

pub struct Postgres;
impl Dialect for Postgres {
    fn name(&self) -> &'static str { "postgres" }
    fn placeholder(&self, n: usize) -> String { format!("${n}") }
    fn column_type(&self, f: &FieldDef) -> String {
        match f.ty.as_str() {
            "Float" | "Monetary" => "NUMERIC".into(),
            "Char" | "Selection" | "Reference" => "VARCHAR".into(),
            "Binary" | "Image" => "BYTEA".into(),
            "Json" | "Properties" => "JSONB".into(),
            "Date" => "DATE".into(),
            "Datetime" => "TIMESTAMP".into(),
            _ => base_type(f, "TEXT", "INTEGER", "NUMERIC", "BYTEA", "BOOLEAN", "JSONB"),
        }
    }
    fn pk_column(&self) -> &'static str { "SERIAL PRIMARY KEY" }
    fn now(&self) -> &'static str { "NOW() AT TIME ZONE 'UTC'" }
}

/// orbit-rs speaks the PostgreSQL wire protocol/dialect; reuse with a distinct name for capability gating.
pub struct OrbitRs;
impl Dialect for OrbitRs {
    fn name(&self) -> &'static str { "orbit-rs" }
    fn placeholder(&self, n: usize) -> String { Postgres.placeholder(n) }
    fn column_type(&self, f: &FieldDef) -> String { Postgres.column_type(f) }
    fn pk_column(&self) -> &'static str { Postgres.pk_column() }
    fn now(&self) -> &'static str { Postgres.now() }
}

pub struct MySql;
impl Dialect for MySql {
    fn name(&self) -> &'static str { "mysql" }
    fn columns_query(&self) -> &'static str { "SELECT table_name AS table_name, column_name AS column_name FROM information_schema.columns WHERE table_schema = DATABASE()" }
    fn quote(&self, i: &str) -> String { format!("`{}`", i.replace('`', "``")) }
    fn column_type(&self, f: &FieldDef) -> String {
        match f.ty.as_str() {
            "Char" | "Selection" | "Reference" => format!("VARCHAR({})", f.size.as_ref().and_then(|s| s.as_u64()).unwrap_or(255)),
            "Float" | "Monetary" => "DECIMAL(20,6)".into(),
            "Binary" | "Image" => "LONGBLOB".into(),
            "Text" | "Html" => "LONGTEXT".into(),
            "Date" => "DATE".into(),
            "Datetime" => "DATETIME".into(),
            _ => base_type(f, "TEXT", "BIGINT", "DOUBLE", "LONGBLOB", "TINYINT(1)", "JSON"),
        }
    }
    fn pk_column(&self) -> &'static str { "BIGINT AUTO_INCREMENT PRIMARY KEY" }
    fn ilike(&self, col: &str, ph: &str) -> String { format!("{col} LIKE {ph}") } // default collation is case-insensitive
    fn supports_returning(&self) -> bool { false }
    fn concat(&self, p: &[String]) -> String { format!("CONCAT({})", p.join(", ")) }
}

pub struct DuckDb;
impl Dialect for DuckDb {
    fn name(&self) -> &'static str { "duckdb" }
    fn columns_query(&self) -> &'static str { "SELECT table_name, column_name FROM information_schema.columns WHERE table_schema = 'main'" }
    fn placeholder(&self, n: usize) -> String { format!("${n}") }
    fn column_type(&self, f: &FieldDef) -> String { base_type(f, "VARCHAR", "BIGINT", "DOUBLE", "BLOB", "BOOLEAN", "JSON") }
    fn pk_column(&self) -> &'static str { "BIGINT PRIMARY KEY DEFAULT nextval('{seq}')" }
    fn alter_fk(&self) -> bool { false }
    fn pre_table(&self, t: &str) -> Option<String> { Some(format!("CREATE SEQUENCE IF NOT EXISTS {}_id_seq", t)) }
    fn pk_for(&self, t: &str) -> String { format!("BIGINT PRIMARY KEY DEFAULT nextval('{}_id_seq')", t) }
}

pub fn by_name(n: &str) -> Option<Box<dyn Dialect>> {
    Some(match n { "sqlite" => Box::new(Sqlite), "postgres" | "postgresql" => Box::new(Postgres), "mysql" | "mariadb" => Box::new(MySql), "duckdb" => Box::new(DuckDb), "orbit-rs" | "orbit" => Box::new(OrbitRs), _ => return None })
}
