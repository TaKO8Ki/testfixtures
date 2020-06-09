use async_trait::async_trait;
use sqlx::{Connect, Connection, Database as DB, Pool, Query};

#[async_trait]
pub trait Database<T, C>
where
	T: DB + Sync + Send,
	C: Connection<Database = T> + Connect<Database = T> + Sync + Send,
{
	async fn init(&mut self, db: &Pool<C>) -> anyhow::Result<()>;

	// TODO: complete this function
	// async fn database_name(&self, db: &Pool<C>) -> anyhow::Result<String>;

	// TODO: complete this function
	// async fn table_names(&self, db: &Pool<C>) -> anyhow::Result<Vec<String>>;

	async fn with_transaction<'a>(
		&self,
		pool: &Pool<C>,
		queries: Vec<Query<'a, T>>,
	) -> anyhow::Result<()>
	where
		T: DB + Sync + Send,
		C: Connection<Database = T> + Connect<Database = T>;
}
