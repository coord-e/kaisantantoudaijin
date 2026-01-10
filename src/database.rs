use std::collections::HashSet;
use std::hash::Hash;

mod redis;
pub use redis::RedisHandle;

mod dynamodb;
pub use dynamodb::DynamoDbHandle;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DatabaseValue {
    String(String),
    U32(u32),
}

impl<'a> From<&'a str> for DatabaseValue {
    fn from(value: &'a str) -> Self {
        DatabaseValue::String(value.to_owned())
    }
}

impl From<String> for DatabaseValue {
    fn from(value: String) -> Self {
        DatabaseValue::String(value)
    }
}

impl From<u32> for DatabaseValue {
    fn from(value: u32) -> Self {
        DatabaseValue::U32(value)
    }
}

impl TryFrom<DatabaseValue> for String {
    type Error = ();

    fn try_from(value: DatabaseValue) -> Result<Self, Self::Error> {
        match value {
            DatabaseValue::String(s) => Ok(s),
            _ => Err(()),
        }
    }
}

impl TryFrom<DatabaseValue> for u32 {
    type Error = ();

    fn try_from(value: DatabaseValue) -> Result<Self, Self::Error> {
        match value {
            DatabaseValue::U32(n) => Ok(n),
            _ => Err(()),
        }
    }
}

#[async_trait::async_trait]
pub trait DatabaseHandle {
    type Error: std::error::Error;

    async fn get<T: TryFrom<DatabaseValue>>(&self, key: &str) -> Result<Option<T>, Self::Error>;
    async fn set<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<(), Self::Error>;
    async fn set_members<T: Eq + Hash + TryFrom<DatabaseValue>>(
        &self,
        key: &str,
    ) -> Result<HashSet<T>, Self::Error>;
    async fn set_add<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool, Self::Error>;
    async fn set_remove<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool, Self::Error>;
    async fn flag_get(&self, key: &str, default: bool) -> Result<bool, Self::Error>;
    async fn flag_set(&self, key: &str, flag: bool) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone)]
pub enum AnyDatabaseHandle {
    Redis(RedisHandle),
    DynamoDb(DynamoDbHandle),
}

impl From<RedisHandle> for AnyDatabaseHandle {
    fn from(handle: RedisHandle) -> Self {
        AnyDatabaseHandle::Redis(handle)
    }
}

impl From<DynamoDbHandle> for AnyDatabaseHandle {
    fn from(handle: DynamoDbHandle) -> Self {
        AnyDatabaseHandle::DynamoDb(handle)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AnyDatabaseHandleError {
    #[error("{0}")]
    RedisError(#[from] <RedisHandle as DatabaseHandle>::Error),
    #[error("{0}")]
    DynamoDbError(#[from] <DynamoDbHandle as DatabaseHandle>::Error),
}

#[async_trait::async_trait]
impl DatabaseHandle for AnyDatabaseHandle {
    type Error = AnyDatabaseHandleError;

    async fn get<T: TryFrom<DatabaseValue>>(&self, key: &str) -> Result<Option<T>, Self::Error> {
        match self {
            AnyDatabaseHandle::Redis(handle) => handle.get(key).await.map_err(Into::into),
            AnyDatabaseHandle::DynamoDb(handle) => handle.get(key).await.map_err(Into::into),
        }
    }

    async fn set<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<(), Self::Error> {
        match self {
            AnyDatabaseHandle::Redis(handle) => handle.set(key, value).await.map_err(Into::into),
            AnyDatabaseHandle::DynamoDb(handle) => handle.set(key, value).await.map_err(Into::into),
        }
    }

    async fn set_members<T: Eq + Hash + TryFrom<DatabaseValue>>(
        &self,
        key: &str,
    ) -> Result<HashSet<T>, Self::Error> {
        match self {
            AnyDatabaseHandle::Redis(handle) => handle.set_members(key).await.map_err(Into::into),
            AnyDatabaseHandle::DynamoDb(handle) => {
                handle.set_members(key).await.map_err(Into::into)
            }
        }
    }

    async fn set_add<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool, Self::Error> {
        match self {
            AnyDatabaseHandle::Redis(handle) => {
                handle.set_add(key, value).await.map_err(Into::into)
            }
            AnyDatabaseHandle::DynamoDb(handle) => {
                handle.set_add(key, value).await.map_err(Into::into)
            }
        }
    }

    async fn set_remove<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool, Self::Error> {
        match self {
            AnyDatabaseHandle::Redis(handle) => {
                handle.set_remove(key, value).await.map_err(Into::into)
            }
            AnyDatabaseHandle::DynamoDb(handle) => {
                handle.set_remove(key, value).await.map_err(Into::into)
            }
        }
    }

    async fn flag_get(&self, key: &str, default: bool) -> Result<bool, Self::Error> {
        match self {
            AnyDatabaseHandle::Redis(handle) => {
                handle.flag_get(key, default).await.map_err(Into::into)
            }
            AnyDatabaseHandle::DynamoDb(handle) => {
                handle.flag_get(key, default).await.map_err(Into::into)
            }
        }
    }

    async fn flag_set(&self, key: &str, flag: bool) -> Result<(), Self::Error> {
        match self {
            AnyDatabaseHandle::Redis(handle) => {
                handle.flag_set(key, flag).await.map_err(Into::into)
            }
            AnyDatabaseHandle::DynamoDb(handle) => {
                handle.flag_set(key, flag).await.map_err(Into::into)
            }
        }
    }
}
