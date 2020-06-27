use crate::fixture_file::{FixtureFile, SqlParam};
use crate::helper::Database;
use async_trait::async_trait;
use chrono::{Offset, TimeZone};
use sqlx::mysql::MySqlQueryAs;
use sqlx::{
    arguments::Arguments, mysql::MySqlArguments, mysql::MySqlRow, Error, MySql as M,
    MySqlConnection, MySqlPool, Query, Row,
};
use std::collections::HashMap;

/// **MySQL** helper.
pub struct MySql {
    pub table_names: Vec<String>,
    pub db_name: Option<String>,
    pub tables_checksum: HashMap<String, i32>,
}

impl Default for MySql {
    fn default() -> Self {
        MySql {
            table_names: vec![],
            db_name: None,
            tables_checksum: HashMap::new(),
        }
    }
}

#[async_trait]
impl<O, Tz> Database<M, MySqlConnection, O, Tz> for MySql
where
    O: Offset + Sync + Send + 'static,
    Tz: TimeZone<Offset = O> + Send + Sync + 'static,
{
    /// Initialize MySQL struct.
    async fn init(&mut self, pool: &MySqlPool) -> anyhow::Result<()> {
        let rec: (String,) = sqlx::query_as("SELECT DATABASE()").fetch_one(pool).await?;
        self.db_name = Some(rec.0);

        let table_names = self.table_names(pool).await?;
        for t in table_names {
            self.table_names.push(t)
        }
        Ok(())
    }

    /// Get a database name.
    async fn database_name(&self) -> anyhow::Result<String> {
        Ok(self.db_name.as_ref().unwrap().to_string())
    }

    // /// Get table names.
    // async fn table_names(&self, pool: &MySqlPool) -> anyhow::Result<Vec<String>> {
    //     let table_names: Vec<String> = sqlx::query(
    //         r#"
    //         SELECT table_name
    //         FROM information_schema.tables
    //         WHERE table_schema = ? AND table_type = 'BASE TABLE';
    //     "#,
    //     )
    //     .bind(self.db_name.as_ref().unwrap())
    //     .try_map(|row: MySqlRow| row.try_get(0))
    //     .fetch_all(pool)
    //     .await?;

    //     let mut names = vec![];
    //     for t in table_names {
    //         names.push(t)
    //     }
    //     Ok(names)
    // }

    /// Execute SQL queries in a transaction for MySQL.
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

        for fixture_file in fixture_files {
            if !self
                .is_table_modified(pool, fixture_file.file_stem())
                .await?
            {
                continue;
            }
            for sql in &fixture_file.insert_sqls {
                let mut args = MySqlArguments::default();
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
            Ok(_) => {
                tx.commit().await?;
            }
            Err(err) => {
                tx.rollback().await?;
                return Err(anyhow::anyhow!("testfixtures error: {}", err));
            }
        };
        Ok(())
    }

    async fn after_load(&mut self, pool: &MySqlPool) -> anyhow::Result<()> {
        if self.tables_checksum.len() != 0 {
            return Ok(());
        }
        for t in self.table_names.clone() {
            let checksum = self.get_checksum(pool, t.clone()).await?;
            self.tables_checksum.insert(t.clone(), checksum);
        }
        Ok(())
    }
}

impl MySql {
    async fn table_names(&self, pool: &MySqlPool) -> anyhow::Result<Vec<String>> {
        let table_names: Vec<String> = sqlx::query(
            r#"
            SELECT table_name
            FROM information_schema.tables
            WHERE table_schema = ? AND table_type = 'BASE TABLE';
        "#,
        )
        .bind(self.db_name.as_ref().unwrap())
        .try_map(|row: MySqlRow| row.try_get(0))
        .fetch_all(pool)
        .await?;

        let mut names = vec![];
        for t in table_names {
            names.push(t)
        }
        Ok(names)
    }

    async fn is_table_modified(
        &self,
        pool: &MySqlPool,
        table_name: String,
    ) -> anyhow::Result<bool> {
        let checksum = self.get_checksum(pool, table_name.clone()).await?;
        let old_checksum = self.tables_checksum.get(&table_name.clone());

        Ok(*old_checksum.unwrap() == 0 || checksum != *old_checksum.unwrap())
    }

    async fn get_checksum(&self, pool: &MySqlPool, table_name: String) -> anyhow::Result<i32> {
        let rec: (String, i32) = sqlx::query_as(format!("CHECKSUM TABLE {}", table_name).as_str())
            .fetch_one(pool)
            .await?;
        Ok(rec.1)
    }
}

#[cfg(test)]
#[cfg(feature = "mysql")]
mod tests {
    use crate::fixture_file::FixtureFile;
    use crate::mysql::helper::MySql;
    use crate::mysql::loader::MySqlLoader;
    use chrono::{prelude::*, NaiveDate, Utc};
    use sqlx::{cursor::Cursor, MySqlPool, Row};
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn test_table_names() -> anyhow::Result<()> {
        let pool = MySqlPool::new(&env::var("TEST_DB_URL")?).await?;
        let helper = MySql::default();
        let table_names = helper.table_names(&pool).await?;
        assert_eq!(table_names, vec!["todos".to_string()]);
        Ok(())
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn test_get_checksum() -> anyhow::Result<()> {
        let pool = MySqlPool::new(&env::var("TEST_DB_URL")?).await?;
        let mut tx = pool.begin().await?;
        let helper = MySql::default();
        assert_eq!(helper.get_checksum(&mut tx, "todos".to_string()).await?, 0);
        Ok(())
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn test_with_transaction() -> anyhow::Result<()> {
        let pool = MySqlPool::new(&env::var("TEST_DB_URL")?).await?;
        let dir = tempdir()?;
        let file_path = dir.path().join("todos.yml");
        let fixture_file_path = file_path.clone();
        let mut file = File::create(file_path)?;
        writeln!(
            file,
            r#"
        - id: 1
          description: fizz
          done: false
          progress: 10.5
          created_at: 2020/01/01 01:01:01"#
        )
        .unwrap();

        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.location(Utc);
        loader.helper = Some(Box::new(MySql::default()));
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
        let result = loader
            .helper
            .unwrap()
            .with_transaction(&pool, &loader.fixture_files)
            .await;

        if let Err(err) = result {
            panic!("test error: {}", err)
        };

        let mut cursor =
            sqlx::query("SELECT id, description, done, progress, created_at FROM todos")
                .fetch(&pool);
        let row = cursor.next().await?.unwrap();
        let id: u16 = row.get("id");
        let description: String = row.get("description");
        let done: bool = row.get("done");
        let progress: f32 = row.get("progress");
        let created_at: NaiveDateTime = row.get("created_at");
        assert_eq!(id, 1);
        assert_eq!(description, "fizz");
        assert_eq!(done, false);
        assert_eq!(progress, 10.5);
        assert_eq!(created_at, NaiveDate::from_ymd(2020, 1, 1).and_hms(1, 1, 1));
        Ok(())
    }
}
