use async_trait::async_trait;
use sqlx::MySqlPool;

#[async_trait]
pub trait Database {
	async fn init(&mut self, db: &MySqlPool) -> anyhow::Result<()>;
	async fn table_names(&self, db: &MySqlPool) -> anyhow::Result<Vec<String>>;
}
