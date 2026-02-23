use sled::{Db, Tree};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::{DateTime, Utc, Duration};
use tokio::task::JoinHandle;
use thiserror::Error;

/// 自定义错误类型，实现 Send
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Sled error: {0}")]
    Sled(#[from] sled::Error),
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Tokio join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("UTF8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Date parse error: {0}")]
    DateParse(#[from] chrono::ParseError),
}

/// 数据库结构，存储周数和最后访问时间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekData {
    pub week_count: u64,
    pub last_click_time: Option<DateTime<Utc>>,
}

/// 异步数据库管理器
/// 使用 tokio 任务将阻塞的数据库操作移到后台线程池
pub struct Database {
    db: Arc<Db>,
    week_tree: Arc<Tree>,
    click_tree: Arc<Tree>,
}

impl Database {
    /// 创建新的数据库实例
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db = sled::open(path)?;
        let week_tree = db.open_tree("weeks")?;
        let click_tree = db.open_tree("clicks")?;

        Ok(Database {
            db: Arc::new(db),
            week_tree: Arc::new(week_tree),
            click_tree: Arc::new(click_tree),
        })
    }

    /// 异步获取当前周数
    pub async fn get_week_count(&self) -> Result<u64, DbError> {
        let week_tree = self.week_tree.clone();
        tokio::task::spawn_blocking(move || {
            let key = b"current_week";
            if let Some(value) = week_tree.get(key)? {
                let data: WeekData = bincode::deserialize(&value)?;
                Ok(data.week_count)
            } else {
                Ok(0)
            }
        })
        .await?
    }

    /// 异步增加周数（带 IP 检查，用于首页访问）
    /// 使用事务确保原子性，防止并发情况下的数据竞争
    pub async fn increment_week_with_ip_check(&self, ip: String) -> Result<bool, DbError> {
        let week_tree = self.week_tree.clone();
        let click_tree = self.click_tree.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || {
            let week_key = b"current_week";
            let ip_key = format!("ip:{}", ip);

            // 先检查 IP 是否在当天已经访问过（这个检查不需要在事务中）
            let ip_bytes = ip_key.as_bytes();
            if let Some(prev_click_bytes) = click_tree.get(ip_bytes)? {
                let prev_click_str = std::str::from_utf8(&prev_click_bytes)?;
                let prev_click = DateTime::parse_from_rfc3339(prev_click_str)?;
                let prev_date = prev_click.date_naive();
                let current_date = now.date_naive();

                if prev_date == current_date {
                    // 同一天内已经访问过
                    return Ok(false);
                }
            }

            // 使用事务更新周数（确保并发安全）
            let new_week_count = week_tree.transaction(|tree| {
                // 获取当前数据
                let mut data: WeekData = if let Some(value) = tree.get(week_key)? {
                    bincode::deserialize(&value).map_err(|e| {
                        sled::transaction::ConflictableTransactionError::Abort(
                            sled::Error::Unsupported(e.to_string())
                        )
                    })?
                } else {
                    WeekData {
                        week_count: 0,
                        last_click_time: None,
                    }
                };

                // 增加周数
                data.week_count += 1;
                data.last_click_time = Some(now);

                // 保存
                let serialized = bincode::serialize(&data).map_err(|e| {
                    sled::transaction::ConflictableTransactionError::Abort(
                        sled::Error::Unsupported(e.to_string())
                    )
                })?;
                tree.insert(week_key, serialized)?;

                Ok(data.week_count)
            })
            .map_err(|e| match e {
                sled::transaction::TransactionError::Abort(err) => {
                    DbError::Sled(err)
                }
                sled::transaction::TransactionError::Storage(err) => {
                    DbError::Sled(err)
                }
            })?;

            // 记录 IP 访问时间（在事务成功后）
            click_tree.insert(ip_bytes, now.to_rfc3339().as_bytes())?;

            Ok(new_week_count > 0)
        })
        .await?
    }

    /// 异步增加周数（无 IP 检查，用于按钮点击）
    /// 使用事务确保原子性，防止并发情况下的数据竞争
    pub async fn increment_week(&self) -> Result<u64, DbError> {
        let week_tree = self.week_tree.clone();

        tokio::task::spawn_blocking(move || {
            let key = b"current_week";

            // 使用事务确保原子性
            // Sled 事务会自动重试，直到成功或达到最大重试次数
            week_tree.transaction(|tree| {
                // 获取当前数据
                let mut data: WeekData = if let Some(value) = tree.get(key)? {
                    // 手动反序列化以处理事务中的错误
                    bincode::deserialize(&value).map_err(|e| {
                        sled::transaction::ConflictableTransactionError::Abort(
                            sled::Error::Unsupported(e.to_string())
                        )
                    })?
                } else {
                    WeekData {
                        week_count: 0,
                        last_click_time: None,
                    }
                };

                // 直接增加周数，不检查 IP
                data.week_count += 1;

                // 保存到数据库（事务的一部分）
                let serialized = bincode::serialize(&data).map_err(|e| {
                    sled::transaction::ConflictableTransactionError::Abort(
                        sled::Error::Unsupported(e.to_string())
                    )
                })?;
                tree.insert(key, serialized)?;

                // 返回新的周数
                Ok(data.week_count)
            })
            .map_err(|e| match e {
                sled::transaction::TransactionError::Abort(err) => {
                    // 事务被中止，转换错误类型
                    DbError::Sled(err)
                }
                sled::transaction::TransactionError::Storage(err) => {
                    // 存储错误，转换错误类型
                    DbError::Sled(err)
                }
            })
        })
        .await?
    }

    /// 异步获取完整的周数据信息
    pub async fn get_week_data(&self) -> Result<WeekData, DbError> {
        let week_tree = self.week_tree.clone();
        tokio::task::spawn_blocking(move || {
            let key = b"current_week";
            if let Some(value) = week_tree.get(key)? {
                let data: WeekData = bincode::deserialize(&value)?;
                Ok(data)
            } else {
                Ok(WeekData {
                    week_count: 0,
                    last_click_time: None,
                })
            }
        })
        .await?
    }

    /// 异步重置周数（用于测试或管理）
    pub async fn reset_weeks(&self) -> Result<(), DbError> {
        let week_tree = self.week_tree.clone();
        tokio::task::spawn_blocking(move || {
            let key = b"current_week";
            week_tree.remove(key)?;
            let _ = week_tree.flush()?;
            Ok(())
        })
        .await?
    }

    /// 异步刷新数据库到磁盘
    pub fn flush_async(&self) -> JoinHandle<Result<(), DbError>> {
        let week_tree = self.week_tree.clone();
        tokio::task::spawn_blocking(move || {
            week_tree.flush()?;
            Ok(())
        })
    }
}

// 计算从预设时间开始的日期
pub fn calculate_date_from_weeks(weeks: u64) -> DateTime<Utc> {
    // 预设的起始时间：2024-01-01 00:00:00 UTC
    let base_time = DateTime::<Utc>::from_timestamp(1704067200, 0).unwrap();

    // 计算目标时间（处理溢出）
    let weeks_duration = Duration::weeks(weeks as i64);
    base_time + weeks_duration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_date() {
        let date = calculate_date_from_weeks(0);
        assert_eq!(date.timestamp(), 1704067200); // 2024-01-01 00:00:00 UTC
    }

    #[test]
    fn test_calculate_date_one_week() {
        let date = calculate_date_from_weeks(1);
        assert_eq!(date.timestamp(), 1704067200 + 7 * 24 * 60 * 60);
    }
}
