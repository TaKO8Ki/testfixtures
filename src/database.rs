use async_trait::async_trait;
use sqlx::{Connection, Database, Execute, Pool, Transaction};
use std::future::Future;
use std::pin::Pin;

#[async_trait]
pub trait DB<T, C>
where
	T: Database + Sync + Send,
	C: Connection<Database = T> + Sync + Send,
{
	async fn init(&mut self, db: &Pool<C>) -> anyhow::Result<()>;
	async fn database_name(&self, db: &Pool<C>) -> anyhow::Result<String>;
	async fn table_names(&self, db: &Pool<C>) -> anyhow::Result<Vec<String>>;
	// async fn with_transaction(
	// 	&self,
	// 	db: &C,
	// 	f: Box<
	// 		dyn FnOnce(
	// 				&mut Transaction<C>,
	// 			) -> Pin<Box<(dyn Future<Output = anyhow::Result<()>> + Send)>>
	// 			+ Send,
	// 	>,
	// ) -> anyhow::Result<()>;
}
