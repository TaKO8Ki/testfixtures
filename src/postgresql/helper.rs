use crate::fixture_file::{FixtureFile, SqlParam};
use crate::helper::Database as DB;
use async_trait::async_trait;
use chrono::{Offset, TimeZone};
use sqlx::postgres::PgQueryAs;
use sqlx::{
    arguments::Arguments, postgres::PgArguments, PgConnection, PgPool, Postgres as P, Query,
};

/// **PostgreSQL** helper.
pub struct PostgreSql {
    pub tables: Vec<String>,
    // pub use_alter_constraint: bool,
    // pub skip_reset_sequences: bool,
    // pub reset_sequences_to:   u32,
    // pub sequences:                Vec<String>,
    // pub nonDeferrableConstraints: Vec<PgConstraint>,
    // pub tablesChecksum:           map[string]string
}

// pub struct PgConstraint {
//     table_name: String,
//     constraint_name: String,
// }

impl Default for PostgreSql {
    fn default() -> Self {
        PostgreSql { tables: vec![] }
    }
}

#[async_trait]
impl<O, Tz> DB<P, PgConnection, O, Tz> for PostgreSql
where
    Tz: TimeZone<Offset = O> + Send + Sync + 'static,
    O: Offset + Sync + Send + 'static,
{
    async fn init(&mut self, _pool: &PgPool) -> anyhow::Result<()> {
        Ok(())
    }

    async fn database_name(&self, pool: &PgPool) -> anyhow::Result<String> {
        let rec: (String,) = sqlx::query_as("SELECT DATABASE()").fetch_one(pool).await?;
        Ok(rec.0)
    }

    // TODO: complete this function
    // async fn table_names(&self, pool: &PgPool) -> anyhow::Result<Vec<String>> {
    //     let tables = sqlx::query!(
    //         r#"
    //         SELECT table_name
    //         FROM information_schema.tables
    //         WHERE table_schema = ? AND table_type = 'BASE TABLE';
    //     "#,
    //         "test"
    //     )
    //     .fetch_all(pool)
    //     .await?;

    //     let mut names = vec![];
    //     for table in tables {
    //         names.push(table.table_name)
    //     }
    //     Ok(names)
    // }

    async fn with_transaction(
        &self,
        pool: &PgPool,
        fixture_files: &[FixtureFile<Tz>],
    ) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;
        let result: anyhow::Result<()> = async {
            let mut queries = vec![];
            let delete_queries: Vec<String> = fixture_files.iter().map(|x| (x.delete())).collect();
            let mut delete_queries: Vec<Query<'_, P>> =
                delete_queries.iter().map(|x| sqlx::query(x)).collect();
            queries.append(&mut delete_queries);

            for fixtures_file in fixture_files {
                for sql in &fixtures_file.insert_sqls {
                    let mut args = PgArguments::default();
                    for param in &sql.params {
                        match param {
                            SqlParam::String(param) => args.add(param),
                            SqlParam::Integer(param) => args.add(param),
                            SqlParam::Datetime(param) => args.add(param.naive_local()),
                            SqlParam::Float(param) => args.add(param),
                            SqlParam::Boolean(param) => args.add(param),
                        }
                    }
                    queries.push(sqlx::query(sql.sql.as_str()).bind_all(args))
                }
            }
            for query in queries {
                query.execute(&mut tx).await?;
            }
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
