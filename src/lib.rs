use sqlx::MySqlPool;
use std::fs::File;
use std::path::Path;

#[derive(Debug)]
pub struct Loader {
    pub db: Option<MySqlPool>,
    pub helper: Option<Dialect>,
    pub fixtures_files: Option<Vec<FixtureFile>>,
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
    pub insert_sqls: Vec<String>,
}

pub struct InsertSQL {
    pub sql: String,
    pub params: Vec<String>,
}

impl Default for Loader {
    fn default() -> Loader {
        Loader {
            db: None,
            helper: None,
            fixtures_files: None,
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
        loader
    }

    // pub fn database(mut self, db: MySqlPool) -> Self {
    //     self.db = Some(db);
    //     self
    // }

    pub fn database(db: MySqlPool) -> Box<dyn FnOnce(&mut Loader)> {
        Box::new(|loader| loader.db = Some(db))
    }

    pub fn dialect(dialect: String) -> Box<dyn FnOnce(&mut Loader)> {
        let dialect = match dialect.as_str() {
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

    pub fn files(files: Vec<String>) -> Box<dyn FnOnce(&mut Loader)> {
        let fixtures = Loader::fixtures_from_files(files);
        Box::new(|loader| loader.fixtures_files = Some(fixtures))
    }

    // pub fn fixtures_from_dir(directory: String) -> Vec<FixtureFile> {}

    fn fixtures_from_files(files: Vec<String>) -> Vec<FixtureFile> {
        let mut fixture_files: Vec<FixtureFile> = vec![];
        for f in files {
            let fixture = FixtureFile {
                path: f.clone(),
                file_name: Path::new(f.as_str())
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                content: File::open(f.clone().as_str()).unwrap(),
                insert_sqls: vec!["feafewa".to_string()],
            };
            fixture_files.push(fixture);
        }
        fixture_files
    }
}
