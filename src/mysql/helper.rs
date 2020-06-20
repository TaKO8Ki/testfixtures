use crate::fixture_file::{FixtureFile, SqlParam};
use crate::helper::Database as DB;
use async_trait::async_trait;
use chrono::{Offset, TimeZone};
use sqlx::mysql::MySqlQueryAs;
use sqlx::{
    arguments::Arguments, mysql::MySqlArguments, Error, MySql as M, MySqlConnection, MySqlPool,
    Query,
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

    async fn with_transaction(
        &self,
        pool: &MySqlPool,
        fixture_files: &[FixtureFile<Tz>],
    ) -> anyhow::Result<()> {
        let mut tx = pool.begin().await?;

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
                        SqlParam::Float(param) => args.add(param),
                    }
                }
                queries.push(sqlx::query(sql.sql.as_str()).bind_all(args))
            }
        }

        let result: Result<u64, Error> = async {
            sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
                .execute(&mut tx)
                .await?;

            for query in queries {
                let result = query.execute(&mut tx).await;
                if result.is_err() {
                    return result;
                }
            }

            sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
                .execute(&mut tx)
                .await?;
            Ok(1)
        }
        .await;

        match result {
            Ok(_) => tx.commit().await?,
            Err(err) => {
                eprintln!("testfixtures error: {}", err);
                tx.rollback().await?
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::fixture_file::FixtureFile;
    use crate::mysql::helper::MySql;
    use crate::mysql::loader::MySqlLoader;
    use chrono::{prelude::*, NaiveDate, Utc};
    use sqlx::{cursor::Cursor, MySqlPool, Row};
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[async_std::test]
    async fn test_with_transaction() -> anyhow::Result<()> {
        let pool = MySqlPool::new("mysql://root@127.0.0.1:3314/test").await?;
        let dir = tempdir()?;
        let fixture_file_path = dir.path().join("todos.yml");
        let file_path = dir.path().join("todos.yml");
        let mut file = File::create(file_path)?;
        writeln!(
            file,
            r#"
        - id: 1
          description: fizz
          done: 1
          created_at: 2020/01/01 01:01:01"#
        )
        .unwrap();

        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.location(Utc);
        loader.helper = Some(Box::new(MySql { tables: vec![] }));
        let fixture_file = FixtureFile::<Utc> {
            path: fixture_file_path.to_str().unwrap().to_string(),
            file_name: fixture_file_path
                .clone()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            content: File::open(fixture_file_path).unwrap(),
            insert_sqls: vec![],
        };
        loader.fixture_files = vec![fixture_file];
        loader.build_insert_sqls();
        loader
            .helper
            .unwrap()
            .with_transaction(&pool, &loader.fixture_files)
            .await?;

        let mut cursor =
            sqlx::query("SELECT id, description, done, created_at FROM todos").fetch(&pool);
        let row = cursor.next().await?.unwrap();
        let id: i16 = row.get("id");
        let description: String = row.get("description");
        let done: i16 = row.get("done");
        let created_at: NaiveDateTime = row.get("created_at");
        assert_eq!(id, 1);
        assert_eq!(description, "fizz");
        assert_eq!(done, 1);
        assert_eq!(created_at, NaiveDate::from_ymd(2020, 1, 1).and_hms(1, 1, 1));
        Ok(())
    }
}
