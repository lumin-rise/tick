use crate::models::{Group, StockRef};
use anyhow::{Context, Result};
use serde_json::Value;

const GROUP_QUERY_URL: &str = "https://ugc.10jqka.com.cn/optdata/selfgroup/open/api/group/v1/query";
const SELF_STOCK_URL: &str = "https://t.10jqka.com.cn/newcircle/group/getSelfStockWithMarket/";

const APP_UA: &str = "Hexin_Gphone/11.28.03 (Royal Flush) hxtheme/0 innerversion/G037.09.028.1.32 followPhoneSystemTheme/0 userid/000000000 getHXAPPAccessibilityMode/0 hxNewFont/1 isVip/0 getHXAPPFontSetting/normal getHXAPPAdaptOldSetting/0 okhttp/3.14.9";
const WEB_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";

/// 同花顺市场号 → 展示市场缩写
/// 参考 ths-favorite constant.py：17=沪 33=深 71/151=北交 55=港 61=美，87 为北交备用码
pub fn ths_market_to_abbr(id: &str) -> &'static str {
    match id {
        "17" | "22" | "18" | "20" => "SH", // 沪主板/沪ST/科创/沪ETF
        "33" | "36" | "38" => "SZ",        // 深主板/深ETF/创业板
        "71" | "151" | "87" => "BJ",       // 北交所
        "55" => "HK",
        "61" => "US",
        _ => "",
    }
}

#[derive(serde::Deserialize)]
struct RawGroup {
    id: String,
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    #[serde(rename = "type")]
    group_type: i64,
    #[serde(default)]
    content: String,
}

/// 拉取全部分组 + 默认自选，组装为业务分组列表。
/// 返回 (分组列表, 是否登录态有效)。
pub async fn fetch_all(client: &reqwest::Client, cookie: &str) -> Result<Vec<Group>> {
    // 1. 自定义分组
    let resp = client
        .get(GROUP_QUERY_URL)
        .query(&[("from", "sjcg_gphone"), ("types", "0,1")])
        .header("User-Agent", APP_UA)
        .header("Cookie", cookie)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("分组请求失败")?;
    if !resp.status().is_success() {
        anyhow::bail!("分组接口返回 {}", resp.status());
    }
    let text = resp.text().await?;
    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("分组响应不是 JSON: {}", truncate(&text)))?;
    // 登录态失效判断：errorCode（或 status_code）非零
    let err_code = json.get("errorCode").and_then(|v| v.as_i64())
        .or_else(|| json.get("status_code").and_then(|v| v.as_i64()))
        .or_else(|| json.get("data").and_then(|d| d.get("errorCode")).and_then(|v| v.as_i64()))
        .or_else(|| json.get("data").and_then(|d| d.get("status_code")).and_then(|v| v.as_i64()));
    if let Some(code) = err_code {
        if code != 0 {
            anyhow::bail!("同花顺登录态已失效 (errorCode={code})");
        }
    }
    // group_list 兼容两种结构：顶层 或 data 内
    let group_node = json.get("group_list")
        .and_then(|v| v.as_array())
        .or_else(|| json.get("data").and_then(|d| d.get("group_list")).and_then(|v| v.as_array()));
    let mut groups: Vec<Group> = Vec::new();
    if let Some(list) = group_node {
        for g in list {
            let raw: RawGroup = serde_json::from_value(g.clone())
                .with_context(|| format!("分组字段解析失败: {}", truncate(&g.to_string())))?;
            groups.push(Group {
                id: raw.id,
                name: raw.name,
                stocks: parse_content(&raw.content),
            });
        }
    }

    // 2. 默认"我的自选"（虚拟分组，放在最前）
    let default_stocks = fetch_default_self(client, cookie).await;
    let mut all = vec![Group {
        id: "default".into(),
        name: "我的自选".into(),
        stocks: default_stocks,
    }];
    all.extend(groups);
    Ok(all)
}

/// 解析分组 content 打包格式："code1|code2,...,market1|market2|..."
fn parse_content(content: &str) -> Vec<StockRef> {
    if content.is_empty() {
        return Vec::new();
    }
    let mut parts = content.splitn(2, ',');
    let codes_seg = parts.next().unwrap_or("");
    let markets_seg = parts.next().unwrap_or("");
    let codes: Vec<&str> = codes_seg.split('|').filter(|s| !s.is_empty()).collect();
    let markets: Vec<&str> = markets_seg.split('|').filter(|s| !s.is_empty()).collect();
    codes
        .iter()
        .enumerate()
        .map(|(i, code)| {
            let mid = markets.get(i).copied().unwrap_or("");
            StockRef {
                market: ths_market_to_abbr(mid).to_string(),
                code: code.to_string(),
                name: String::new(),
                source: "ths".into(),
            }
        })
        .collect()
}

/// 拉默认"我的自选"（JSONP），失败返回空列表（老账号可能无数据）
async fn fetch_default_self(client: &reqwest::Client, cookie: &str) -> Vec<StockRef> {
    let ts = chrono::Utc::now().timestamp_millis();
    let result = client
        .get(SELF_STOCK_URL)
        .query(&[("callback", "selfStock"), ("_", &ts.to_string())])
        .header("User-Agent", WEB_UA)
        .header("Referer", "http://stock.10jqka.com.cn/my/zixuan.shtml")
        .header("Cookie", cookie)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;
    let Ok(resp) = result else { return Vec::new() };
    let Ok(text) = resp.text().await else { return Vec::new() };
    // JSONP 外壳形如 selfStock({...});  （结尾可能带分号）
    let trimmed = text.trim().trim_end_matches(';');
    let inner = trimmed
        .strip_prefix("selfStock(")
        .and_then(|s| s.strip_suffix(")"))
        .unwrap_or(trimmed);
    let Ok(json) = serde_json::from_str::<Value>(inner) else {
        return Vec::new();
    };
    if json.get("errorCode").and_then(|v| v.as_i64()) != Some(0) {
        return Vec::new();
    }
    json.get("result")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    let code = e.get("code")?.as_str()?.to_string();
                    let mid = e.get("marketid").and_then(|m| m.as_str()).unwrap_or("");
                    Some(StockRef {
                        market: ths_market_to_abbr(mid).to_string(),
                        code,
                        name: String::new(),
                        source: "ths".into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn truncate(s: &str) -> String {
    if s.chars().count() > 120 {
        s.chars().take(120).collect()
    } else {
        s.to_string()
    }
}
