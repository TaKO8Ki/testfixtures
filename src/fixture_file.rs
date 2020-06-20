use chrono::{DateTime, TimeZone};
use std::fs::File;
use std::path::Path;

pub struct FixtureFile<Tz: TimeZone + Send + Sync> {
    pub path: String,
    pub file_name: String,
    pub content: File,
    pub insert_sqls: Vec<InsertSql<Tz>>,
}

pub struct InsertSql<Tz: TimeZone + Send + Sync> {
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
    Float(f32),
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_file_stem() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.yml");
        let fixture_file_path = file_path.clone();
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

        let fixture_file = FixtureFile::<Utc> {
            path: fixture_file_path.to_str().unwrap().to_string(),
            file_name: fixture_file_path
                .clone()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            content: File::open(fixture_file_path.clone()).unwrap(),
            insert_sqls: vec![],
        };

        assert_eq!(fixture_file.file_stem(), "test");
        Ok(())
    }

    #[test]
    fn test_delete() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.yml");
        let fixture_file_path = file_path.clone();
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

        let fixture_file = FixtureFile::<Utc> {
            path: fixture_file_path.to_str().unwrap().to_string(),
            file_name: fixture_file_path
                .clone()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            content: File::open(fixture_file_path.clone()).unwrap(),
            insert_sqls: vec![],
        };

        assert_eq!(fixture_file.delete(), "DELETE FROM test");
        Ok(())
    }
}
