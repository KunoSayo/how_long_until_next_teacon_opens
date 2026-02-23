# 并发安全与数据一致性说明

## 概述

本应用使用 Sled 数据库的**事务机制**来确保在高并发场景下的数据一致性，防止数据竞争。

## 核心问题：竞态条件

### 问题场景

当多个用户同时点击"增加一周"按钮时，可能出现以下竞态条件：

```
时间线:
T1: 请求A 读取 week_count = 10
T2: 请求B 读取 week_count = 10  ← 在 A 写入前读取
T3: 请求A 计算 10 + 1 = 11，写入数据库
T4: 请求B 计算 10 + 1 = 11，写入数据库  ← 覆盖了 A 的结果

结果：周数应该是 12，但实际是 11（丢失了一次增加）
```

## 解决方案：Sled 事务

### 什么是事务？

事务（Transaction）是一组数据库操作，要么全部成功，要么全部失败。Sled 事务提供：
- **原子性**（Atomicity）：操作不可分割
- **一致性**（Consistency）：数据始终处于有效状态
- **隔离性**（Isolation）：并发事务互不干扰
- **持久性**（Durability）：提交后永久保存

### 实现细节

#### 1. `increment_week()` 方法（按钮点击）

```rust
pub async fn increment_week(&self) -> Result<u64, DbError> {
    let week_tree = self.week_tree.clone();

    tokio::task::spawn_blocking(move || {
        let key = b"current_week";

        // 使用事务确保原子性
        week_tree.transaction(|tree| {
            // 1. 读取当前值
            let mut data: WeekData = if let Some(value) = tree.get(key)? {
                bincode::deserialize(&value)?
            } else {
                WeekData {
                    week_count: 0,
                    last_click_time: None,
                }
            };

            // 2. 修改值
            data.week_count += 1;

            // 3. 写回数据库
            let serialized = bincode::serialize(&data)?;
            tree.insert(key, serialized)?;

            Ok(data.week_count)
        })
    })
    .await?
}
```

#### 2. `increment_week_with_ip_check()` 方法（首页访问）

```rust
pub async fn increment_week_with_ip_check(&self, ip: String) -> Result<bool, DbError> {
    // 1. 先检查 IP 是否在当天已访问（不需要事务）
    let ip_key = format!("ip:{}", ip);
    if let Some(prev_click_bytes) = click_tree.get(ip_key.as_bytes())? {
        let prev_click = DateTime::parse_from_rfc3339(/*...*/)?;
        if prev_click.date_naive() == now.date_naive() {
            return Ok(false);  // 同一天已访问
        }
    }

    // 2. 使用事务更新周数
    let new_week_count = week_tree.transaction(|tree| {
        // ... 事务逻辑
    })?;

    // 3. 记录 IP 访问（事务成功后）
    click_tree.insert(ip_key.as_bytes(), now.to_rfc3339().as_bytes())?;

    Ok(true)
}
```

## Sled 事务的工作原理

### 1. 乐观并发控制（OCC）

Sled 使用**乐观并发控制**，不使用锁：
- 事务执行时不阻塞其他事务
- 提交时检查是否有冲突
- 有冲突则自动重试

### 2. 冲突检测与重试

```
请求A:                  请求B:
  读取 x=10
  计算 x=11
                         读取 x=10
                         计算 x=11
  尝试写入 x=11 (成功)
                         尝试写入 x=11 (冲突！)
                         重试:
                           读取 x=11
                           计算 x=12
                           写入 x=12 (成功)
```

### 3. 事务 API 的特点

```rust
week_tree.transaction(|tree| {
    // 在这个闭包中：
    // 1. 所有操作都是原子的
    // 2. 如果检测到冲突，会自动重试
    // 3. 返回 Ok 表示提交
    // 4. 返回 Err 表示中止
    Ok(result)
})
```

## 错误处理

### 事务错误类型

```rust
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
```

### 事务内部的错误转换

事务闭包需要特殊错误处理：

```rust
bincode::deserialize(&value).map_err(|e| {
    sled::transaction::ConflictableTransactionError::Abort(
        sled::Error::Unsupported(e.to_string())
    )
})?
```

## 性能考虑

### 1. 事务开销

