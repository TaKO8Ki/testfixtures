use super::database::Database;
use async_trait::async_trait;
use sqlx::MySqlPool;

#[derive(Debug)]
pub struct MySQL {
    pub tables: Vec<String>,
}

impl<'a> Default for MySQL {
    fn default() -> Self {
        MySQL { tables: vec![] }
    }
}

#[async_trait]
impl Database for MySQL {
    async fn init(&mut self, db: &MySqlPool) -> anyhow::Result<()> {
        self.table_names(db).await?;
        self.tables = self.table_names(db).await?;
        Ok(())
    }

    // async fn database_name(self, db: MySqlPool) -> anyhow::Result<()> {
    //     let db_name = "";
    //     let recs = sqlx::query!("SELECT * from todos;").fetch_all(db).await?;
    //     Ok(recs)
    // }

    async fn table_names(&self, db: &MySqlPool) -> anyhow::Result<Vec<String>> {
        let tables = sqlx::query!(
            r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = ? AND table_type = 'BASE TABLE';
    "#,
            "test"
        )
        .fetch_all(db)
        .await?;

        let mut names = vec![];
        for table in tables {
            names.push(table.table_name)
        }
        Ok(names)
    }
}
