//! odoo-core: functional, backend-agnostic port of Odoo's ORM and metadata layer.
pub mod ddl;
pub mod dialect;
pub mod domain;
pub mod error;
pub mod orm;
pub mod schema;
pub mod store;
pub mod value;

pub use domain::Domain;
pub use error::{OdooError, Result};
pub use orm::{Env, Rules};
pub use schema::Registry;
pub use value::{Row, Value};
