# testfixtures

## [WIP] Usage

Your tests would look like this:

```rust
use sqlx::MySqlPool;
use std::env;

#[async_std::main]
#[paw::main]
async fn test_function() -> anyhow::Result<()> {
    let pool = MySqlPool::new(&env::var("DATABASE_URL")?).await?;

    let loader = testfixtures::Loader::new(vec![
        testfixtures::Loader::database(pool),
        testfixtures::Loader::dialect("mysql"),
        testfixtures::Loader::files(vec!["todos.yml"]),
        testfixtures::Loader::skip_test_database_check(),
    ])
    .await?;

    // load your fixtures
    loader.load().await?;

    // run your tests

    Ok(())
}

```

# Reference
https://github.com/go-testfixtures/testfixtures
