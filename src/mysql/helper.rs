use crate::fixture_file::{FixtureFile, SqlParam};
use crate::helper::Database as DB;
use async_trait::async_trait;
use chrono::{Offset, TimeZone};
use sqlx::mysql::MySqlQueryAs;
use sqlx::{
    arguments::Arguments, mysql::MySqlArguments, MySql as M, MySqlConnection, MySqlPool, Query,
};

pub struct MySql {
    pub tables: Vec<String>,
}

impl Default for MySql {
    fn default() -> Self {
        MySql { tables: vec![] }
    }
}

#[async_trait]
impl<O, Tz> DB<M, MySqlConnection, O, Tz> for MySql
where
    O: Offset + Sync + Send + 'static,
    Tz: TimeZone<Offset = O> + Send + Sync + 'static,
{
    async fn init(&mut self, _pool: &MySqlPool) -> anyhow::Result<()> {
        Ok(())
    }

    async fn database_name(&self, pool: &MySqlPool) -> anyhow::Result<String> {
        let rec: (String,) = sqlx::query_as("SELECT DATABASE()").fetch_one(pool).await?;
        Ok(rec.0)
    }

    // TODO: complete this function
    // async fn table_names(&self, pool: &MySqlPool) -> anyhow::Result<Vec<String>> {
    //     let tables = sqlx::query_as(
    //         r#"
    //         SELECT table_name
    //         FROM information_schema.tables
    //         WHERE table_schema = ? AND table_type = 'BASE TABLE';
    //     "#,
    //     )
    //     .bind("test")
    //     .fetch_all(pool)
    //     .await?;

    //     let mut names = vec![];
    //     for table in tables {
    //         names.push(table.table_name)
    //     }
    //     Ok(names)
    // }

    async fn with_transaction<'b>(
        &self,
        pool: &MySqlPool,
        fixture_files: &[FixtureFile<Tz>],
    ) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;
        let result: anyhow::Result<()> = async {
            sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
                .execute(&mut tx)
                .await?;

            let mut queries = vec![];
            let delete_queries: Vec<String> = fixture_files.iter().map(|x| (x.delete())).collect();
            let mut delete_queries: Vec<Query<'_, M>> =
                delete_queries.iter().map(|x| sqlx::query(x)).collect();
            queries.append(&mut delete_queries);

            for fixtures_file in fixture_files {
                for sql in &fixtures_file.insert_sqls {
                    let mut args = MySqlArguments::default();
                    for param in &sql.params {
                        match param {
                            SqlParam::String(param) => args.add(param),
                            SqlParam::Integer(param) => args.add(param),
                            SqlParam::Datetime(param) => args.add(param.naive_local()),
                        }
                    }
                    queries.push(sqlx::query(sql.sql.as_str()).bind_all(args))
                }
            }

            for query in queries {
                query.execute(&mut tx).await?;
            }
            sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
                .execute(&mut tx)
                .await?;
            Ok(())
        }
        .await;
        if result.is_ok() {
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }
        Ok(())
    }
}
