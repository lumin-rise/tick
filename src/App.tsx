import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, onAuthExpired, onGroupsUpdate, onQuotesUpdate } from "./api";
import AddStockModal from "./components/AddStockModal";
import DetailModal from "./components/DetailModal";
import GroupBar, { GROUP_ALL, GROUP_LOCAL } from "./components/GroupBar";
import IndexBar from "./components/IndexBar";
import SettingsModal from "./components/SettingsModal";
import Sidebar from "./components/Sidebar";
import StockList, { type Row, type SortKey, type SortState } from "./components/StockList";
import TopBar from "./components/TopBar";
import { quoteKey, type MarketTab, type Quote, type StateSnapshot, type StockRef } from "./types";
import "./styles.css";

const CN_MARKETS = new Set(["SH", "SZ", "BJ"]);

export default function App() {
  const [snap, setSnap] = useState<StateSnapshot | null>(null);
  const [quotes, setQuotes] = useState<Map<string, Quote>>(new Map());
  const [tab, setTab] = useState<MarketTab>("CN");
  const [groupId, setGroupId] = useState<string>(GROUP_ALL);
  const [sort, setSort] = useState<SortState>({ key: "pct", desc: true });
  const [editMode, setEditMode] = useState(false);
  const [detail, setDetail] = useState<{ market: string; code: string } | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showAdd, setShowAdd] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  const showToast = useCallback((msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 3200);
  }, []);

  // 初始化 + 事件订阅
  useEffect(() => {
    api.getState().then((s) => {
      setSnap(s);
      setQuotes(new Map(s.quotes.map((q) => [quoteKey(q.market, q.code), q])));
    });
    const unsubs = [
      onQuotesUpdate((qs) => {
        setQuotes((prev) => {
          const m = new Map(prev);
          for (const q of qs) m.set(quoteKey(q.market, q.code), q);
          return m;
        });
      }),
      onGroupsUpdate(async () => {
        const s = await api.getState();
        setSnap(s);
      }),
      onAuthExpired(async (msg) => {
        showToast(`同花顺登录态失效：${msg}`);
        const s = await api.getState();
        setSnap(s);
        setShowSettings(true);
      }),
      listen<string>("ths-login-done", async (e) => {
        showToast(e.payload);
        const s = await api.getState();
        setSnap(s);
        setShowSettings(false);
      }),
    ];
    return () => unsubs.forEach((u) => u.then((f) => f()));
  }, [showToast]);

  // 列表数据组装：选分组 → 市场过滤 → 合并行情 → 排序
  const rows: Row[] = useMemo(() => {
    if (!snap) return [];
    let stocks: StockRef[];
    if (groupId === GROUP_ALL) {
      const seen = new Set<string>();
      stocks = [];
      for (const g of snap.groups) {
        for (const s of g.stocks) {
          const k = quoteKey(s.market, s.code);
          if (!seen.has(k)) { seen.add(k); stocks.push(s); }
        }
      }
      for (const s of snap.local_stocks) {
        const k = quoteKey(s.market, s.code);
        if (!seen.has(k)) { seen.add(k); stocks.push(s); }
      }
    } else if (groupId === GROUP_LOCAL) {
      stocks = snap.local_stocks;
    } else {
      stocks = snap.groups.find((g) => g.id === groupId)?.stocks ?? [];
    }
    if (tab === "CN") stocks = stocks.filter((s) => CN_MARKETS.has(s.market));
    else if (tab !== "ALL") stocks = stocks.filter((s) => s.market === tab);

    const withQuote = stocks.map((s) => ({ stock: s, quote: quotes.get(quoteKey(s.market, s.code)) }));
    const dir = sort.desc ? -1 : 1;
    withQuote.sort((a, b) => {
      const q = sort.key;
      if (q === "name") return dir * `${a.quote?.name ?? a.stock.code}`.localeCompare(`${b.quote?.name ?? b.stock.code}`, "zh");
      const val = (r: Row) =>
        q === "price" ? r.quote?.price
        : q === "low" ? r.quote?.low
        : q === "high" ? r.quote?.high
        : q === "auction" ? r.quote?.auction_amount ?? undefined
        : r.quote?.change_pct;
      const va = val(a), vb = val(b);
      if (va === undefined && vb === undefined) return 0;
      if (va === undefined) return 1;
      if (vb === undefined) return -1;
      return dir * (va - vb);
    });
    return withQuote;
  }, [snap, quotes, tab, groupId, sort]);

  const onSort = (key: SortKey) => {
    setSort((s) => (s.key === key ? { key, desc: !s.desc } : { key, desc: true }));
  };

  const doSync = async () => {
    setSyncing(true);
    try {
      const s = await api.syncNow();
      setSnap(s);
      showToast(`已同步（${s.groups.reduce((n, g) => n + g.stocks.length, 0)} 只 / ${s.groups.length} 组）`);
    } catch (e) {
      showToast(`同步失败：${e}`);
      setShowSettings(true);
    } finally {
      setSyncing(false);
    }
  };

  const onDelete = async (stock: StockRef) => {
    await api.removeLocalStock(stock.market, stock.code);
    setSnap((prev) =>
      prev ? { ...prev, local_stocks: prev.local_stocks.filter((s) => s.market !== stock.market || s.code !== stock.code) } : prev
    );
    showToast("已从本地自选删除");
  };

  const total = rows.length;
  const detailQuote = detail ? quotes.get(quoteKey(detail.market, detail.code)) : undefined;

  return (
    <div className="app">
      <TopBar tab={tab} onTab={setTab} onOpenSettings={() => setShowSettings(true)} />
      <div className="body">
        <Sidebar
          onAdd={() => setShowAdd(true)}
          onSync={doSync}
          syncing={syncing}
          editMode={editMode}
          onToggleEdit={() => setEditMode((v) => !v)}
        />
        <main className="main">
          <GroupBar
            groups={snap?.groups ?? []}
            localCount={snap?.local_stocks.length ?? 0}
            loggedIn={snap?.logged_in ?? false}
            active={groupId}
            onSelect={setGroupId}
            onLogin={() => setShowSettings(true)}
          />
          {total === 0 && !snap ? (
            <div className="list-wrap">
              <div className="list-header">
                <div style={{ flex: 2 }}>名称</div>
                <div style={{ flex: 1, textAlign: "right" }}>现价</div>
                <div style={{ flex: 0.9, textAlign: "right" }}>最低</div>
                <div style={{ flex: 0.9, textAlign: "right" }}>最高</div>
                <div style={{ flex: 0.9, textAlign: "right" }}>竞价量</div>
                <div style={{ flex: 1.1, textAlign: "right" }}>涨跌幅</div>
              </div>
              {Array.from({ length: 12 }).map((_, i) => (
                <div className="row skeleton-row" key={i}>
                  <div className="name-block">
                    <span className="skeleton" style={{ width: 90, height: 14 }} />
                    <span className="skeleton" style={{ width: 60, height: 10 }} />
                  </div>
                  <div className="skeleton" style={{ flex: 1, height: 12, maxWidth: 64, marginLeft: "auto" }} />
                  <div className="skeleton" style={{ flex: 0.9, height: 12, maxWidth: 48 }} />
                  <div className="skeleton" style={{ flex: 0.9, height: 12, maxWidth: 48 }} />
                  <div className="skeleton" style={{ flex: 0.9, height: 12, maxWidth: 52 }} />
                  <div className="skeleton" style={{ flex: 1.1, height: 22, maxWidth: 64 }} />
                </div>
              ))}
            </div>
          ) : total === 0 ? (
            <div className="list-wrap">
              <div className="empty">
                <div className="big">📊</div>
                <div>{snap?.logged_in ? "当前分组暂无股票" : "登录同花顺，同步你的自选股"}</div>
                <div className="actions">
                  {!snap?.logged_in && (
                    <button className="btn primary" onClick={() => setShowSettings(true)}>登录同花顺</button>
                  )}
                  <button className="btn" onClick={() => setShowAdd(true)}>添加自选</button>
                </div>
              </div>
            </div>
          ) : (
            <StockList
              rows={rows}
              sort={sort}
              onSort={onSort}
              editMode={editMode}
              onOpenDetail={(market, code) => setDetail({ market, code })}
              onDelete={onDelete}
            />
          )}
          <IndexBar
            quotes={[...quotes.values()]}
            loggedIn={snap?.logged_in ?? false}
            lastSync={snap?.last_sync ?? null}
          />
        </main>
      </div>

      {detail && (
        <DetailModal
          market={detail.market}
          code={detail.code}
          quote={detailQuote}
          onClose={() => setDetail(null)}
        />
      )}
      {showSettings && (
        <SettingsModal
          snapshot={snap}
          onUpdated={(s) => {
            setSnap(s);
            setQuotes((prev) => {
              const m = new Map(prev);
              for (const q of s.quotes) m.set(quoteKey(q.market, q.code), q);
              return m;
            });
          }}
          onClose={() => setShowSettings(false)}
          onToast={showToast}
        />
      )}
      {showAdd && <AddStockModal onClose={() => setShowAdd(false)} onAdded={showToast} />}
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
