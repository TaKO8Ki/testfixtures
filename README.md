# [WIP] testfixtures

## Dependencies

```toml
[dependencies]
testfixtures = "1.0"
sqlx = "0.3"
```

## Usage

Create fixture files. Each file should contain data for a single table and have the name <table_name>.yml:

```yml
# todos.yml
- id: 1
  description: buy a new camera
  done: 0
- id: 2
  description: meeting
  done: 0
```

Your tests would look like this.

```rust
use sqlx::MySqlPool;
use std::env;
use testfixtures::MySqlLoader;

#[async_std::main]
#[paw::main]
async fn test_function() -> anyhow::Result<()> {
    let pool = MySqlPool::new(&env::var("DATABASE_URL")?).await?;
    let loader = MySqlLoader::new(|cfg| {
        cfg.location("fehwo");
        cfg.database(pool);
        cfg.skip_test_database_check();
        cfg.paths(vec!["fixtures/todos.yml"]);
    })
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
- [x] MySQL and MariaDB
- [ ] Postgres
- [ ] SQLite

### Option
- [x] database
- [x] load files
- [x] skip_test_database_check
- [x] location
- [ ] load files from a directory
- [ ] yaml template

# Reference
https://github.com/go-testfixtures/testfixtures
