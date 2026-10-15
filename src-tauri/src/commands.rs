use crate::kline;
use crate::models::*;
use crate::state::{http_client, AppState};
use crate::ths;
use std::sync::Arc;

#[tauri::command]
pub fn get_state(state: tauri::State<'_, Arc<AppState>>) -> StateSnapshot {
    state.snapshot()
}

/// 账号密码登录同花顺并立即同步分组
#[tauri::command]
pub async fn login_ths(state: tauri::State<'_, Arc<AppState>>, account: String, passwd: String) -> Result<StateSnapshot, String> {
    let client = http_client();
    let session = ths::auth::login(&client, &account, &passwd)
        .await
        .map_err(|e| format!("{e:#}"))?;
    apply_cookie(&state, session.cookies).await
}

/// 手动导入浏览器 cookie（兜底方案）
#[tauri::command]
pub async fn import_cookie(state: tauri::State<'_, Arc<AppState>>, cookie_text: String) -> Result<StateSnapshot, String> {
    let cookie = normalize_cookie(&cookie_text);
    if !cookie.contains("userid") {
        return Err("cookie 中缺少 userid 字段，请确认复制的是登录后的整段 cookie".into());
    }
    apply_cookie(&state, cookie).await
}

#[tauri::command]
pub fn logout_ths(state: tauri::State<'_, Arc<AppState>>) {
    {
        let mut inner = state.inner.lock().unwrap();
        inner.cookie.clear();
        inner.logged_in = false;
        inner.groups.clear();
        inner.last_sync = None;
    }
    state.persist();
}

/// 手动触发一次同步
#[tauri::command]
pub async fn sync_now(state: tauri::State<'_, Arc<AppState>>) -> Result<StateSnapshot, String> {
    let cookie = state.inner.lock().unwrap().cookie.clone();
    if cookie.is_empty() {
        return Err("尚未登录同花顺".into());
    }
    let client = http_client();
    let groups = ths::groups::fetch_all(&client, &cookie)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    {
        let mut inner = state.inner.lock().unwrap();
        inner.groups = groups;
        inner.logged_in = true;
        inner.last_sync = Some(now);
    }
    state.persist();
    Ok(state.snapshot())
}

/// 本地添加自选。输入支持：600519 / 000001 / 430047 / 00700 / AAPL / sh600519
#[tauri::command]
pub async fn add_local_stock(state: tauri::State<'_, Arc<AppState>>, input: String) -> Result<StockRef, String> {
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return Err("请输入代码".into());
    }
    let (market, code) = guess_market(&input)?;
    let symbol = format!("{market}{code}");
    let client = http_client();
    let (name, market) = crate::quotes::fetch_name(&client, &symbol)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let stock = StockRef { market, code: code.to_uppercase(), name, source: "local".into() };
    {
        let mut inner = state.inner.lock().unwrap();
        if inner.local_stocks.iter().any(|s| s.market == stock.market && s.code == stock.code) {
            return Err("已在本地自选中".into());
        }
        inner.local_stocks.push(stock.clone());
    }
    state.persist();
    Ok(stock)
}

#[tauri::command]
pub fn remove_local_stock(state: tauri::State<'_, Arc<AppState>>, market: String, code: String) -> Result<(), String> {
    let mut inner = state.inner.lock().unwrap();
    inner.local_stocks.retain(|s| !(s.market == market && s.code == code));
    drop(inner);
    state.persist();
    Ok(())
}

#[tauri::command]
pub async fn search_stocks(query: String) -> Result<Vec<crate::market::Item>, String> {
    crate::market::search(&query, 20).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn get_kline(symbol: String, period: String) -> Result<KlineData, String> {
    let client = http_client();
    kline::fetch_kline(&client, &symbol, &period)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn set_settings(state: tauri::State<'_, Arc<AppState>>, settings: Settings) -> Result<(), String> {
    {
        let mut inner = state.inner.lock().unwrap();
        inner.settings = settings;
    }
    state.persist();
    Ok(())
}

/// 写入 cookie 后立即同步验证有效性
pub(crate) async fn apply_cookie(state: &tauri::State<'_, Arc<AppState>>, cookie: String) -> Result<StateSnapshot, String> {
    let client = http_client();
    let groups = ths::groups::fetch_all(&client, &cookie)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    {
        let mut inner = state.inner.lock().unwrap();
        inner.cookie = cookie;
        inner.groups = groups;
        inner.logged_in = true;
        inner.last_sync = Some(now);
    }
    state.persist();
    Ok(state.snapshot())
}

/// 用户可能粘贴 "k=v; k2=v2" 或整段 Request Header，统一成 "k=v; k2=v2"
fn normalize_cookie(raw: &str) -> String {
    let mut pairs: Vec<String> = Vec::new();
    for part in raw.split(|c| c == ';' || c == '\n') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // 去掉 "Cookie:" 前缀
        let part = part.strip_prefix("cookie:").unwrap_or(part).trim();
        if let Some((k, v)) = part.split_once('=') {
            let kv = format!("{}={}", k.trim(), v.trim());
            if !pairs.contains(&kv) {
                pairs.push(kv);
            }
        }
    }
    pairs.join("; ")
}

/// 按代码规则推断市场，返回 (小写前缀, 代码)
fn guess_market(input: &str) -> Result<(String, String), String> {
    if let Some(rest) = input.strip_prefix("sh").or_else(|| input.strip_prefix("sz"))
        .or_else(|| input.strip_prefix("bj")).or_else(|| input.strip_prefix("hk"))
        .or_else(|| input.strip_prefix("us")) {
        if rest.len() >= 4 {
            return Ok((prefix_of(input), rest.to_string()));
        }
    }
    let code = input;
    if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
        return Ok(if code.starts_with("60") || code.starts_with("68") || code.starts_with("9") {
            ("sh".into(), code.into())
        } else if code.starts_with("00") || code.starts_with("30") || code.starts_with("2") {
            ("sz".into(), code.into())
        } else {
            ("bj".into(), code.into())
        });
    }
    if code.len() == 5 && code.chars().all(|c| c.is_ascii_digit()) {
        return Ok(("hk".into(), code.into()));
    }
    if code.chars().all(|c| c.is_ascii_alphanumeric()) && code.chars().any(|c| c.is_ascii_alphabetic()) {
        return Ok(("us".into(), code.to_uppercase()));
    }
    Err("无法识别的代码格式，支持如 600519 / 000001 / 430047 / 00700 / AAPL".into())
}

fn prefix_of(input: &str) -> String {
    input[..2].to_string()
}
