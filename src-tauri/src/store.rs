use crate::models::{Group, Settings, StockRef};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;

/// 落盘的数据（app data dir 下的 stocker.json）
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Persisted {
    #[serde(default)]
    pub cookie: String,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub local_stocks: Vec<StockRef>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub last_sync: Option<String>,
    /// 竞价量（手），key: "SH:600519"
    #[serde(default)]
    pub auction_volumes: HashMap<String, f64>,
    /// 竞价量所属交易日（YYYYMMDD），跨日自动清空
    #[serde(default)]
    pub auction_date: String,
}

pub fn load(path: &PathBuf) -> Persisted {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &PathBuf, data: &Persisted) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).context("创建数据目录失败")?;
    }
    std::fs::write(path, serde_json::to_string_pretty(data)?).context("写入数据文件失败")
}
