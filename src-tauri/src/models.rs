use serde::{Deserialize, Serialize};

/// 一只自选股的引用（不含行情，行情由 quotes 引擎填充）
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StockRef {
    /// 展示市场：SH / SZ / BJ / HK / US
    pub market: String,
    /// 6 位代码（美股为字母代码）
    pub code: String,
    /// 名称，同花顺分组不提供名称，由行情首次返回时回填；本地自选添加时查询
    pub name: String,
    /// 来源：ths（同花顺同步）/ local（本地添加）
    pub source: String,
}

impl StockRef {
    /// 行情请求符号，如 sh600519 / hk00700 / usAAPL
    pub fn symbol(&self) -> String {
        format!("{}{}", self.market.to_lowercase(), self.code)
    }
}

/// 同花顺（或本地）自选分组
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub stocks: Vec<StockRef>,
}

/// 实时行情快照
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Quote {
    pub market: String,
    pub code: String,
    pub name: String,
    pub price: f64,
    pub prev_close: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub change: f64,
    pub change_pct: f64,
    /// 成交量（手）
    pub volume: f64,
    /// 成交额（万元）
    pub turnover: f64,
    /// 行情时间（原始字符串，各市场格式不一）
    pub time: String,
    /// 类型：stock / index
    pub kind: String,
    /// 集合竞价成交量（手），仅 A 股；竞价期间实时采集或盘后从分时回填
    #[serde(default)]
    pub auction_volume: Option<f64>,
}

/// K 线数据（详情弹窗用）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KlineItem {
    /// 日K：日期；分时：HHmm
    pub date: String,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
    /// 分时均价线（仅分时有）
    pub avg: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KlineData {
    /// 昨收，分时图基线
    pub prev_close: f64,
    pub items: Vec<KlineItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    /// 行情轮询间隔（秒）
    pub quote_interval_secs: u64,
    /// 同花顺分组同步间隔（秒）
    pub sync_interval_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self { quote_interval_secs: 3, sync_interval_secs: 300 }
    }
}

/// 启动/同步后发给前端的完整状态快照
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StateSnapshot {
    pub logged_in: bool,
    pub groups: Vec<Group>,
    pub local_stocks: Vec<StockRef>,
    pub quotes: Vec<Quote>,
    pub settings: Settings,
    pub last_sync: Option<String>,
}

/// 底部指数条
pub const INDEX_SYMBOLS: [&str; 4] = ["sh000001", "sz399001", "sz399006", "sh000688"];
