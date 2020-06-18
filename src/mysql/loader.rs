use crate::loader::Loader;
use crate::mysql::helper;
use chrono::{Offset, TimeZone};
use sqlx::{MySql, MySqlConnection};
use std::fmt::Display;

pub type MySqlLoader<O, Tz> = Loader<MySql, MySqlConnection, O, Tz>;

impl<O, Tz> MySqlLoader<O, Tz>
where
    O: Offset + Display + Send + Sync + 'static,
    Tz: TimeZone<Offset = O> + Send + Sync + 'static,
{
    pub async fn new<F>(options: F) -> anyhow::Result<MySqlLoader<O, Tz>>
    where
        F: FnOnce(&mut MySqlLoader<O, Tz>),
    {
        let mut loader = Self::default();
        options(&mut loader);
        loader.helper = Some(Box::new(helper::MySql { tables: vec![] }));
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
