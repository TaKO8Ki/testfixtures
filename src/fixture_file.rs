use chrono::{DateTime, TimeZone};
use std::fs::File;
use std::path::Path;

pub struct FixtureFile<Tz: TimeZone + Send + Sync> {
    pub path: String,
    pub file_name: String,
    pub content: File,
    pub insert_sqls: Vec<InsertSQL<Tz>>,
}

pub struct InsertSQL<Tz: TimeZone + Send + Sync> {
    pub sql: String,
    pub params: Vec<SqlParam<Tz>>,
}

pub enum SqlParam<Tz>
where
    Tz: TimeZone + Send + Sync,
{
    String(String),
    Datetime(DateTime<Tz>),
    Integer(u32),
}

impl<Tz> FixtureFile<Tz>
where
    Tz: TimeZone + Send + Sync,
{
    pub(crate) fn file_stem(&self) -> String {
        Path::new(self.file_name.as_str())
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    pub(crate) fn delete(&self) -> String {
        format!("DELETE FROM {}", self.file_stem())
    }
}
