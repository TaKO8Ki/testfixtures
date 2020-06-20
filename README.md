# [WIP] testfixtures

![](https://img.shields.io/github/workflow/status/TaKO8Ki/testfixtures/CI/master) ![img](https://img.shields.io/github/license/TaKO8Ki/testfixtures)

## Dependencies

```toml
[dependencies]
testfixtures = "1.0"
sqlx = "0.3"
```

## Usage

Create fixture files. Each file should contain data for a certain table and have the name <table_name>.yml.

```yml
# todos.yml
- id: 1
  description: buy a new camera
  done: 0
- id: 2
  description: meeting
  done: 0
```

If you need to write raw SQL, probably to call a function, prefix the value of the column with RAW=.

```yml
- id: 1
  description: buy a new camera
  done: 0
  created_at: RAW=NOW()
```

Your tests would look like this.

```rust

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::MySqlPool;
    use std::env;
    use testfixtures::MySqlLoader;

    #[async_std::test]
    async fn test_something() -> anyhow::Result<()> {
        let pool = MySqlPool::new(&env::var("DATABASE_URL")?).await?;
        let loader = MySqlLoader::new(|cfg| {
            cfg.location(Utc);
            cfg.database(pool);
            cfg.skip_test_database_check();
            cfg.paths(vec!["fixtures/todos.yml"]);
        })
        .await?;

        // load your fixtures
        loader.load().await?;

        // run your tests

        Ok(())
    }
}

```
## Options

### database(required)
database is a option for passing db connection pool to a Loader.

```rust
let pool = MySqlPool::new(&env::var("DATABASE_URL")?).await?;
let loader = MySqlLoader::new(|cfg| {
    cfg.database(pool);
    // ...
})
.await?;
```

### location(required)
location is a option for setting timezone.

```rust
use chrono::Utc;

let loader = MySqlLoader::new(|cfg| {
    cfg.location(Utc);
    // ...
})
.await?;
```

### skip_test_database_check(optional)
skip_test_database_check is a option for setting a flag for checking if database name ends with "test".

```rust
let loader = MySqlLoader::new(|cfg| {
    cfg.skip_test_database_check();
    // ...
})
.await?;
```

### files(optional)
files is a option for reading your fixture files.

```rust
let loader = MySqlLoader::new(|cfg| {
    cfg.paths(vec!["fizz.yml"]);
    // ...
})
.await?;
```

### directory(optional)
files is a option for reading your fixture files in a directory.

```rust
let loader = MySqlLoader::new(|cfg| {
    cfg.directory("fixture");
    // ...
})
.await?;
```

### paths(optional)
paths is a option that is a combination of files option and directory option.

```rust
let loader = MySqlLoader::new(|cfg| {
    cfg.directory(vec!["fizz", "buzz/todos.yml"]);
    // ...
})
.await?;
```

## Contribute

```sh
# setup test db
$ make db

# load environment variables
$ make env
$ direnv allow # https://github.com/direnv/direnv

# run all tests
$ make test
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
- [x] directory
- [x] paths
- [ ] template


## Reference
https://github.com/go-testfixtures/testfixtures
