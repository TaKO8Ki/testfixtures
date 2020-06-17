use crate::fixture_file::{FixtureFile, InsertSQL, SqlParam};
use crate::helper::Database as DB;
use chrono::{DateTime, Offset, ParseError, TimeZone};
use sqlx::{Connect, Connection, Database, Pool};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use yaml_rust::{Yaml, YamlLoader};

pub struct Loader<T, C, O, Tz>
where
    T: Database + Sync + Send,
    C: Connection<Database = T> + Connect<Database = T> + Sync + Send,
    O: Offset,
    Tz: TimeZone<Offset = O> + Send + Sync,
{
    pub pool: Option<Pool<C>>,
    pub helper: Option<Box<dyn DB<T, C, O, Tz>>>,
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

impl<T, C, O, Tz> Default for Loader<T, C, O, Tz>
where
    T: Database + Sync + Send,
    C: Connection<Database = T> + Connect<Database = T> + Sync + Send,
    O: Offset,
    Tz: TimeZone<Offset = O> + Send + Sync,
{
    fn default() -> Self {
        Loader::<T, C, O, Tz> {
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

impl<T, C, O, Tz> Loader<T, C, O, Tz>
where
    T: Database + Sync + Send,
    C: Connection<Database = T> + Connect<Database = T> + Sync + Send,
    O: Offset + Display + Send + Sync,
    Tz: TimeZone<Offset = O> + Send + Sync,
{
    pub async fn load(&self) -> anyhow::Result<()> {
        self.helper
            .as_ref()
            .unwrap()
            .with_transaction(self.pool.as_ref().unwrap(), &self.fixture_files)
            .await?;
        Ok(())
    }

    pub fn database(&mut self, pool: Pool<C>) {
        self.pool = Some(pool)
    }

    pub fn skip_test_database_check(&mut self) {
        self.skip_test_database_check = true
    }

    pub fn location(&mut self, location: Tz) {
        self.location = Some(location)
    }

    pub fn files(&mut self, files: Vec<&str>) {
        let mut fixtures = Self::fixtures_from_files(files);
        self.fixture_files.append(&mut fixtures)
    }

    pub fn directory(&mut self, directory: &str) {
        let mut fixtures = Self::fixtures_from_directory(directory);
        self.fixture_files.append(&mut fixtures)
    }

    pub fn paths(&mut self, paths: Vec<&str>) {
        let mut fixtures = Self::fixtures_from_paths(paths);
        self.fixture_files.append(&mut fixtures)
    }

    fn try_str_to_date(&self, s: String) -> Result<DateTime<Tz>, ParseError> {
        self.location
            .as_ref()
            .unwrap()
            .datetime_from_str(s.as_str(), "%Y/%m/%d %H:%M:%S")
    }

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
                    self.fixture_files[index].insert_sqls.push(InsertSQL {
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
                let key = match key {
                    Yaml::String(k) => k.to_string(),
                    Yaml::Integer(k) => k.to_string(),
                    _ => "".to_string(),
                };
                sql_columns.push(key);
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
}

#[cfg(test)]
mod tests {
    use crate::fixture_file::{FixtureFile, SqlParam};
    use crate::mysql::loader::MySqlLoader;
    use chrono::prelude::*;
    use chrono::Utc;
    use std::fs::File;
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use yaml_rust::{Yaml, YamlLoader};

    #[test]
    fn test_location() {
        let mut loader = MySqlLoader::<Utc, Utc>::default();
        loader.location(Utc);
        assert_eq!(loader.location.unwrap(), Utc);
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
            assert_eq!(sql_str, format!("INSERT INTO {} (id, description, created_at, updated_at) VALUES (?, ?, ?, NOW())", fixture_file.file_stem()));
            if let SqlParam::Integer(param) = &values[0] {
                assert_eq!(*param, 1)
            }
            if let SqlParam::String(param) = &values[1] {
                assert_eq!(*param, "fizz".to_string())
            }
            if let SqlParam::Datetime(param) = &values[2] {
                assert_eq!(*param, Utc.ymd(2020, 1, 1).and_hms(1, 1, 1))
            }
        }
    }
}
