use std::fs::File;
use std::path::Path;

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

impl FixtureFile {
    pub fn file_stem(&self) -> String {
        Path::new(self.file_name.as_str())
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    pub fn delete(&self) -> String {
        format!("DELETE FROM {}", self.file_stem())
    }
}
