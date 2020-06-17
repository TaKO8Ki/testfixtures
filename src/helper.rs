use crate::fixture_file::FixtureFile;
use async_trait::async_trait;
use chrono::{Offset, TimeZone};
use sqlx::{Connect, Connection, Database as DB, Pool};

#[async_trait]
pub trait Database<T, C, O, Tz>
where
	T: DB + Sync + Send,
	C: Connection<Database = T> + Connect<Database = T> + Sync + Send,
	O: Offset + Sync + Send,
	Tz: TimeZone<Offset = O> + Send + Sync,
{
	async fn init(&mut self, db: &Pool<C>) -> anyhow::Result<()>;

	async fn database_name(&self, db: &Pool<C>) -> anyhow::Result<String>;

	// TODO: complete this function
	// async fn table_names(&self, db: &Pool<C>) -> anyhow::Result<Vec<String>>;

	async fn with_transaction<'a>(
		&self,
		pool: &Pool<C>,
		fixture_files: &[FixtureFile<Tz>],
	) -> anyhow::Result<()>;
}
