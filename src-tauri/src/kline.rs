use crate::models::{KlineData, KlineItem};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;

/// 拉取日 K（前复权，320 根）或分时（当日）。
/// symbol 形如 sh600519。
pub async fn fetch_kline(client: &reqwest::Client, symbol: &str, period: &str) -> Result<KlineData> {
    match period {
        "day" => fetch_day(client, symbol).await,
        "minute" => fetch_minute(client, symbol).await,
        other => Err(anyhow!("不支持的周期: {other}")),
    }
}

async fn fetch_day(client: &reqwest::Client, symbol: &str) -> Result<KlineData> {
    let url = format!(
        "https://web.ifzq.gtimg.cn/appstock/app/fqkline/get?param={symbol},day,,,320,qfq"
    );
    let resp = client
        .get(&url)
        .header("Referer", "https://gu.qq.com/")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("日K请求失败")?;
    let json: Value = resp.json().await.context("日K响应解析失败")?;
    let node = json
        .get("data")
        .and_then(|d| d.get(symbol))
        .ok_or_else(|| anyhow!("日K数据为空"))?;
    // 优先 qfqday，缺省回退 day
    let rows = node
        .get("qfqday")
        .or_else(|| node.get("day"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("日K数组缺失"))?;
    let mut items = Vec::with_capacity(rows.len());
    for r in rows {
        let arr = r.as_array().ok_or_else(|| anyhow!("日K行格式异常"))?;
        if arr.len() < 6 {
            continue;
        }
        items.push(KlineItem {
            date: str_field(arr, 0),
            open: num_field(arr, 1),
            close: num_field(arr, 2),
            high: num_field(arr, 3),
            low: num_field(arr, 4),
            volume: num_field(arr, 5),
            avg: None,
        });
    }
    let prev_close = items
        .first()
        .map(|it| it.open)
        .filter(|v| *v > 0.0)
        .unwrap_or(0.0);
    Ok(KlineData { prev_close, items })
}

async fn fetch_minute(client: &reqwest::Client, symbol: &str) -> Result<KlineData> {
    let url = format!("https://web.ifzq.gtimg.cn/appstock/app/minute/query?code={symbol}");
    let resp = client
        .get(&url)
        .header("Referer", "https://gu.qq.com/")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("分时请求失败")?;
    let json: Value = resp.json().await.context("分时响应解析失败")?;
    let data = json.get("data").ok_or_else(|| anyhow!("分时数据为空"))?;
    let rows = data
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("分数组缺失"))?;
    let mut items = Vec::with_capacity(rows.len());
    for r in rows {
        // "0930 1270.10 83 10541830.00" → 时间 价格 累计量(手) 累计额(元)
        let Some(s) = r.as_str() else { continue };
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let price: f64 = parts[1].parse().unwrap_or(0.0);
        let cum_vol: f64 = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0.0);
        let cum_amt: f64 = parts.get(3).and_then(|p| p.parse().ok()).unwrap_or(0.0);
        let avg = if cum_vol > 0.0 { Some(cum_amt / (cum_vol * 100.0)) } else { None };
        items.push(KlineItem {
            date: parts[0].to_string(),
            open: price,
            close: price,
            high: price,
            low: price,
            volume: cum_vol,
            avg,
        });
    }
    // 昨收取行情快照 qt 字段的昨收（idx 4），取不到回退开盘价
    let prev_close = data
        .get(symbol)
        .and_then(|s| s.get("qt"))
        .and_then(|s| s.get(symbol))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.get(4))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .filter(|v| *v > 0.0)
        .or_else(|| items.first().map(|it| it.open))
        .unwrap_or(0.0);
    Ok(KlineData { prev_close, items })
}

/// 拉取当日竞价金额（元）：校验分时数据是 today 当天的，取第一根(09:30)的累计成交额。
/// 数据不是今天（如盘前拉到昨日数据）返回 None。
pub async fn fetch_auction_amount(client: &reqwest::Client, symbol: &str, today: &str) -> Option<f64> {
    let url = format!("https://web.ifzq.gtimg.cn/appstock/app/minute/query?code={symbol}");
    let resp = client
        .get(&url)
        .header("Referer", "https://gu.qq.com/")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    let json: Value = resp.json().await.ok()?;
    let node = json.get("data")?.get(symbol)?;
    // qt 快照里的时间（idx 30，YYYYMMDDHHMMSS）校验日期
    let qt_time = node
        .get("qt")?.get(symbol)?
        .as_array()?.get(30)?
        .as_str()?;
    if !qt_time.starts_with(today) {
        return None;
    }
    let first = node
        .get("data")?.get("data")?
        .as_array()?.first()?
        .as_str()?;
    // "0930 价格 累计量(手) 累计额(元)"
    first.split_whitespace().nth(3)?.parse().ok()
}

fn str_field(arr: &[Value], i: usize) -> String {
    match arr.get(i) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

fn num_field(arr: &[Value], i: usize) -> f64 {
    match arr.get(i) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}
