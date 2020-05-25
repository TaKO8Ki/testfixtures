use sqlx::MySqlPool;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug)]
pub struct Loader {
    pub db: Option<MySqlPool>,
    pub helper: Option<Dialect>,
    pub fixtures_files: Vec<FixtureFile>,
    pub skip_test_database_check: Option<bool>,
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

impl Default for Loader {
    fn default() -> Loader {
        Loader {
            db: None,
            helper: None,
            fixtures_files: vec![],
            skip_test_database_check: None,
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

#[derive(Debug)]
pub enum Dialect {
    MySql,
}

impl Loader {
    pub fn new(option: Vec<Box<dyn FnOnce(&mut Loader)>>) -> Loader {
        let mut loader: Loader = Default::default();
        for o in option {
            o(&mut loader);
        }
        loader.build_insert_sqls();
        loader
    }

    pub async fn load(self) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
    INSERT INTO todos ( description )
    VALUES ( ? )
            "#,
            "hfoae"
        )
        .execute(&self.db.unwrap())
        .await?;
        // for file in self.fixtures_files {
        //  ;   .unwrap();
        // }
        Ok(())
    }

    // pub fn database(mut self, db: MySqlPool) -> Self {
    //     self.db = Some(db);
    //     self
    // }

    pub fn database(db: MySqlPool) -> Box<dyn FnOnce(&mut Loader)> {
        Box::new(|loader| loader.db = Some(db))
    }

    pub fn dialect(dialect: &str) -> Box<dyn FnOnce(&mut Loader)> {
        let dialect = match dialect {
            "mysql" | "mariadb" => Some(Dialect::MySql),
            _ => None,
        };
        Box::new(|loader| loader.helper = dialect)
    }

    // pub fn directory(mut self, directory: String) -> Self {
    //     let fixtures = self.fixtures_from_dir(directory);
    //     self.fixturesFiles = Some(fixtures);
    //     self
    // }

    pub fn files(files: Vec<&str>) -> Box<dyn FnOnce(&mut Loader)> {
        let fixtures = Loader::fixtures_from_files(files);
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
            let file = File::open(&self.fixtures_files[index].path.clone()).unwrap();
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
                            sql: sql.to_string(),
                            params: values.to_vec(),
                        });
                    }
                }
                others => (),
            }
        }
    }

    fn build_insert_sql(&self, file: &FixtureFile, record: &Yaml) -> (String, Vec<String>) {
        match &record {
            Yaml::Hash(hash) => {
                for (i, v) in hash {
                    println!("{:?}: {:?}", i, v);
                }
            }
            others => (),
        }
        ("feh".to_string(), vec!["foewaf".to_string()])
    }
}
