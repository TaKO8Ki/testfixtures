use chrono::{prelude::*, NaiveDate, Utc};
use sqlx::{cursor::Cursor, mysql::MySqlQueryAs, MySqlPool, Row};
use std::env;
use std::fs::File;
use std::io::Write;
use std::panic;
use tempfile::tempdir;
use testfixtures::MySqlLoader;

#[cfg_attr(feature = "runtime-async-std", async_std::test)]
#[cfg_attr(feature = "runtime-tokio", tokio::test)]
async fn it_returns_ok() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("todos.yml");
    let fixture_file_path = file_path.clone();
    let mut file = File::create(file_path)?;
    writeln!(
        file,
        r#"
        - id: 1
          description: fizz
          done: true
          progress: 10.5
          created_at: 2020/01/01 01:01:01
        - id: 2
          description: buzz
          done: false
          progress: 30.0
          created_at: 2020/01/01 02:02:02
        - id: 3
          description: buzz
          done: false
          progress: 25.0
          created_at: RAW=NOW()"#
    )
    .unwrap();

    let pool = MySqlPool::new(&env::var("TEST_DB_URL")?).await?;
    let pool_for_query = pool.clone();
    let loader = MySqlLoader::new(|cfg| {
        cfg.location(Utc);
        cfg.database(pool);
        cfg.paths(vec![fixture_file_path.to_str().unwrap()]);
    })
    .await?;
    let rec: (i32,) = sqlx::query_as("SELECT count(*) from todos")
        .fetch_one(&pool_for_query.clone())
        .await?;
    assert!(loader.load().await.is_ok());
    assert_eq!(rec.0, 1);
    let mut cursor = sqlx::query("SELECT id, description, done, progress, created_at FROM todos")
        .fetch(&pool_for_query);
    let row = cursor.next().await?.unwrap();
    let id: u16 = row.get("id");
    let description: String = row.get("description");
    let done: bool = row.get("done");
    let progress: f32 = row.get("progress");
    let created_at: NaiveDateTime = row.get("created_at");
    assert_eq!(id, 1);
    assert_eq!(description, "fizz");
    assert_eq!(done, true);
    assert_eq!(progress, 10.5);
    assert_eq!(created_at, NaiveDate::from_ymd(2020, 1, 1).and_hms(1, 1, 1));

    let row = cursor.next().await?.unwrap();
    let id: u16 = row.get("id");
    let description: String = row.get("description");
    let done: bool = row.get("done");
    let progress: f32 = row.get("progress");
    let created_at: NaiveDateTime = row.get("created_at");
    assert_eq!(id, 2);
    assert_eq!(description, "buzz");
    assert_eq!(done, false);
    assert_eq!(progress, 30.0);
    assert_eq!(created_at, NaiveDate::from_ymd(2020, 1, 1).and_hms(2, 2, 2));

    let row = cursor.next().await?.unwrap();
    let id: u16 = row.get("id");
    let description: String = row.get("description");
    let done: bool = row.get("done");
    let progress: f32 = row.get("progress");
    assert_eq!(id, 3);
    assert_eq!(description, "buzz");
    assert_eq!(done, false);
    assert_eq!(progress, 25.0);
    // TODO: check if created_at is the expected value.
    Ok(())
}

#[cfg_attr(feature = "runtime-async-std", async_std::test)]
#[cfg_attr(feature = "runtime-tokio", tokio::test)]
async fn it_returns_database_check_error() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("todos.yml");
    let fixture_file_path = file_path.clone();
    let mut file = File::create(file_path)?;
    writeln!(
        file,
        r#"
        - id: 1
          description: fizz
          done: 1
          progress: 10.5"#
    )
    .unwrap();

    let pool = MySqlPool::new(&env::var("TEST_DB_URL_FOR_DB_CHECK")?).await?;
    let loader = MySqlLoader::new(|cfg| {
        cfg.location(Utc);
        cfg.database(pool);
        cfg.paths(vec![fixture_file_path.to_str().unwrap()]);
    })
    .await?;
    let result = loader.load().await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert_eq!(
            err.to_string(),
            r#"testfixtures: 'fizz' does not appear to be a test database"#
        );
    }
    Ok(())
}

#[cfg_attr(feature = "runtime-async-std", async_std::test)]
#[cfg_attr(feature = "runtime-tokio", tokio::test)]
async fn it_returns_transaction_error() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("todos.yml");
    let fixture_file_path = file_path.clone();
    let mut file = File::create(file_path)?;
    writeln!(
        file,
        r#"
        - id: 1
          description: fizz
          done: 1
          progress: 10.5
          updated_at: 2020/01/01 01:01:01"#
    )
    .unwrap();

    let pool = MySqlPool::new(&env::var("TEST_DB_URL")?).await?;
    let loader = MySqlLoader::new(|cfg| {
        cfg.location(Utc);
        cfg.database(pool);
        cfg.paths(vec![fixture_file_path.to_str().unwrap()]);
    })
    .await?;
    let result = loader.load().await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert_eq!(
            err.to_string(),
            r#"testfixtures: Unknown column 'updated_at' in 'field list'"#
        );
    }
    Ok(())
}
