use crate::loader::Loader;
use crate::postgres::helper;
use sqlx::{PgConnection, Postgres};

impl PostgresLoader {
    pub async fn new(
        options: Vec<Box<dyn FnOnce(&mut PostgresLoader)>>,
    ) -> anyhow::Result<PostgresLoader> {
        let mut loader = Self::default();
        for o in options {
            o(&mut loader);
        }
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

pub type PostgresLoader = Loader<Postgres, PgConnection>;
