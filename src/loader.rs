use crate::fixture_file::{FixtureFile, InsertSQL};
use crate::helper::Database as DB;
use sqlx::{Connect, Connection, Database, Pool, Query};
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use yaml_rust::{Yaml, YamlLoader};

pub struct Loader<T, C>
where
    T: Database + Sync + Send,
    C: Connection<Database = T> + Connect<Database = T> + Sync + Send,
{
    pub pool: Option<Pool<C>>,
    pub helper: Option<Box<dyn DB<T, C> + Send + Sync>>,
    pub fixtures_files: Vec<FixtureFile>,
    pub skip_test_database_check: bool,
    pub location: Option<String>,
    pub template: Option<bool>,
    pub template_funcs: Option<String>,
    pub template_left_delim: Option<String>,
    pub template_right_delim: Option<String>,
    pub template_options: Option<Vec<String>>,
    pub template_data: Option<String>,
}

impl<T, C> Default for Loader<T, C>
where
    T: Database + Sync + Send,
    C: Connection<Database = T> + Connect<Database = T> + Sync + Send,
{
    fn default() -> Self {
        Loader::<T, C> {
            pool: None,
            helper: None,
            fixtures_files: vec![],
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

impl<T, C> Loader<T, C>
where
    T: Database + Sync + Send,
    C: Connection<Database = T> + Connect<Database = T> + Sync + Send,
{
    pub async fn load(&self) -> anyhow::Result<()> {
        let mut queries = vec![];

        let delete_queries = self.delete_queries();
        let mut delete_queries: Vec<Query<'_, T>> =
            delete_queries.iter().map(|x| sqlx::query(x)).collect();

        queries.append(&mut delete_queries);

        for fixtures_file in &self.fixtures_files {
            for i in &fixtures_file.insert_sqls {
                queries.push(sqlx::query(i.sql.as_str()))
            }
        }

        self.helper
            .as_ref()
            .unwrap()
            .with_transaction(self.pool.as_ref().unwrap(), queries)
            .await?;
        Ok(())
    }

    pub fn database(pool: Pool<C>) -> Box<dyn FnOnce(&mut Loader<T, C>)> {
        Box::new(|loader| loader.pool = Some(pool))
    }

    pub fn skip_test_database_check() -> Box<dyn FnOnce(&mut Loader<T, C>)> {
        Box::new(|loader| loader.skip_test_database_check = true)
    }

    pub fn location(location: &str) -> Box<dyn FnOnce(&mut Loader<T, C>)> {
        let location = location.to_string();
        Box::new(|loader| loader.location = Some(location))
    }

    pub fn files(files: Vec<&str>) -> Box<dyn FnOnce(&mut Loader<T, C>)> {
        let mut fixtures = Self::fixtures_from_files(files);
        Box::new(move |loader| loader.fixtures_files.append(&mut fixtures))
    }

    pub fn directory(directory: &str) -> Box<dyn FnOnce(&mut Loader<T, C>)> {
        let mut fixtures = Self::fixtures_from_directory(directory);
        Box::new(move |loader| loader.fixtures_files.append(&mut fixtures))
    }

    pub fn paths(paths: Vec<&str>) -> Box<dyn FnOnce(&mut Loader<T, C>)> {
        let mut fixtures = Self::fixtures_from_paths(paths);
        Box::new(move |loader| loader.fixtures_files.append(&mut fixtures))
    }

    fn fixtures_from_files(files: Vec<&str>) -> Vec<FixtureFile> {
        let mut fixture_files: Vec<FixtureFile> = vec![];
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

    fn fixtures_from_directory(directory: &str) -> Vec<FixtureFile> {
        let mut fixture_files: Vec<FixtureFile> = vec![];
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

    fn fixtures_from_paths(paths: Vec<&str>) -> Vec<FixtureFile> {
        let mut fixture_files: Vec<FixtureFile> = vec![];
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
        for index in 0..self.fixtures_files.len() {
            let file = &self.fixtures_files[index].content;
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents).unwrap();
            let records = YamlLoader::load_from_str(contents.as_str()).unwrap();

            if let Yaml::Array(records) = &records[0] {
                for record in records {
                    let (sql, values) = self.build_insert_sql(&self.fixtures_files[index], record);
                    self.fixtures_files[index].insert_sqls.push(InsertSQL {
                        sql,
                        params: values,
                    });
                }
            };
        }
    }

    fn build_insert_sql(&self, file: &FixtureFile, record: &Yaml) -> (String, Vec<String>) {
        let mut sql_columns = vec![];
        let mut sql_values = vec![];
        let mut values = vec![];
        if let Yaml::Hash(hash) = &record {
            for (key, value) in hash {
                let value = match value {
                    Yaml::String(v) => format!(r#""{}""#, v.to_string()),
                    Yaml::Integer(v) => v.to_string(),
                    _ => "".to_string(),
                };
                let key = match key {
                    Yaml::String(k) => k.to_string(),
                    Yaml::Integer(k) => k.to_string(),
                    _ => "".to_string(),
                };
                sql_columns.push(key);
                if value.starts_with("RAW=") {
                    sql_values.push(value.replace("RAW=", ""));
                    continue;
                }

                sql_values.push("?".to_string());
                values.push(value);
            }
        };

        let sql_str = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            file.file_stem(),
            sql_columns.join(", "),
            values.join(", "),
        );
        (sql_str, values)
    }

    fn delete_queries(&self) -> Vec<String> {
        self.fixtures_files.iter().map(|x| (x.delete())).collect()
    }
}
