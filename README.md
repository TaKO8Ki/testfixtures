# testfixtures

## [WIP] Usage

Your tests would look like this.

```rust
use sqlx::MySqlPool;
use std::env;
use testfixtures::MySqlLoader;

#[async_std::main]
#[paw::main]
async fn test_function() -> anyhow::Result<()> {
    let pool = MySqlPool::new(&env::var("DATABASE_URL")?).await?;
    let loader = MySqlLoader::new(vec![
        MySqlLoader::database(pool),
        MySqlLoader::files(vec!["todos.yml"]),
        MySqlLoader::skip_test_database_check(),
    ])
    .await?;

    // load your fixtures
    loader.load().await?;

    // run your tests
    test_something();

    Ok(())
}

```

## Implemation status
### Database
- [x] mysql
- [ ] postgres
- [ ] sqlite

### Option
- [x] database
- [x] load files
- [ ] load files from a directory
- [ ] yaml template

# Reference
https://github.com/go-testfixtures/testfixtures
