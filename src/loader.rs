use crate::database::DB;
use crate::fixture_file::{FixtureFile, InsertSQL};
use crate::mysql;
use sqlx::{Connect, Connection, Database, MySql, MySqlConnection, Pool, Query};
use std::any::type_name;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use yaml_rust::{Yaml, YamlLoader};

pub type MySqlLoader = Loader<MySql, MySqlConnection>;

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
    pub async fn new(
        options: Vec<Box<dyn FnOnce(&mut Loader<T, C>)>>,
    ) -> anyhow::Result<Loader<T, C>> {
        let mut loader = Self::default();
        for o in options {
            o(&mut loader);
        }
        loader.helper = Self::dialect();
        loader.build_insert_sqls();
        loader
            .helper
            .as_mut()
            .unwrap()
            .init(loader.pool.as_ref().unwrap())
            .await?;
        Ok(loader)
    }

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

    fn dialect() -> Option<Box<dyn DB<T, C> + Send + Sync>> {
        // TODO: postgres, sqlite
        let dialect = match type_name::<T>() {
            "sqlx_core::mysql::database::MySql" => Box::new(mysql::MySql { tables: vec![] }),
            _ => Box::new(mysql::MySql { tables: vec![] }),
        };
        Some(dialect)
    }

    pub fn skip_test_database_check() -> Box<dyn FnOnce(&mut Loader<T, C>)> {
        Box::new(|loader| loader.skip_test_database_check = true)
    }

    pub fn location(location: &str) -> Box<dyn FnOnce(&mut Loader<T, C>)> {
        let location = location.to_string();
        Box::new(|loader| loader.location = Some(location))
    }

    // TODO: complete this function
    // pub fn directory(mut self, directory: String) -> Self {
    //     let fixtures = self.fixtures_from_dir(directory);
    //     self.fixturesFiles = Some(fixtures);
    //     self
    // }

    pub fn files(files: Vec<&str>) -> Box<dyn FnOnce(&mut Loader<T, C>)> {
        let fixtures = Self::fixtures_from_files(files);
        Box::new(|loader| loader.fixtures_files = fixtures)
    }

    // TODO: complete this function
    // pub fn fixtures_from_dir(directory: String) -> Vec<FixtureFile> {}

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

    fn build_insert_sqls(&mut self) {
        for index in 0..self.fixtures_files.len() {
            let file = File::open(self.fixtures_files[index].path.clone()).unwrap();
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents).unwrap();
            let records = YamlLoader::load_from_str(contents.as_str()).unwrap();

            match &records[0] {
                Yaml::Array(records) => {
                    for record in records {
                        let (sql, values) =
                            self.build_insert_sql(&self.fixtures_files[index], record);
                        self.fixtures_files[index].insert_sqls.push(InsertSQL {
                            sql: sql,
                            params: values,
                        });
                    }
                }
                _ => (),
            }
        }
    }

    fn build_insert_sql(&self, file: &FixtureFile, record: &Yaml) -> (String, Vec<String>) {
        let mut sql_columns = vec![];
        let mut sql_values = vec![];
        let mut values = vec![];
        match &record {
            Yaml::Hash(hash) => {
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
            }
            _ => (),
        }
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
