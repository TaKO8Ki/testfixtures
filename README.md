# testfixtures

[![build]](https://github.com/TaKO8Ki/testfixtures/actions) [![crates]](https://crates.io/crates/testfixtures) [![docs]](https://docs.rs/testfixtures) [![license]](https://github.com/TaKO8Ki/testfixtures/blob/master/LICENSE) [![downloads]](https://crates.io/crates/testfixtures)

[build]: https://img.shields.io/github/workflow/status/TaKO8Ki/testfixtures/CI/master?logo=github
[license]: https://img.shields.io/github/license/TaKO8Ki/testfixtures
[docs]: https://img.shields.io/badge/docs.rs-testfixtures-8da0cb?labelColor=555555&logo=rust
[downloads]: https://img.shields.io/crates/d/testfixtures
[crates]: https://img.shields.io/crates/v/testfixtures.svg?logo=rust

testfixtures is a Rust library for preparing test data from yaml files.

## Install

This crate is compatible with the async-std and tokio runtimes.

async-std

```toml
[dependencies]
testfixtures = "0.1"
sqlx = "0.3"
chrono = "0.4.11"
```

tokio

```toml
[dependencies]
testfixtures = { version = "0.1", default-features = false, features = [ "runtime-tokio" ] }
sqlx = { version = "0.3", default-features = false, features = [ "runtime-tokio", "macros" ] }
chrono = "0.4.11"
```

## Usage

Create fixture files like the following.

`todos.yml`
```yml
- id: 1
  description: buy a new camera
  done: true
  progress: 10.5
  created_at: 2020/01/01 01:01:01
- id: 2
  description: meeting
  done: false
  progress: 30.0
  created_at: 2020/01/01 02:02:02
```

<details><summary>Click and see the datetime format example</summary><div>

```rust
2020-01-01 01:01
2020-01-01 01:01:01
20200101 01:01
20200101 01:01:01
01012020 01:01
01012020 01:01:01
2020/01/01 01:01
2020/01/01 01:01:01
```
</div></details><br>

If you need to write raw SQL, probably to call a function, prefix the value of the column with RAW=.

```yml
- id: 1
  description: fizz
  done: true
  progress: 10.5
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
        loader.load().await.unwrap();

        // run your tests

        Ok(())
    }
}

```

**PgLoader** and **SqliteLoader** are under development.

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
    // or cfg.location(Local);
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
    cfg.files(vec!["fizz.yml"]);
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
    cfg.paths(vec!["fizz", "buzz/todos.yml"]);
    // ...
})
.await?;
```

## Implemation status
### Database
- [x] MySQL and MariaDB
- [ ] Postgres
- [ ] SQLite

### Options
- [x] database
- [x] load files
- [x] skip_test_database_check
- [x] location
- [x] directory
- [x] paths
- [ ] template

## Contribution

```sh
# setup test db
$ make db

# load environment variables
$ make env
$ direnv allow # https://github.com/direnv/direnv

# run unit tests
$ make test
```

## Reference
https://github.com/go-testfixtures/testfixtures
