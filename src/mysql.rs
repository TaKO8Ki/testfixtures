use crate::database::DB;
use async_trait::async_trait;
use sqlx::{Connect, Connection, Database, Pool, Query};

#[derive(Debug)]
pub struct MySql {
    pub tables: Vec<String>,
}

impl Default for MySql {
    fn default() -> Self {
        MySql { tables: vec![] }
    }
}

#[async_trait]
impl<T, C> DB<T, C> for MySql
where
    T: Database + Sync + Send,
    C: Connection<Database = T> + Connect<Database = T> + Sync + Send,
{
    async fn init(&mut self, db: &Pool<C>) -> anyhow::Result<()> {
        self.tables = self.table_names(db).await?;
        Ok(())
    }

    async fn database_name(&self, db: &Pool<C>) -> anyhow::Result<String> {
        // let rec: (String,) = sqlx::query!("SELECT DATABASE()").fetch_one(db).await?;
        // Ok(rec.0)
        Ok("fizz".to_string())
    }

    async fn table_names(&self, pool: &Pool<C>) -> anyhow::Result<Vec<String>> {
        // let tables = sqlx::query!(
        //     r#"
        //     SELECT table_name
        //     FROM information_schema.tables
        //     WHERE table_schema = ? AND table_type = 'BASE TABLE';
        // "#,
        //     "test"
        // )
        // .fetch_all(pool)
        // .await?;

        // let mut names = vec![];
        // for table in tables {
        //     names.push(table.table_name)
        // }
        Ok(vec!["fizz".to_string()])
    }

    async fn with_transaction<'b>(
        &self,
        pool: &Pool<C>,
        queries: Vec<Query<'b, T>>,
    ) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;
        for query in queries {
            query.execute(&mut tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
