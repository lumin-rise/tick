use crate::models::*;
use crate::quotes;
use crate::store::{self, Persisted};
use crate::ths;
use chrono::Datelike;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Emitter;

pub struct AppState {
    pub inner: Mutex<Inner>,
    pub data_path: PathBuf,
}

#[derive(Default)]
pub struct Inner {
    pub cookie: String,
    pub logged_in: bool,
    pub groups: Vec<Group>,
    pub local_stocks: Vec<StockRef>,
    pub quote_cache: HashMap<String, Quote>,
    pub settings: Settings,
    pub last_sync: Option<String>,
    pub auction_volumes: HashMap<String, f64>,
    pub auction_date: String,
    auction_saved_at: Option<std::time::Instant>,
}

impl AppState {
    pub fn new(data_path: PathBuf) -> Self {
        let p: Persisted = store::load(&data_path);
        let logged_in = !p.cookie.is_empty();
        Self {
            inner: Mutex::new(Inner {
                cookie: p.cookie,
                logged_in,
                groups: p.groups,
                local_stocks: p.local_stocks,
                settings: p.settings,
                last_sync: p.last_sync,
                auction_volumes: p.auction_volumes,
                auction_date: p.auction_date,
                ..Default::default()
            }),
            data_path,
        }
    }

    pub fn persist(&self) {
        let inner = self.inner.lock().unwrap();
        let _ = store::save(
            &self.data_path,
            &Persisted {
                cookie: inner.cookie.clone(),
                groups: inner.groups.clone(),
                local_stocks: inner.local_stocks.clone(),
                settings: inner.settings.clone(),
                last_sync: inner.last_sync.clone(),
                auction_volumes: inner.auction_volumes.clone(),
                auction_date: inner.auction_date.clone(),
            },
        );
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let inner = self.inner.lock().unwrap();
        let mut quotes: Vec<Quote> = inner.quote_cache.values().cloned().collect();
        if !inner.auction_date.is_empty() {
            for q in quotes.iter_mut() {
                if is_cn_market(&q.market) {
                    q.auction_volume = inner
                        .auction_volumes
                        .get(&format!("{}:{}", q.market, q.code))
                        .copied();
                }
            }
        }
        StateSnapshot {
            logged_in: inner.logged_in,
            groups: inner.groups.clone(),
            local_stocks: inner.local_stocks.clone(),
            quotes,
            settings: inner.settings.clone(),
            last_sync: inner.last_sync.clone(),
        }
    }

    /// 轮询对象：所有分组并集 + 本地自选 + 指数
    pub fn poll_symbols(&self) -> Vec<String> {
        let inner = self.inner.lock().unwrap();
        let mut seen: Vec<String> = Vec::new();
        let mut push = |s: &StockRef| {
            if !s.market.is_empty() {
                let sym = s.symbol();
                if !seen.contains(&sym) {
                    seen.push(sym);
                }
            }
        };
        for g in &inner.groups {
            for s in &g.stocks {
                push(s);
            }
        }
        for s in &inner.local_stocks {
            push(s);
        }
        for idx in INDEX_SYMBOLS {
            seen.push(idx.to_string());
        }
        seen
    }

    /// 行情回来时回填名称（同花顺分组不带名称）
    pub fn backfill_names(&self, quotes: &[Quote]) {
        let mut inner = self.inner.lock().unwrap();
        let mut changed = false;
        for g in inner.groups.iter_mut() {
            for s in g.stocks.iter_mut() {
                if s.name.is_empty() {
                    if let Some(q) = quotes.iter().find(|q| q.market == s.market && q.code == s.code) {
                        s.name = q.name.clone();
                        changed = true;
                    }
                }
            }
        }
        for s in inner.local_stocks.iter_mut() {
            if s.name.is_empty() {
                if let Some(q) = quotes.iter().find(|q| q.market == s.market && q.code == s.code) {
                    s.name = q.name.clone();
                    changed = true;
                }
            }
        }
        if changed {
            drop(inner);
            self.persist();
        }
    }
}

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .build()
        .expect("http client")
}

fn is_cn_market(market: &str) -> bool {
    matches!(market, "SH" | "SZ" | "BJ")
}

impl AppState {
    /// 竞价期间（09:15-09:29）从实时行情采集竞价量；9:29 后冻结不再覆盖。
    /// 返回 true 表示调用方应落盘（已按 15 秒节流）。
    pub fn capture_auction(&self, quotes: &[Quote]) -> bool {
        let now = chrono::Local::now();
        let today = now.format("%Y%m%d").to_string();
        let hms = now.format("%H%M%S").to_string();
        let capturing = hms.as_str() < "092905" && now.weekday().number_from_monday() <= 5;
        let mut changed = false;
        let mut inner = self.inner.lock().unwrap();
        if inner.auction_date != today {
            inner.auction_date = today.clone();
            inner.auction_volumes.clear();
            inner.auction_saved_at = None;
            changed = true;
        }
        if capturing {
            for q in quotes {
                if !is_cn_market(&q.market) || q.volume <= 0.0 || !q.time.starts_with(&today) {
                    continue;
                }
                let key = format!("{}:{}", q.market, q.code);
                if inner.auction_volumes.get(&key) != Some(&q.volume) {
                    inner.auction_volumes.insert(key, q.volume);
                    changed = true;
                }
            }
        }
        if changed {
            let due = inner
                .auction_saved_at
                .map_or(true, |t| t.elapsed() >= std::time::Duration::from_secs(15));
            if due {
                inner.auction_saved_at = Some(std::time::Instant::now());
                return true;
            }
        }
        false
    }

