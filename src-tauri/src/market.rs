//! 全市场 A 股目录：新浪分页并发加载，支持 名称/代码/拼音首字母 模糊搜索
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(serde::Serialize, Clone)]
pub struct Item {
    pub market: String,
    pub code: String,
    pub name: String,
    pub pinyin: String,
}

static CATALOG: RwLock<Option<(Arc<Vec<Item>>, u64)>> = RwLock::const_new(None);

/// 名称 → 拼音首字母（小写）
fn first_letters(name: &str) -> String {
    use pinyin::ToPinyin;
    name.to_pinyin()
        .flatten()
        .map(|p| p.first_letter())
        .collect::<String>()
        .to_lowercase()
}

/// 搜索：名称 / 代码 / 拼音首字母（大小写不敏感），前缀匹配优先
pub async fn search(query: &str, limit: usize) -> Result<Vec<Item>> {
    ensure_loaded().await?;
    let guard = CATALOG.read().await;
    let (items, _) = guard.as_ref().expect("catalog loaded");
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let q_raw = query.trim();
    let mut head = Vec::new();
    let mut tail = Vec::new();
    for it in items.iter() {
        let hit_prefix = it.code.starts_with(&q) || it.name.starts_with(q_raw) || it.pinyin.starts_with(&q);
        let hit = it.code.contains(&q) || it.name.contains(q_raw) || it.pinyin.contains(&q);
        if hit_prefix {
            head.push(Item { market: it.market.clone(), code: it.code.clone(), name: it.name.clone(), pinyin: it.pinyin.clone() });
            if head.len() >= limit {
                break;
            }
        } else if hit {
            tail.push(Item { market: it.market.clone(), code: it.code.clone(), name: it.name.clone(), pinyin: it.pinyin.clone() });
        }
    }
    head.extend(tail);
    head.truncate(limit);
    Ok(head)
}

/// 4 小时内只加载一次
async fn ensure_loaded() -> Result<()> {
    {
        let guard = CATALOG.read().await;
        if let Some((_, ts)) = guard.as_ref() {
            if now_ms() - ts < 4 * 3600_000 {
                return Ok(());
            }
        }
    }
    let mut guard = CATALOG.write().await;
    if let Some((_, ts)) = guard.as_ref() {
        if now_ms() - ts < 4 * 3600_000 {
            return Ok(());
        }
    }
    let items = load_all().await?;
    *guard = Some((Arc::new(items), now_ms()));
    Ok(())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 新浪 hs_a 分页并发加载（每页 100，约 60 页）
async fn load_all() -> Result<Vec<Item>> {
    let client = crate::state::http_client();
    let mut handles = Vec::new();
    for page in 1..=60 {
        let c = client.clone();
        handles.push(tokio::spawn(async move { fetch_page(&c, page).await }));
    }
    let mut all = Vec::new();
    for h in handles {
        if let Ok(Ok(items)) = h.await {
            all.extend(items);
        }
    }
    Ok(all)
}

async fn fetch_page(client: &reqwest::Client, page: u32) -> Result<Vec<Item>> {
    let url = format!(
        "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData?page={page}&num=100&sort=symbol&asc=1&node=hs_a"
    );
    let text = client
        .get(&url)
        .header("Referer", "https://finance.sina.com.cn")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?
        .text()
        .await?;
    let body = text.trim();
    if body.is_empty() || body == "null" || body.starts_with('<') {
        return Ok(Vec::new());
    }
    let arr: Vec<serde_json::Value> = serde_json::from_str(body)?;
    let mut out = Vec::new();
    for o in arr {
        let Some(symbol) = o.get("symbol").and_then(|v| v.as_str()) else { continue };
        if symbol.len() < 4 {
            continue;
        }
        let market = match &symbol[..2] {
            "sh" => "SH",
            "sz" => "SZ",
            "bj" => "BJ",
            _ => continue,
        };
        let Some(name) = o.get("name").and_then(|v| v.as_str()) else { continue };
        out.push(Item {
            market: market.into(),
            code: symbol[2..].into(),
            name: name.into(),
            pinyin: first_letters(name),
        });
    }
    Ok(out)
}
