use crate::loader::Loader;
use crate::postgres::helper;
use sqlx::{PgConnection, Postgres};

pub type PostgresLoader = Loader<Postgres, PgConnection>;

impl PostgresLoader {
    pub async fn new<F>(options: F) -> anyhow::Result<PostgresLoader>
    where
        F: FnOnce(&mut PostgresLoader),
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
