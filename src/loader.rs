use crate::fixture_file::{FixtureFile, InsertSql, SqlParam};
use crate::helper::Database as DB;
use chrono::{DateTime, Offset, TimeZone};
use regex::Regex;
use sqlx::{Connect, Connection, Database, Pool};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;
use yaml_rust::{Yaml, YamlLoader};

/// This type accepts and set some options.
pub struct Loader<D, C, O, Tz>
where
    D: Database + Sync + Send,
    C: Connection<Database = D> + Connect<Database = D> + Sync + Send,
    O: Offset,
    Tz: TimeZone<Offset = O> + Send + Sync,
{
    pub pool: Option<Pool<C>>,
    pub helper: Option<Box<dyn DB<D, C, O, Tz>>>,
    pub fixture_files: Vec<FixtureFile<Tz>>,
    pub skip_test_database_check: bool,
    pub location: Option<Tz>,
    pub template: Option<bool>,
    pub template_funcs: Option<String>,
    pub template_left_delim: Option<String>,
    pub template_right_delim: Option<String>,
    pub template_options: Option<Vec<String>>,
    pub template_data: Option<String>,
}

impl<D, C, O, Tz> Default for Loader<D, C, O, Tz>
where
    D: Database + Sync + Send,
    C: Connection<Database = D> + Connect<Database = D> + Sync + Send,
    O: Offset,
    Tz: TimeZone<Offset = O> + Send + Sync,
{
    fn default() -> Self {
        Loader::<D, C, O, Tz> {
            pool: None,
            helper: None,
            fixture_files: vec![],
            skip_test_database_check: false,
            location: None,
            template: None,
            template_funcs: None,
            template_left_delim: None,
            template_right_delim: None,
            template_options: None,
            template_data: None,
        }
    }
}

