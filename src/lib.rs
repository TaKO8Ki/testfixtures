//! [![github]](https://github.com/TaKO8Ki/testfixtures)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?labelColor=555555&logo=github
//!
//! This crate is a Rust library for preparing test data from yaml files.
//!
//! ## Examples
//! `todos.yml`
//! ```yml
//! - id: 1
//!   description: buy a new camera
//!   done: 0
//! - id: 2
//!   description: meeting
//!   done: 0
//! ```
//!
//!  ```rust
//! #[cfg(test)]
//! mod tests {
//!     use chrono::Utc;
//!     use sqlx::MySqlPool;
//!     use std::env;
//!     use testfixtures::MySqlLoader;
//!     #[async_std::test]
//!     async fn test_something() -> anyhow::Result<()> {
//!         let pool = MySqlPool::new(&env::var("DATABASE_URL")?).await?;
//!         let loader = MySqlLoader::new(|cfg| {
//!             cfg.location(Utc);
//!             cfg.database(pool);
//!             cfg.skip_test_database_check();
//!             cfg.paths(vec!["fixtures/todos.yml"]);
//!         })
//!         .await?;
//!
//!         // load your fixtures
//!         loader.load().await?;
//!         // run your tests
//!         Ok(())
//!     }
//! }
//! ```

mod fixture_file;
mod helper;
mod loader;
mod mysql;
// mod postgresql;

// reexport key APIs
pub use fixture_file::{FixtureFile, InsertSql, SqlParam};
pub use helper::Database;
pub use loader::Loader;
pub use mysql::helper::MySql;
pub use mysql::loader::MySqlLoader;
// pub use postgresql::helper::PostgreSql;
// pub use postgres::loader::PgLoader;
