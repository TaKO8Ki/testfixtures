use crate::database::DB;
use async_trait::async_trait;
use sqlx::{Connect, Connection, Database, Pool, Transaction};
use std::future::Future;
use std::pin::Pin;
use std::thread;

#[derive(Debug)]
pub struct MySQL {
    pub tables: Vec<String>,
}

impl Default for MySQL {
    fn default() -> Self {
        MySQL { tables: vec![] }
    }
}

#[async_trait]
impl<'a, T, C> DB<T, C> for MySQL
where
    T: Database + Sync + Send,
    C: Connection<Database = T> + Connect + Sync + Send,
    // F: FnOnce(
    //     &mut Transaction<C>,
    // ) -> Pin<Box<(dyn Future<Output = anyhow::Result<()>> + Send + 'a)>>,
{
    async fn init(&mut self, db: &Pool<C>) -> anyhow::Result<()> {
        self.tables = self.table_names(db).await?;
        Ok(())
    }

    async fn database_name(&self, db: &Pool<C>) -> anyhow::Result<String> {
        // let rec: (String,) = sqlx::query_as("SELECT DATABASE()").fetch_one(db).await?;
        Ok("fwoaef".to_string())
    }

    async fn table_names(&self, db: &Pool<C>) -> anyhow::Result<Vec<String>> {
        self.database_name(db);
        //     let tables = sqlx::query!(
        //         r#"
        //     SELECT table_name
        //     FROM information_schema.tables
        //     WHERE table_schema = ? AND table_type = 'BASE TABLE';
        // "#,
        //         "test"
        //     )
        //     .fetch_all(db)
        //     .await?;

        // let mut names = vec![];
        // for table in tables {
        //     names.push(table.table_name)
        // }
        Ok(vec!["string".to_string()])
    }

    // async fn with_transaction(
    //     &self,
    //     db: &C,
    //     f: Box<
    //         dyn FnOnce(
    //                 &'a mut Transaction<C>,
    //             )
    //                 -> Pin<Box<(dyn Future<Output = anyhow::Result<()>> + Sync + Send + 'a)>>
    //             + Send
    //             + Sync
    //             + 'a,
    //     >,
    // ) {
    //     let handler = thread::spawn(|| {});
    //     let mut tx = db.begin().await.unwrap();
    //     f(&mut tx).await;
    //     tx.commit();
    //     // Ok(())
    // }
}