    /// 批量写入竞价量（回填用），调用方负责最后 persist 一次
    pub fn set_auction_many(&self, today: &str, items: &[(String, String, f64)]) {
        let mut inner = self.inner.lock().unwrap();
        if inner.auction_date != today {
            inner.auction_date = today.to_string();
            inner.auction_volumes.clear();
        }
        for (market, code, vol) in items {
            inner.auction_volumes.insert(format!("{market}:{code}"), *vol);
        }
    }

    /// 今天还缺竞价量的 A 股代码列表（用于回填）。日期未初始化或跨日视为全部缺失。
    pub fn missing_auction_today(&self, today: &str) -> Vec<(String, String, String)> {
        let inner = self.inner.lock().unwrap();
        let fresh = inner.auction_date == today;
        let mut out = Vec::new();
        let mut push = |s: &StockRef| {
            if !is_cn_market(&s.market) || s.market.is_empty() {
                return;
            }
            let key = format!("{}:{}", s.market, s.code);
            if !fresh || !inner.auction_volumes.contains_key(&key) {
                out.push((s.market.clone(), s.code.clone(), s.symbol()));
            }
        };
        for g in &inner.groups {
            for s in &g.stocks {
                push(s);
            }
        }
        for s in &inner.local_stocks {
            push(s);
        }
        out
    }
}

/// 行情轮询引擎：批量拉取当前关注股票 + 指数，event 推送前端
pub fn spawn_quote_engine(app: tauri::AppHandle, state: std::sync::Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let client = http_client();
        loop {
            let interval = state.inner.lock().unwrap().settings.quote_interval_secs.max(1);
            let symbols = state.poll_symbols();
            if !symbols.is_empty() {
                match quotes::fetch_quotes(&client, &symbols).await {
                    Ok(qs) => {
                        state.backfill_names(&qs);
                        if state.capture_auction(&qs) {
                            state.persist();
                        }
                        let mut qs = qs;
                        {
                            let inner = state.inner.lock().unwrap();
                            if !inner.auction_date.is_empty() {
                                for q in qs.iter_mut() {
                                    if is_cn_market(&q.market) {
                                        q.auction_volume = inner
                                            .auction_volumes
                                            .get(&format!("{}:{}", q.market, q.code))
                                            .copied();
                                    }
                                }
                            }
                        }
                        {
                            let mut inner = state.inner.lock().unwrap();
                            for (k, v) in quotes::quotes_to_map(&qs) {
                                inner.quote_cache.insert(k, v);
                            }
                        }
                        let _ = app.emit("quotes-update", &qs);
                    }
                    Err(e) => {
                        eprintln!("[quotes] {e:#}");
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    });
}

/// 竞价量回填：软件在 9:25 之后才启动时，从腾讯分时接口拉当日第一根(9:30)的量补齐。
/// 软件在竞价期间就开着的话由 capture_auction 实时采集，本任务空转。
pub fn spawn_auction_backfill(app: tauri::AppHandle, state: std::sync::Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let client = http_client();
        // 等启动同步把分组落稳
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        for pass in 0..3 {
            let now = chrono::Local::now();
            if now.weekday().number_from_monday() > 5 {
                return;
            }
            let hms = now.format("%H%M%S").to_string();
            if hms.as_str() < "092600" {
                // 盘前启动：竞价期间 capture_auction 会实时采集，无需回填
                return;
            }
            let today = now.format("%Y%m%d").to_string();
            let missing = state.missing_auction_today(&today);
            crate::ths_login::log(&app, format!("竞价回填 pass{pass}: 缺 {}/{} 只", missing.len(), state.poll_symbols().len()));
            if missing.is_empty() {
                return;
            }
            let mut filled: Vec<(String, String, f64)> = Vec::new();
            for (market, code, symbol) in missing {
                if let Some(vol) = crate::kline::fetch_auction_volume(&client, &symbol, &today).await {
                    filled.push((market, code, vol));
                }
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
            if !filled.is_empty() {
                state.set_auction_many(&today, &filled);
                state.persist();
            }
            crate::ths_login::log(&app, format!("竞价回填 pass{pass}: 补了 {} 只", filled.len()));
            if pass < 2 {
                // 个别股票分时数据滞后时两分钟后再补一轮
                tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            }
        }
    });
}
/// 同步调度：启动即同步一次，之后按设置间隔；失败且疑似登录态失效时通知前端
pub fn spawn_sync_scheduler(app: tauri::AppHandle, state: std::sync::Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        // 延后 2 秒：让首屏行情先跑起来，同步的网络+写盘不与启动抢资源
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let client = http_client();
        loop {
            let (interval, cookie) = {
                let inner = state.inner.lock().unwrap();
                (inner.settings.sync_interval_secs.max(30), inner.cookie.clone())
            };
            if !cookie.is_empty() {
                match ths::groups::fetch_all(&client, &cookie).await {
                    Ok(groups) => {
                        let snapshot_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        let total: usize = groups.iter().map(|g| g.stocks.len()).sum();
                        crate::ths_login::log(&app, format!("自动同步成功：{} 组 {} 只", groups.len(), total));
                        {
                            let mut inner = state.inner.lock().unwrap();
                            inner.groups = groups;
                            inner.logged_in = true;
                            inner.last_sync = Some(snapshot_time);
                        }
                        state.persist();
                        let _ = app.emit("groups-update", ());
                    }
                    Err(e) => {
                        let msg = format!("{e:#}");
                        crate::ths_login::log(&app, format!("自动同步失败: {msg}"));
                        if msg.contains("失效") || msg.contains("errorCode") {
                            let mut inner = state.inner.lock().unwrap();
                            inner.logged_in = false;
                            drop(inner);
                            let _ = app.emit("ths-auth-expired", msg);
                        }
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    });
}
