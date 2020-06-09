use crate::loader::Loader;
use crate::mysql::helper;
use sqlx::{MySql, MySqlConnection};

pub type MySqlLoader = Loader<MySql, MySqlConnection>;

impl MySqlLoader {
    pub async fn new(
        options: Vec<Box<dyn FnOnce(&mut MySqlLoader)>>,
    ) -> anyhow::Result<MySqlLoader> {
        let mut loader = Self::default();
        for o in options {
            o(&mut loader);
        }
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
