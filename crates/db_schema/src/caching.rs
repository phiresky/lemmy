use crate::{traits::UncachedCrud, utils::DbPool};
use diesel::result::Error;
use moka::future::Cache;
use std::{
  hash::Hash,
  sync::{Arc, OnceLock},
  time::Duration,
};
use tokio::sync::RwLock;
use type_map::concurrent::TypeMap;

/// get a cache map for a specific object kind
/// creates a new cache map if not already exists
async fn get_cache_for_type<Key, Value>() -> Cache<Key, Value>
where
  Key: Send + Sync + Clone + Eq + Hash + 'static,
  Value: Send + Sync + Clone + 'static,
{
  static CACHES: OnceLock<RwLock<type_map::concurrent::TypeMap>> = OnceLock::new();
  let caches = CACHES.get_or_init(|| RwLock::new(TypeMap::new()));
  // in most cases (except the first call) we only need a read-only lock, do hot path separately for perf
  let existing_cache = {
    caches
      .read()
      .await
      .get::<Cache<Key, Value>>()
      .map(|e| e.clone())
  };
  match existing_cache {
    Some(cache) => cache,
    None => {
      let mut caches = caches.write().await;
      caches
        .entry()
        .or_insert_with(|| {
          Cache::builder()
            .max_capacity(10000) //Value::cache_max_capacity())
            .time_to_live(Duration::from_secs(60)) //Value::cache_time_to_live())
            .build()
        })
        .clone()
    }
  }
}

#[async_trait]
pub trait CachingCrud: Sized {
  type InsertForm;
  type UpdateForm;
  type IdType: Send;

  async fn read(pool: &DbPool, id: Self::IdType) -> Result<Self, Error>;
  async fn create(pool: &DbPool, form: &Self::InsertForm) -> Result<Self, Error> {
    todo!();
  }

  async fn update(pool: &DbPool, id: Self::IdType, form: &Self::UpdateForm) -> Result<Self, Error> {
    todo!();
  }
  async fn delete(_pool: &DbPool, _id: Self::IdType) -> Result<usize, Error> {
    todo!();
  }
}

#[async_trait]
impl<T: UncachedCrud + Send + Sync + Clone + 'static> CachingCrud for T {
  type InsertForm = T::InsertForm;
  type UpdateForm = T::UpdateForm;
  type IdType = T::IdType;

  async fn read(pool: &DbPool, id: Self::IdType) -> Result<Self, Error> {
    let cache = get_cache_for_type::<Self::IdType, Self>().await;
    cache
      .try_get_with(id.clone(), async move { Self::read_uncached(pool, id).await })
      .await
      .map_err(|e| /* todo: race condition */ Arc::try_unwrap(e).unwrap_or_else(|e| Error::SerializationError(Box::new(e))))
  }

  async fn create(pool: &DbPool, form: &Self::InsertForm) -> Result<Self, Error> {
    todo!();
  }

  async fn update(pool: &DbPool, id: Self::IdType, form: &Self::UpdateForm) -> Result<Self, Error> {
    todo!();
  }
  async fn delete(_pool: &DbPool, _id: Self::IdType) -> Result<usize, Error> {
    todo!();
  }
}
