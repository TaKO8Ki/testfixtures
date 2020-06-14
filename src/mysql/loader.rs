use crate::loader::Loader;
use crate::mysql::helper;
use sqlx::{MySql, MySqlConnection};

pub type MySqlLoader = Loader<MySql, MySqlConnection>;

impl MySqlLoader {
    pub async fn new<F>(options: F) -> anyhow::Result<MySqlLoader>
    where
        F: FnOnce(&mut MySqlLoader),
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
