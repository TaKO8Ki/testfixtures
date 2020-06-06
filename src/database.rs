use async_trait::async_trait;
use sqlx::{Connect, Connection, Database, Pool, Query};

#[async_trait]
pub trait DB<T, C>
where
	T: Database + Sync + Send,
	C: Connection<Database = T> + Sync + Send,
{
	async fn init(&mut self, db: &Pool<C>) -> anyhow::Result<()>;
	async fn database_name(&self, db: &Pool<C>) -> anyhow::Result<String>;
	async fn table_names(&self, db: &Pool<C>) -> anyhow::Result<Vec<String>>;
	async fn with_transaction<'a>(
		&self,
		pool: &Pool<C>,
		queries: Vec<Query<'a, T>>,
	) -> anyhow::Result<()>
	where
		T: Database + Sync + Send,
		C: Connection<Database = T> + Connect<Database = T>;
}
