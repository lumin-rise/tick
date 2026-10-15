export interface StockRef {
  market: string; // SH/SZ/BJ/HK/US
  code: string;
  name: string;
  source: "ths" | "local" | string;
}

export interface Group {
  id: string;
  name: string;
  stocks: StockRef[];
}

export interface Quote {
  market: string;
  code: string;
  name: string;
  price: number;
  prev_close: number;
  open: number;
  high: number;
  low: number;
  change: number;
  change_pct: number;
  volume: number;
  turnover: number;
  time: string;
  kind: "stock" | "index" | string;
  /** 集合竞价成交量（手），仅 A 股 */
  auction_volume?: number | null;
}

export interface Settings {
  quote_interval_secs: number;
  sync_interval_secs: number;
}

export interface StateSnapshot {
  logged_in: boolean;
  groups: Group[];
  local_stocks: StockRef[];
  quotes: Quote[];
  settings: Settings;
  last_sync: string | null;
}

export interface KlineItem {
  date: string;
  open: number;
  close: number;
  high: number;
  low: number;
  volume: number;
  avg: number | null;
}

export interface KlineData {
  prev_close: number;
  items: KlineItem[];
}

export type MarketTab = "ALL" | "CN" | "HK" | "US" | "CRYPTO";

export const quoteKey = (market: string, code: string) => `${market}:${code}`;
