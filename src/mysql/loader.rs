use crate::loader::Loader;
use crate::mysql::helper;
use chrono::{Offset, TimeZone};
use sqlx::{MySql, MySqlConnection};
use std::fmt::Display;

/// An alias for [Loader](testfixtures::Loader), specialized for **MySQL**.
pub type MySqlLoader<O, Tz> = Loader<MySql, MySqlConnection, O, Tz>;

impl<O, Tz> MySqlLoader<O, Tz>
where
    O: Offset + Display + Send + Sync + 'static,
    Tz: TimeZone<Offset = O> + Send + Sync + 'static,
{
    /// Creates a Loader, specialized for MySQL and Set options.
    ///
    /// # Example
    /// ```rust
    /// #[cfg(test)]
    /// mod tests {
    ///     use testfixtures::MySqlLoader;
    ///     #[async_std::test]
    ///     async fn test_something() -> anyhow::Result<()> {
    ///         let loader = MySqlLoader::new(|cfg| {
    ///             //...
    ///         })
    ///         .await?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub async fn new<F>(options: F) -> anyhow::Result<MySqlLoader<O, Tz>>
    where
        F: FnOnce(&mut MySqlLoader<O, Tz>),
    {
        let mut loader = Self::default();
        options(&mut loader);
        loader.helper = Some(Box::new(helper::MySql { tables: vec![] }));
        loader.build_insert_sqls();
        loader
            .helper
            .as_mut()
            .unwrap()
            .init(loader.pool.as_ref().unwrap())
            .await?;
        Ok(loader)
    }
}

#[cfg(test)]
mod tests {
    use crate::mysql::loader::MySqlLoader;
    use chrono::Utc;
    use sqlx::MySqlPool;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[async_std::test]
    async fn test_new() -> anyhow::Result<()> {
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

        let pool = MySqlPool::new("fizz").await?;
        let loader = MySqlLoader::new(|cfg| {
            cfg.location(Utc);
            cfg.database(pool);
            cfg.skip_test_database_check();
            cfg.files(vec![tempfile.path().to_str().unwrap()]);
        })
        .await?;

        assert_eq!(loader.location.unwrap(), Utc);
        assert!(loader.pool.is_some());
        assert!(loader.skip_test_database_check);
        assert!(loader.helper.is_some());
        assert_eq!(loader.fixture_files.len(), 1);
        Ok(())
    }
}