impl<D, C, O, Tz> Loader<D, C, O, Tz>
where
    D: Database + Sync + Send,
    C: Connection<Database = D> + Connect<Database = D> + Sync + Send,
    O: Offset + Display + Send + Sync,
    Tz: TimeZone<Offset = O> + Send + Sync,
{
    /// Execute SQL queries builded from yaml files.
    pub async fn load(&self) -> anyhow::Result<()> {
        if !self.skip_test_database_check {
            if let Err(err) = self.ensure_test_database().await {
                return Err(anyhow::anyhow!("testfixtures error: {}", err));
            }
        }

        self.helper
            .as_ref()
            .unwrap()
            .with_transaction(self.pool.as_ref().unwrap(), &self.fixture_files)
            .await?;
        Ok(())
    }

    /// Set database pool.
    pub fn database(&mut self, pool: Pool<C>) {
        self.pool = Some(pool)
    }

    /// Turn test database check off.
    pub fn skip_test_database_check(&mut self) {
        self.skip_test_database_check = true
    }

    /// Set timezone.
    pub fn location(&mut self, location: Tz) {
        self.location = Some(location)
    }

    /// Set fixture files directly.
    pub fn files(&mut self, files: Vec<&str>) {
        let mut fixtures = Self::fixtures_from_files(files);
        self.fixture_files.append(&mut fixtures)
    }

    /// Set fixture files from a directory.
    pub fn directory(&mut self, directory: &str) {
        let mut fixtures = Self::fixtures_from_directory(directory);
        self.fixture_files.append(&mut fixtures)
    }

    /// This option is a combination of files option and directory option.
    pub fn paths(&mut self, paths: Vec<&str>) {
        let mut fixtures = Self::fixtures_from_paths(paths);
        self.fixture_files.append(&mut fixtures)
    }

    /// Try change str to datetime.
    fn try_str_to_date(&self, s: String) -> anyhow::Result<DateTime<Tz>> {
        let formats = vec![
            "%Y-%m-%d %H:%M",
            "%Y-%m-%d %H:%M:%S",
            "%Y%m%d %H:%M",
            "%Y%m%d %H:%M:%S",
            "%d%m%Y %H:%M",
            "%d%m%Y %H:%M:%S",
            "%Y/%m/%d %H:%M",
            "%Y/%m/%d %H:%M:%S",
        ];
        for f in formats {
            let result = self
                .location
                .as_ref()
                .unwrap()
                .datetime_from_str(s.as_str(), f);
            if let Ok(datetime) = result {
                return Ok(datetime);
            }
        }
        Err(anyhow::anyhow!(
            "testfixtures error: datetime format is invalid"
        ))
    }

    /// Set fixture file content to FixtureFile struct.
    fn fixtures_from_files(files: Vec<&str>) -> Vec<FixtureFile<Tz>> {
        let mut fixture_files: Vec<FixtureFile<Tz>> = vec![];
        for f in files {
            let fixture = FixtureFile {
                path: f.to_string(),
                file_name: Path::new(f)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                content: File::open(f).unwrap(),
                insert_sqls: vec![],
            };
            fixture_files.push(fixture);
        }
        fixture_files
    }

    /// Set fixture file content from a directory to [FixtureFile](crate::fixture_file::FixtureFile) struct.
    fn fixtures_from_directory(directory: &str) -> Vec<FixtureFile<Tz>> {
        let mut fixture_files: Vec<FixtureFile<Tz>> = vec![];
        for f in fs::read_dir(directory).unwrap() {
            let f = f.unwrap();
            let fixture = FixtureFile {
                path: f.path().to_str().unwrap().to_string(),
                file_name: f.file_name().to_str().unwrap().to_string(),
                content: File::open(f.path()).unwrap(),
                insert_sqls: vec![],
            };
            fixture_files.push(fixture);
        }
        fixture_files
    }

    /// Set fixture file content from a directory to [FixtureFile](crate::fixture_file::FixtureFile) struct.
    fn fixtures_from_paths(paths: Vec<&str>) -> Vec<FixtureFile<Tz>> {
        let mut fixture_files: Vec<FixtureFile<Tz>> = vec![];
        for path in paths {
            if Path::new(path).is_dir() {
                fixture_files.append(&mut Self::fixtures_from_directory(path))
            } else {
                fixture_files.append(&mut Self::fixtures_from_files(vec![path]))
            }
        }
        fixture_files
    }

    /// Build SQL queries from fixture files.
    pub(crate) fn build_insert_sqls(&mut self) {
        for index in 0..self.fixture_files.len() {
            let file = &self.fixture_files[index].content;
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents).unwrap();
            let records = YamlLoader::load_from_str(contents.as_str()).unwrap();

            if let Yaml::Array(records) = &records[0] {
                for record in records {
                    let (sql, values) = self.build_insert_sql(&self.fixture_files[index], record);
                    self.fixture_files[index].insert_sqls.push(InsertSql {
                        sql,
                        params: values,
                    });
                }
            };
        }
    }

    fn build_insert_sql(
        &self,
        file: &FixtureFile<Tz>,
        record: &Yaml,
    ) -> (String, Vec<SqlParam<Tz>>) {
        let mut sql_columns = vec![];
        let mut sql_values = vec![];
        let mut values = vec![];
        if let Yaml::Hash(hash) = &record {
            for (key, value) in hash {
                match key {
                    Yaml::String(k) => sql_columns.push(k.to_string()),
                    Yaml::Integer(k) => sql_columns.push(k.to_string()),
                    _ => (),
                };
                match value {
                    Yaml::String(v) => {
                        if v.starts_with("RAW=") {
                            sql_values.push(v.replace("RAW=", ""));
                            continue;
                        } else {
                            match self.try_str_to_date(v.to_string()) {
                                Ok(datetime) => values.push(SqlParam::Datetime(datetime)),
                                Err(_) => values.push(SqlParam::String(v.to_string())),
                            }
                        }
                    }
                    Yaml::Integer(v) => values.push(SqlParam::Integer(*v as u32)),
                    Yaml::Real(v) => values.push(SqlParam::Float(f32::from_str(v).unwrap())),
                    Yaml::Boolean(v) => values.push(SqlParam::Boolean(*v)),
                    _ => (),
                };
                sql_values.push("?".to_string());
            }
        };

        let sql_str = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            file.file_stem(),
            sql_columns.join(", "),
            sql_values.join(", "),
        );
        (sql_str, values)
    }

    // Check if database name ends with test.
    async fn ensure_test_database(&self) -> anyhow::Result<()> {
        let db_name = self
            .helper
            .as_ref()
            .unwrap()
            .database_name(self.pool.as_ref().unwrap())
            .await?;
        let re = Regex::new(r"^*?test$")?;
        if !re.is_match(db_name.as_str()) {
            return Err(anyhow::anyhow!(
                r#"'{}' does not appear to be a test database"#,
                db_name
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::fixture_file::{FixtureFile, SqlParam};
    use crate::helper::Database as DB;
    use crate::mysql::loader::MySqlLoader;
    use async_trait::async_trait;
    use chrono::{prelude::*, Utc};
    use sqlx::{MySql as M, MySqlConnection, MySqlPool};
    use std::fs::File;
    use std::io::{prelude::*, BufReader, Write};
    use tempfile::{tempdir, NamedTempFile};
    use yaml_rust::{Yaml, YamlLoader};

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn it_returns_ok() -> anyhow::Result<()> {
        pub struct TestLoadNormal {}
        impl Default for TestLoadNormal {
            fn default() -> Self {
                TestLoadNormal {}
            }
        }
        #[async_trait]
        impl<O, Tz> DB<M, MySqlConnection, O, Tz> for TestLoadNormal
        where
            O: Offset + Sync + Send + 'static,
            Tz: TimeZone<Offset = O> + Send + Sync + 'static,
        {
            async fn init(&mut self, _pool: &MySqlPool) -> anyhow::Result<()> {
                Ok(())
            }

            async fn database_name(&self, _pool: &MySqlPool) -> anyhow::Result<String> {
                Ok("test".to_string())
            }

            async fn with_transaction(
                &self,
                _pool: &MySqlPool,
                _fixture_files: &[FixtureFile<Tz>],
            ) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.pool = Some(MySqlPool::new("fizz").await?);
        loader.helper = Some(Box::new(TestLoadNormal {}));
        let result = loader.load().await;
        assert!(result.is_ok());
        Ok(())
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn it_returns_transaction_error() -> anyhow::Result<()> {
        pub struct TestLoadTransactionError {}
        impl Default for TestLoadTransactionError {
            fn default() -> Self {
                TestLoadTransactionError {}
            }
        }
        #[async_trait]
        impl<O, Tz> DB<M, MySqlConnection, O, Tz> for TestLoadTransactionError
        where
            O: Offset + Sync + Send + 'static,
            Tz: TimeZone<Offset = O> + Send + Sync + 'static,
        {
            async fn init(&mut self, _pool: &MySqlPool) -> anyhow::Result<()> {
                Ok(())
            }

            async fn database_name(&self, _pool: &MySqlPool) -> anyhow::Result<String> {
                Ok("test".to_string())
            }

            async fn with_transaction(
                &self,
                _pool: &MySqlPool,
                _fixture_files: &[FixtureFile<Tz>],
            ) -> anyhow::Result<()> {
                Err(anyhow::anyhow!("error"))
            }
        }

        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.pool = Some(MySqlPool::new("fizz").await?);
        loader.helper = Some(Box::new(TestLoadTransactionError {}));
        let result = loader.load().await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err.to_string(), "error");
        }
        Ok(())
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn it_returns_dabatase_check_error() -> anyhow::Result<()> {
        pub struct TestLoadDatabaseCheckError {}
        impl Default for TestLoadDatabaseCheckError {
            fn default() -> Self {
                TestLoadDatabaseCheckError {}
            }
        }
        #[async_trait]
        impl<O, Tz> DB<M, MySqlConnection, O, Tz> for TestLoadDatabaseCheckError
        where
            O: Offset + Sync + Send + 'static,
            Tz: TimeZone<Offset = O> + Send + Sync + 'static,
        {
            async fn init(&mut self, _pool: &MySqlPool) -> anyhow::Result<()> {
                Ok(())
            }

            async fn database_name(&self, _pool: &MySqlPool) -> anyhow::Result<String> {
                Ok("fizz".to_string())
            }

            async fn with_transaction(
                &self,
                _pool: &MySqlPool,
                _fixture_files: &[FixtureFile<Tz>],
            ) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.pool = Some(MySqlPool::new("fizz").await?);
        loader.helper = Some(Box::new(TestLoadDatabaseCheckError {}));
        let result = loader.load().await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(
                err.to_string(),
                r#"testfixtures error: 'fizz' does not appear to be a test database"#
            );
        }
        Ok(())
    }

    #[test]
    fn test_location() {
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.location(Utc);
        assert_eq!(loader.location.unwrap(), Utc);
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn test_database() -> anyhow::Result<()> {
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        let database = MySqlPool::new("fizz").await?;
        loader.database(database);
        assert!(loader.pool.is_some());
        Ok(())
    }

    #[test]
    fn test_skip_test_database_check() {
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.skip_test_database_check();
        assert!(loader.skip_test_database_check);
    }

    #[test]
    fn test_files() {
        let mut tempfile = NamedTempFile::new().unwrap();
        writeln!(
            tempfile,
            r#"
        - id: 1
          description: fizz
          created_at: 2020/01/01 01:01:01
          updated_at: RAW=NOW()"#
        )
        .unwrap();
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.files(vec![tempfile.path().to_str().unwrap()]);
        assert_eq!(
            loader.fixture_files[0].file_name,
            tempfile
                .path()
                .clone()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        );
    }

    #[test]
    fn test_directory() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.yml");
        let mut file = File::create(file_path)?;
        writeln!(
            file,
            r#"
        - id: 1
          description: fizz
          created_at: 2020/01/01 01:01:01
          updated_at: RAW=NOW()"#
        )
        .unwrap();
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.directory(dir.path().to_str().unwrap());
        assert_eq!(loader.fixture_files[0].file_name, "test.yml");
        Ok(())
    }

    #[test]
    fn test_paths() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.yml");
        let mut file = File::create(file_path)?;
        let mut tempfile = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
        - id: 1
          description: fizz
          created_at: 2020/01/01 01:01:01
          updated_at: RAW=NOW()"#
        )
        .unwrap();
        writeln!(
            tempfile,
            r#"
        - id: 1
          description: fizz
          created_at: 2020/01/01 01:01:01
          updated_at: RAW=NOW()"#
        )
        .unwrap();
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.paths(vec![
            dir.path().to_str().unwrap(),
            tempfile.path().to_str().unwrap(),
        ]);
        Ok(())
    }

    #[test]
    fn test_try_str_to_date() {
        struct Test {
            argument: String,
            want_err: bool,
        };
        let tests: [Test; 9] = [
            Test {
                argument: "2020-01-01 01:01:01".to_string(),
                want_err: false,
            },
            Test {
                argument: "2020-01-01 01:01".to_string(),
                want_err: false,
            },
            Test {
                argument: "2020/01/01 01:01:01".to_string(),
                want_err: false,
            },
            Test {
                argument: "2020/01/01 01:01".to_string(),
                want_err: false,
            },
            Test {
                argument: "01012020 01:01:01".to_string(),
                want_err: false,
            },
            Test {
                argument: "01012020 01:01".to_string(),
                want_err: false,
            },
            Test {
                argument: "2020-01-01".to_string(),
                want_err: true,
            },
            Test {
                argument: "2020/01/01".to_string(),
                want_err: true,
            },
            Test {
                argument: "01012020".to_string(),
                want_err: true,
            },
        ];
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.location(Utc);
        for t in &tests {
            if t.want_err {
                assert!(loader.try_str_to_date(t.argument.to_string()).is_err());
            } else {
                assert!(loader.try_str_to_date(t.argument.to_string()).is_ok());
            }
        }
    }

    #[test]
    fn test_fixtures_from_files() {
        let mut tempfile = NamedTempFile::new().unwrap();
        writeln!(
            tempfile,
            r#"
        - id: 1
          description: fizz
          created_at: 2020/01/01 01:01:01
          updated_at: RAW=NOW()"#
        )
        .unwrap();
        let fixture_files =
            MySqlLoader::<Utc, Utc>::fixtures_from_files(vec![tempfile.path().to_str().unwrap()]);
        assert_eq!(
            fixture_files[0].file_name,
            tempfile
                .path()
                .clone()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        );
    }

    #[test]
    fn test_fixtures_from_directory() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.yml");
        let mut file = File::create(file_path)?;
        writeln!(
            file,
            r#"
        - id: 1
          description: fizz
          created_at: 2020/01/01 01:01:01
          updated_at: RAW=NOW()"#
        )
        .unwrap();
        let fixture_files =
            MySqlLoader::<Utc, Utc>::fixtures_from_directory(dir.path().to_str().unwrap());
        assert_eq!(fixture_files[0].file_name, "test.yml");
        Ok(())
    }

    #[test]
    fn test_fixtures_from_paths() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.yml");
        let mut file = File::create(file_path)?;
        let mut tempfile = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
        - id: 1
          description: fizz
          created_at: 2020/01/01 01:01:01
          updated_at: RAW=NOW()"#
        )
        .unwrap();
        writeln!(
            tempfile,
            r#"
        - id: 1
          description: fizz
          created_at: 2020/01/01 01:01:01
          updated_at: RAW=NOW()"#
        )
        .unwrap();
        let fixture_files = MySqlLoader::<Utc, Utc>::fixtures_from_paths(vec![
            dir.path().to_str().unwrap(),
            tempfile.path().to_str().unwrap(),
        ]);
        assert_eq!(fixture_files[0].file_name, "test.yml");
        assert_eq!(
            fixture_files[1].file_name,
            tempfile
                .path()
                .clone()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        );
        Ok(())
    }

    #[test]
    fn test_build_insert_sql() {
        // different columns have different types.
        let mut tempfile = NamedTempFile::new().unwrap();
        writeln!(
            tempfile,
            r#"
        - id: 1
          description: fizz
          price: 1.1
          created_at: 2020/01/01 01:01:01
          updated_at: RAW=NOW()"#
        )
        .unwrap();

        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.location(Utc);
        let fixture_file = FixtureFile {
            path: tempfile.path().to_str().unwrap().to_string(),
            file_name: tempfile
                .path()
                .clone()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            content: File::open(tempfile).unwrap(),
            insert_sqls: vec![],
        };
        let mut buf_reader = BufReader::new(&fixture_file.content);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        let records = YamlLoader::load_from_str(contents.as_str()).unwrap();
        if let Yaml::Array(records) = &records[0] {
            let (sql_str, values) = loader.build_insert_sql(&fixture_file, &records[0]);
            assert_eq!(sql_str, format!("INSERT INTO {} (id, description, price, created_at, updated_at) VALUES (?, ?, ?, ?, NOW())", fixture_file.file_stem()));
            assert_eq!(values.len(), 4);
            if let SqlParam::Integer(param) = &values[0] {
                assert_eq!(*param, 1)
            }
            if let SqlParam::String(param) = &values[1] {
                assert_eq!(*param, "fizz".to_string())
            }
            if let SqlParam::Float(param) = &values[2] {
                assert_eq!(*param, 1.1)
            }
            if let SqlParam::Datetime(param) = &values[3] {
                assert_eq!(*param, Utc.ymd(2020, 1, 1).and_hms(1, 1, 1))
            }
        }
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn test_ensure_test_database() -> anyhow::Result<()> {
        pub struct TestEnsureTestDatabaseNormal {}
        impl Default for TestEnsureTestDatabaseNormal {
            fn default() -> Self {
                TestEnsureTestDatabaseNormal {}
            }
        }
        #[async_trait]
        impl<O, Tz> DB<M, MySqlConnection, O, Tz> for TestEnsureTestDatabaseNormal
        where
            O: Offset + Sync + Send + 'static,
            Tz: TimeZone<Offset = O> + Send + Sync + 'static,
        {
            async fn init(&mut self, _pool: &MySqlPool) -> anyhow::Result<()> {
                Ok(())
            }

            async fn database_name(&self, _pool: &MySqlPool) -> anyhow::Result<String> {
                Ok("test".to_string())
            }

            async fn with_transaction(
                &self,
                _pool: &MySqlPool,
                _fixture_files: &[FixtureFile<Tz>],
            ) -> anyhow::Result<()> {
                Ok(())
            }
        }
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.pool = Some(MySqlPool::new("fizz").await?);
        loader.helper = Some(Box::new(TestEnsureTestDatabaseNormal {}));
        let result = loader.ensure_test_database().await?;
        assert_eq!(result, ());

        pub struct TestEnsureTestDatabaseError {}
        impl Default for TestEnsureTestDatabaseError {
            fn default() -> Self {
                TestEnsureTestDatabaseError {}
            }
        }
        #[async_trait]
        impl<O, Tz> DB<M, MySqlConnection, O, Tz> for TestEnsureTestDatabaseError
        where
            O: Offset + Sync + Send + 'static,
            Tz: TimeZone<Offset = O> + Send + Sync + 'static,
        {
            async fn init(&mut self, _pool: &MySqlPool) -> anyhow::Result<()> {
                Ok(())
            }

            async fn database_name(&self, _pool: &MySqlPool) -> anyhow::Result<String> {
                Ok("fizz".to_string())
            }

            async fn with_transaction(
                &self,
                _pool: &MySqlPool,
                _fixture_files: &[FixtureFile<Tz>],
            ) -> anyhow::Result<()> {
                Ok(())
            }
        }
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.pool = Some(MySqlPool::new("fizz").await?);
        loader.helper = Some(Box::new(TestEnsureTestDatabaseError {}));
        let result = loader.ensure_test_database().await;
        assert!(result.is_err());

        Ok(())
    }
}
