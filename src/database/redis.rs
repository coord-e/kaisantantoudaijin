use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;

use crate::database::{DatabaseHandle, DatabaseValue};

use futures::lock::Mutex;
use redis::{AsyncCommands, FromRedisValue, RedisResult, RedisWrite, ToRedisArgs};

#[repr(transparent)]
struct RedisDatabaseValue(DatabaseValue);

impl ToRedisArgs for RedisDatabaseValue {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: RedisWrite + ?Sized,
    {
        match &self.0 {
            DatabaseValue::String(s) => s.write_redis_args(out),
            DatabaseValue::U32(n) => n.write_redis_args(out),
        }
    }
}

impl FromRedisValue for RedisDatabaseValue {
    fn from_redis_value(v: &redis::Value) -> RedisResult<Self> {
        if let Ok(s) = String::from_redis_value(v) {
            Ok(RedisDatabaseValue(DatabaseValue::String(s)))
        } else {
            let n = u32::from_redis_value(v)?;
            Ok(RedisDatabaseValue(DatabaseValue::U32(n)))
        }
    }
}

#[derive(Debug, Clone)]
pub struct RedisHandle {
    prefix: String,
    guild_id: serenity::model::id::GuildId,
    conn: Arc<Mutex<deadpool_redis::Connection>>,
}

impl RedisHandle {
    pub fn new(
        prefix: String,
        guild_id: serenity::model::id::GuildId,
        conn: deadpool_redis::Connection,
    ) -> Self {
        Self {
            prefix,
            guild_id,
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    fn key(&self, key: &str) -> String {
        format!("{}:{}:{}", self.prefix, u64::from(self.guild_id), key)
    }
}

#[async_trait::async_trait]
impl DatabaseHandle for RedisHandle {
    type Error = redis::RedisError;

    async fn get<T: TryFrom<DatabaseValue>>(&self, key: &str) -> Result<Option<T>, Self::Error> {
        let value: Option<RedisDatabaseValue> = self.conn.lock().await.get(self.key(key)).await?;
        value
            .map(|v| {
                v.0.try_into().map_err(|_| {
                    redis::RedisError::from((redis::ErrorKind::TypeError, "unexpected type"))
                })
            })
            .transpose()
    }

    async fn set<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<(), Self::Error> {
        let value = value.into();
        self.conn
            .lock()
            .await
            .set(self.key(key), RedisDatabaseValue(value))
            .await
    }

    async fn set_members<T: Eq + Hash + TryFrom<DatabaseValue>>(
        &self,
        key: &str,
    ) -> Result<HashSet<T>, Self::Error> {
        let members: Vec<RedisDatabaseValue> =
            self.conn.lock().await.smembers(self.key(key)).await?;
        members
            .into_iter()
            .map(|v| {
                v.0.try_into().map_err(|_| {
                    redis::RedisError::from((redis::ErrorKind::TypeError, "unexpected type"))
                })
            })
            .collect()
    }

    async fn set_add<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool, Self::Error> {
        let value = value.into();
        let n: i32 = self
            .conn
            .lock()
            .await
            .sadd(self.key(key), RedisDatabaseValue(value))
            .await?;
        Ok(n != 0)
    }

    async fn set_remove<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool, Self::Error> {
        let value = value.into();
        let n: i32 = self
            .conn
            .lock()
            .await
            .srem(self.key(key), RedisDatabaseValue(value))
            .await?;
        Ok(n != 0)
    }

    async fn flag_get(&self, key: &str, default: bool) -> Result<bool, Self::Error> {
        Ok(match self.get::<u32>(key).await? {
            None => default,
            Some(r) => r != 0,
        })
    }

    async fn flag_set(&self, key: &str, flag: bool) -> Result<(), Self::Error> {
        self.set(key, flag as u32).await
    }
}
