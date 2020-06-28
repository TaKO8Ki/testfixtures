use sqlx::MySqlPool;
use std::env;

#[async_std::main]
#[paw::main]
async fn main() -> anyhow::Result<()> {
    let pool = MySqlPool::new(&env::var("DATABASE_URL")?).await?;
    println!("{}", list_todos(&pool).await?);
    Ok(())
}

async fn list_todos(pool: &MySqlPool) -> anyhow::Result<String> {
    let recs = sqlx::query!(
        r#"
SELECT id, description, done
FROM todos
ORDER BY id
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut todos = "".to_string();

    for rec in recs {
        todos = format!(
            "{}{}",
            todos,
            format!(
                "- [{}] {}: {}\n",
                if rec.done != 0 { "x" } else { " " },
                rec.id,
                &rec.description,
            )
        );
    }

    Ok(todos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sqlx::MySqlPool;
    use std::env;
    use testfixtures::MySqlLoader;

    #[async_std::test]
    async fn test_list_todos() -> anyhow::Result<()> {
        let pool = MySqlPool::new(&env::var("DATABASE_URL")?).await?;
        let pool_for_query = pool.clone();
        let loader = MySqlLoader::new(|cfg| {
            cfg.location(Utc);
            cfg.database(pool);
            cfg.skip_test_database_check();
            cfg.paths(vec!["fixtures/todos.yml"]);
        })
        .await?;

        // load your fixtures
        loader.load().await.unwrap();

        assert_eq!(
            list_todos(&pool_for_query).await?,
            "- [x] 1: fizz\n- [ ] 2: buzz\n"
        );

        Ok(())
    }
}