- **读操作**：几乎没有开销
- **写操作**：需要检测冲突，有少量开销
- **重试**：冲突时会重试，极端情况下可能多次重试

### 2. 异步处理

所有数据库操作都在 `spawn_blocking` 中执行，不阻塞异步运行时：

```rust
tokio::task::spawn_blocking(move || {
    // 数据库操作在专用线程池中执行
    week_tree.transaction(/* ... */)
}).await?
```

### 3. 性能优化建议

- **批量操作**：如果可能，将多个操作放在一个事务中
- **避免长事务**：事务内不要执行耗时操作
- **监控重试**：如果重试次数过多，考虑优化并发策略

## 测试并发安全

### 压力测试脚本

```bash
# 使用 Apache Bench
ab -n 1000 -c 100 -p /dev/null -T application/json \
   http://localhost:8080/api/increment

# 使用 wrk
wrk -t12 -c400 -d30s \
   -s post.lua \
   http://localhost:8080/api/increment
```

### 验证数据一致性

```bash
# 1. 记录初始值
curl http://localhost:8080/api/data

# 2. 发送 N 个并发请求
for i in {1..100}; do
    curl -X POST http://localhost:8080/api/increment &
done
wait

# 3. 检查最终值
curl http://localhost:8080/api/data

# 4. 验证：最终值应该 = 初始值 + 100
```

## 大数处理（前端）

### 问题

JavaScript 的 `Number` 类型是 64 位浮点数，整数精度只有 53 位（`Number.MAX_SAFE_INTEGER` = 2^53 - 1）。

当周数很大时，`weekCount * 7 * 24 * 60 * 60` 可能超过安全整数范围，导致精度丢失。

### 解决方案：使用 BigInt

```javascript
function updateDisplay(weekCount) {
    // 使用 BigInt 处理大数
    const baseTimestamp = 0n;
    const secondsPerWeek = 604800n;  // 7 * 24 * 60 * 60
    const weekCountBigInt = BigInt(weekCount);

    const weeksInSeconds = weekCountBigInt * secondsPerWeek;
    let targetTimestampMs = (baseTimestamp + weeksInSeconds) * 1000n;

    // 处理溢出：取模运算
    const cycleMs = 1000n * 60n * 60n * 24n * 365n * 10000n;  // 10000 年
    targetTimestampMs = ((targetTimestampMs % cycleMs) + cycleMs) % cycleMs;

    // 转换为 Date（此时已确保在安全范围内）
    const targetDate = new Date(Number(targetTimestampMs));
}
```

### BigInt 的优势

- **任意精度**：不受 53 位限制
- **精确计算**：不会丢失精度
- **原生支持**：现代浏览器都支持

## 最佳实践

### 1. 始终使用事务处理写操作

```rust
// ❌ 错误：非原子操作
let count = get_count()?;
let new_count = count + 1;
set_count(new_count)?;

// ✅ 正确：原子操作
tree.transaction(|tree| {
    let count = get_count(tree)?;
    let new_count = count + 1;
    set_count(tree, new_count)?;
    Ok(())
})?;
```

### 2. 避免在事务中执行耗时操作

```rust
// ❌ 错误：事务中执行网络请求
tree.transaction(|tree| {
    let count = get_count(tree)?;
    let response = reqwest::get("https://api.example.com")?;  // 不要！
    Ok(())
})?;

// ✅ 正确：事务外执行
let response = reqwest::get("https://api.example.com")?;
tree.transaction(|tree| {
    let count = get_count(tree)?;
    // ...
    Ok(())
})?;
```

### 3. 处理事务错误

```rust
match tree.transaction(|tree| {
    // ...
}) {
    Ok(result) => { /* 成功 */ }
    Err(TransactionError::Abort(e)) => {
        // 事务被中止（业务逻辑错误）
    }
    Err(TransactionError::Storage(e)) => {
        // 存储错误（磁盘错误等）
    }
}
```

## 总结

- ✅ 使用 Sled 事务确保并发安全
- ✅ 自动处理冲突和重试
- ✅ 异步执行不阻塞工作线程
- ✅ 前端使用 BigInt 处理大数
- ✅ 取模运算模拟溢出行为

这些机制确保了应用在高并发场景下的数据一致性和正确性。
