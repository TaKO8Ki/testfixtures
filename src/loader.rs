use super::database::Database;
use super::mysql;
use regex::Regex;
use sqlx::MySqlPool;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use yaml_rust::{Yaml, YamlLoader};

pub struct Loader {
    pub db: Option<MySqlPool>,
    pub helper: Option<Box<dyn Database>>,
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

#[derive(Debug)]
pub struct FixtureFile {
    pub path: String,
    pub file_name: String,
    pub content: File,
    pub insert_sqls: Vec<InsertSQL>,
}

#[derive(Debug)]
pub struct InsertSQL {
    pub sql: String,
    pub params: Vec<String>,
}

#[derive(Debug)]
pub enum Dialect {
    MySql,
}

impl<'a> Default for Loader {
    fn default() -> Self {
        Loader {
            db: None,
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

impl Loader {
    pub async fn new(options: Vec<Box<dyn FnOnce(&mut Loader)>>) -> anyhow::Result<Loader> {
        let mut loader = Self::default();
        for o in options {
            o(&mut loader);
        }
        loader.build_insert_sqls();
        loader
            .helper
            .as_mut()
            .unwrap()
            .init(loader.db.as_ref().unwrap())
            .await?;
        loader
            .helper
            .as_ref()
            .unwrap()
            .database_name(loader.db.as_ref().unwrap())
            .await?;
        Ok(loader)
    }

    pub async fn load(&self) -> anyhow::Result<()> {
        if !self.skip_test_database_check {
            if !async { self.ensure_test_database().await }.await.unwrap() {
                panic!("aiueo")
            }
        }
        for index in 0..self.fixtures_files.len() {
            for i in &self.fixtures_files[index].insert_sqls {
                sqlx::query(i.sql.as_str())
                    .execute(self.db.as_ref().unwrap())
                    .await?;
            }
        }
        Ok(())
    }

    pub fn database(db: MySqlPool) -> Box<dyn FnOnce(&mut Loader)> {
        Box::new(move |loader| loader.db = Some(db.clone()))
    }

    pub fn dialect(dialect: &str) -> Box<dyn FnOnce(&mut Loader)> {
        let dialect = match dialect {
            "mysql" | "mariadb" => Box::new(mysql::MySQL::default()),
            _ => Box::new(mysql::MySQL { tables: vec![] }),
        };
        Box::new(|loader| loader.helper = Some(dialect))
    }

    pub fn skip_test_database_check() -> Box<dyn FnOnce(&mut Loader)> {
        Box::new(|loader| loader.skip_test_database_check = true)
    }

    pub fn location(location: &str) -> Box<dyn FnOnce(&mut Loader)> {
        let location = location.to_string();
        Box::new(|loader| loader.location = Some(location))
    }

    // pub fn directory(mut self, directory: String) -> Self {
    //     let fixtures = self.fixtures_from_dir(directory);
    //     self.fixturesFiles = Some(fixtures);
    //     self
    // }

    pub fn files(files: Vec<&str>) -> Box<dyn FnOnce(&mut Loader)> {
        let fixtures = Self::fixtures_from_files(files);
        Box::new(|loader| loader.fixtures_files = fixtures)
    }

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

    async fn ensure_test_database(&self) -> anyhow::Result<bool> {
        let db_name = self
            .helper
            .as_ref()
            .unwrap()
            .database_name(&self.db.as_ref().unwrap())
            .await?;
        let re = Regex::new(r"^.?test$").unwrap();
        Ok(re.is_match(db_name.as_str()))
    }
}

impl FixtureFile {
    fn file_stem(&self) -> String {
        Path::new(self.file_name.as_str())
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }
}
