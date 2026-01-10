use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::database::{DatabaseHandle, DatabaseValue};

#[derive(Debug, Clone)]
pub struct DynamoDbHandle {
    client: aws_sdk_dynamodb::Client,
    guild_id: serenity::model::id::GuildId,
    table_name: String,
}

impl DynamoDbHandle {
    pub fn new(
        client: aws_sdk_dynamodb::Client,
        guild_id: serenity::model::id::GuildId,
        table_name: String,
    ) -> Self {
        Self {
            client,
            guild_id,
            table_name,
        }
    }

    fn key(&self) -> aws_sdk_dynamodb::types::AttributeValue {
        aws_sdk_dynamodb::types::AttributeValue::S(format!("Guild#{}", u64::from(self.guild_id)))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DynamoDbHandleError {
    // boxed to reduce size of the enum
    #[error("DynamoDB error: {0}")]
    DynamoDbError(Box<aws_sdk_dynamodb::Error>),
    #[error("unexpected attribute type for attribute {attribute}")]
    UnexpectedAttributeType { attribute: String },
    #[error("malformed attribute value for attribute {attribute}")]
    MalformedAttributeValue { attribute: String },
}

impl<E> From<E> for DynamoDbHandleError
where
    E: Into<aws_sdk_dynamodb::Error>,
{
    fn from(err: E) -> Self {
        DynamoDbHandleError::DynamoDbError(Box::new(err.into()))
    }
}

fn extract_attribute<T: TryFrom<DatabaseValue>>(
    item: &HashMap<String, aws_sdk_dynamodb::types::AttributeValue>,
    key: &str,
) -> Result<Option<T>, DynamoDbHandleError> {
    let Some(attr) = item.get(key) else {
        return Ok(None);
    };
    let value = match attr {
        aws_sdk_dynamodb::types::AttributeValue::S(s) => DatabaseValue::String(s.clone()),
        aws_sdk_dynamodb::types::AttributeValue::N(n) => {
            let n = n
                .parse()
                .map_err(|_| DynamoDbHandleError::MalformedAttributeValue {
                    attribute: key.to_string(),
                })?;
            DatabaseValue::U32(n)
        }
        _ => {
            return Err(DynamoDbHandleError::UnexpectedAttributeType {
                attribute: key.to_string(),
            })
        }
    };
    let value = T::try_from(value).map_err(|_| DynamoDbHandleError::UnexpectedAttributeType {
        attribute: key.to_string(),
    })?;
    Ok(Some(value))
}

fn contains_in_set(
    item: &HashMap<String, aws_sdk_dynamodb::types::AttributeValue>,
    key: &str,
    value: &DatabaseValue,
) -> Result<bool, DynamoDbHandleError> {
    let Some(attr) = item.get(key) else {
        return Ok(false);
    };
    match (attr, value) {
        (aws_sdk_dynamodb::types::AttributeValue::Ns(ns), DatabaseValue::U32(n)) => {
            Ok(ns.iter().any(|num| num.parse() == Ok(*n)))
        }
        (aws_sdk_dynamodb::types::AttributeValue::Ss(ss), DatabaseValue::String(s)) => {
            Ok(ss.iter().any(|str| str == s))
        }
        _ => Err(DynamoDbHandleError::UnexpectedAttributeType {
            attribute: key.to_string(),
        }),
    }
}

fn database_value_to_attribute_value(
    value: DatabaseValue,
) -> aws_sdk_dynamodb::types::AttributeValue {
    match value {
        DatabaseValue::String(s) => aws_sdk_dynamodb::types::AttributeValue::S(s),
        DatabaseValue::U32(n) => aws_sdk_dynamodb::types::AttributeValue::N(n.to_string()),
    }
}

fn database_value_to_set_attribute_value(
    value: DatabaseValue,
) -> aws_sdk_dynamodb::types::AttributeValue {
    match value {
        DatabaseValue::String(s) => aws_sdk_dynamodb::types::AttributeValue::Ss(vec![s]),
        DatabaseValue::U32(n) => aws_sdk_dynamodb::types::AttributeValue::Ns(vec![n.to_string()]),
    }
}

#[async_trait::async_trait]
impl DatabaseHandle for DynamoDbHandle {
    type Error = DynamoDbHandleError;

    async fn get<T: TryFrom<DatabaseValue>>(&self, key: &str) -> Result<Option<T>, Self::Error> {
        let resp = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", self.key())
            .projection_expression("#attr")
            .expression_attribute_names("#attr", key)
            .send()
            .await?;
        let Some(item) = resp.item else {
            return Ok(None);
        };
        extract_attribute(&item, key)
    }

    async fn set<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<(), Self::Error> {
        let value = database_value_to_attribute_value(value.into());
        self.client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", self.key())
            .update_expression("SET #attr = :val")
            .expression_attribute_names("#attr", key)
            .expression_attribute_values(":val", value)
            .send()
            .await?;
        Ok(())
    }

    async fn set_members<T: Eq + Hash + TryFrom<DatabaseValue>>(
        &self,
        key: &str,
    ) -> Result<HashSet<T>, Self::Error> {
        let resp = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", self.key())
            .projection_expression("#attr")
            .expression_attribute_names("#attr", key)
            .send()
            .await?;
        let Some(item) = resp.item else {
            return Ok(HashSet::new());
        };
        let Some(attr) = item.get(key) else {
            return Ok(HashSet::new());
        };
        let values = match attr {
            aws_sdk_dynamodb::types::AttributeValue::Ns(ns) => ns
                .iter()
                .map(|n| {
                    let n =
                        n.parse()
                            .map_err(|_| DynamoDbHandleError::MalformedAttributeValue {
                                attribute: key.to_string(),
                            })?;
                    let db_value = DatabaseValue::U32(n);
                    T::try_from(db_value).map_err(|_| {
                        DynamoDbHandleError::UnexpectedAttributeType {
                            attribute: key.to_string(),
                        }
                    })
                })
                .collect::<Result<HashSet<T>, DynamoDbHandleError>>()?,
            aws_sdk_dynamodb::types::AttributeValue::Ss(ss) => ss
                .iter()
                .map(|s| {
                    let db_value = DatabaseValue::String(s.clone());
                    T::try_from(db_value).map_err(|_| {
                        DynamoDbHandleError::UnexpectedAttributeType {
                            attribute: key.to_string(),
                        }
                    })
                })
                .collect::<Result<HashSet<T>, DynamoDbHandleError>>()?,
            _ => {
                return Err(DynamoDbHandleError::UnexpectedAttributeType {
                    attribute: key.to_string(),
                });
            }
        };
        Ok(values)
    }

    async fn set_add<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool, Self::Error> {
        let value = value.into();
        let set_value = database_value_to_set_attribute_value(value.clone());
        let resp = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", self.key())
            .update_expression("ADD #attr :val")
            .expression_attribute_names("#attr", key)
            .expression_attribute_values(":val", set_value)
            .return_values(aws_sdk_dynamodb::types::ReturnValue::UpdatedOld)
            .send()
            .await?;
        if let Some(attributes) = &resp.attributes {
            contains_in_set(attributes, key, &value).map(|exists| !exists)
        } else {
            Ok(true)
        }
    }

    async fn set_remove<T: Into<DatabaseValue> + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) -> Result<bool, Self::Error> {
        let value = value.into();
        let set_value = database_value_to_set_attribute_value(value.clone());
        let resp = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", self.key())
            .update_expression("DELETE #attr :val")
            .expression_attribute_names("#attr", key)
            .expression_attribute_values(":val", set_value)
            .return_values(aws_sdk_dynamodb::types::ReturnValue::UpdatedOld)
            .send()
            .await?;
        if let Some(attributes) = &resp.attributes {
            contains_in_set(attributes, key, &value)
        } else {
            Ok(false)
        }
    }

    async fn flag_get(&self, key: &str, default: bool) -> Result<bool, Self::Error> {
        let resp = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", self.key())
            .projection_expression("#attr")
            .expression_attribute_names("#attr", key)
            .send()
            .await?;
        let Some(item) = resp.item else {
            return Ok(default);
        };
        let Some(attr) = item.get(key) else {
            return Ok(default);
        };
        attr.as_bool()
            .cloned()
            .map_err(|_| DynamoDbHandleError::UnexpectedAttributeType {
                attribute: key.to_string(),
            })
    }

    async fn flag_set(&self, key: &str, flag: bool) -> Result<(), Self::Error> {
        self.client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", self.key())
            .update_expression("SET #attr = :val")
            .expression_attribute_names("#attr", key)
            .expression_attribute_values(
                ":val",
                aws_sdk_dynamodb::types::AttributeValue::Bool(flag),
            )
            .send()
            .await?;
        Ok(())
    }
}
