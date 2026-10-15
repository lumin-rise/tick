use crate::models::Quote;
use anyhow::{Context, Result};
use encoding_rs::GBK;
use std::collections::HashMap;

const BATCH_SIZE: usize = 60;

/// 批量拉取腾讯实时行情。codes 形如 sh600519 / sz000001 / hk00700。
pub async fn fetch_quotes(client: &reqwest::Client, codes: &[String]) -> Result<Vec<Quote>> {
    let mut out = Vec::with_capacity(codes.len());
    for chunk in codes.chunks(BATCH_SIZE) {
        let url = format!("https://qt.gtimg.cn/q={}", chunk.join(","));
        let resp = client
            .get(&url)
            .header("Referer", "https://gu.qq.com/")
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await
            .context("腾讯行情请求失败")?;
        let bytes = resp.bytes().await?;
        let (text, _, _) = GBK.decode(&bytes);
        for line in text.split(';') {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(q) = parse_line(line) {
                out.push(q);
            }
        }
    }
    Ok(out)
}

/// 解析单行 `v_sh600519="1~贵州茅台~600519~..."`。
/// 涨跌额/涨跌幅统一用 (price - prev_close) 自算，规避各市场字段口径差异。
fn parse_line(line: &str) -> Option<Quote> {
    let eq = line.find('=')?;
    let symbol = line[..eq].trim().strip_prefix("v_")?.to_string();
    let payload = line[eq + 1..].trim().trim_matches('"');
    let fields: Vec<&str> = payload.split('~').collect();
    if fields.len() < 36 {
        return None;
    }
    let market = symbol_prefix_market(&symbol)?;
    let code = symbol[2..].to_string();
    let name = fields[1].trim().to_string();
    let price: f64 = fields[3].parse().ok()?;
    let prev_close: f64 = fields[4].parse().unwrap_or(0.0);
    let open: f64 = fields[5].parse().unwrap_or(0.0);
    let high: f64 = fields[33].parse().unwrap_or(0.0);
    let low: f64 = fields[34].parse().unwrap_or(0.0);
    // 高低价字段缺省时回退 41/42（部分市场）
    let high = if high <= 0.0 { fields.get(41).and_then(|s| s.parse().ok()).unwrap_or(0.0) } else { high };
    let low = if low <= 0.0 { fields.get(42).and_then(|s| s.parse().ok()).unwrap_or(0.0) } else { low };
    let volume: f64 = fields[6].parse().unwrap_or(0.0);
    let turnover: f64 = fields[37].parse().unwrap_or(0.0);
    let time = fields.get(30).unwrap_or(&"").to_string();
    let (change, change_pct) = if prev_close > 0.0 {
        let c = price - prev_close;
        (c, c / prev_close * 100.0)
    } else {
        (0.0, 0.0)
    };
    let kind = if market == "INDEX" { "index".into() } else { "stock".into() };
    Some(Quote {
        market,
        code,
        name,
        price,
        prev_close,
        open,
        high,
        low,
        change,
        change_pct,
        volume,
        turnover,
        time,
        kind,
        auction_amount: None,
    })
}

/// symbol 前缀 → 展示市场。指数（sh000xxx/sz39xxxx）单独标记。
fn symbol_prefix_market(symbol: &str) -> Option<String> {
    let pfx = symbol.get(0..2)?;
    let code = &symbol[2..];
    match pfx {
        "sh" => Some(if code.starts_with("000") { "INDEX".into() } else { "SH".into() }),
        "sz" => Some(if code.starts_with("39") { "INDEX".into() } else { "SZ".into() }),
        "bj" => Some("BJ".into()),
        "hk" => Some("HK".into()),
        "us" => Some("US".into()),
        _ => None,
    }
}

/// 拉单只股票名称（本地添加自选时用）
pub async fn fetch_name(client: &reqwest::Client, symbol: &str) -> Result<(String, String)> {
    let quotes = fetch_quotes(client, &[symbol.to_string()]).await?;
    let q = quotes.into_iter().next().with_context(|| format!("未查到 {symbol} 的行情，请检查代码"))?;
    if q.name.is_empty() {
        anyhow::bail!("未查到 {symbol} 的名称");
    }
    Ok((q.name, q.market))
}

/// 用于本地缓存的 map 形式
pub fn quotes_to_map(quotes: &[Quote]) -> HashMap<String, Quote> {
    quotes
        .iter()
        .map(|q| (format!("{}:{}", q.market, q.code), q.clone()))
        .collect()
}
