use async_trait::async_trait;
use sqlx::MySqlPool;

#[async_trait]
pub trait Database {
	async fn init(&mut self, db: &MySqlPool) -> anyhow::Result<()>;
	async fn database_name(&self, db: &MySqlPool) -> anyhow::Result<String>;
	async fn table_names(&self, db: &MySqlPool) -> anyhow::Result<Vec<String>>;
}
