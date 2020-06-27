use crate::loader::Loader;
use crate::postgresql::helper;
use chrono::{Offset, TimeZone};
use sqlx::{PgConnection, Postgres};
use std::fmt::Display;

// TODO: Complete this type.
/// An alias for [Loader](crate::loader::Loader), specialized for **PostgreSQL**.
pub(crate) type PglLoader<O, Tz> = Loader<Postgres, PgConnection, O, Tz>;

impl<O, Tz> PostgresLoader<O, Tz>
where
    O: Offset + Display + Send + Sync + 'static,
    Tz: TimeZone<Offset = O> + Sync + Send + 'static,
{
    pub async fn new<F>(options: F) -> anyhow::Result<PostgresLoader<O, Tz>>
    where
        F: FnOnce(&mut PostgresLoader<O, Tz>),
    {
        let mut loader = Self::default();
        options(&mut loader);
        loader.helper = Some(Box::new(helper::PostgreSql::default()));
        loader.build_insert_sqls();
        loader
            .helper
            .as_mut()
            .unwrap()
            .init(loader.pool.as_ref().unwrap())
            .await?;
        Ok(loader)
    }
}
