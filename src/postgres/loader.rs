use crate::loader::Loader;
use crate::postgres::helper;
use chrono::{Offset, TimeZone};
use sqlx::{PgConnection, Postgres};
use std::fmt::Display;

pub type PostgresLoader<O, Tz> = Loader<Postgres, PgConnection, O, Tz>;

impl<O, Tz> PostgresLoader<O, Tz>
where
    O: Offset + Display,
    Tz: TimeZone<Offset = O>,
{
    pub async fn new<F>(options: F) -> anyhow::Result<PostgresLoader<O, Tz>>
    where
        F: FnOnce(&mut PostgresLoader<O, Tz>),
    {
        let mut loader = Self::default();
        options(&mut loader);
        loader.helper = Some(Box::new(helper::Postgres { tables: vec![] }));
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
