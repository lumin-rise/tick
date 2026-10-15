import { useEffect, useRef, useState } from "react";
import { api } from "../api";

interface SearchItem {
  market: string;
  code: string;
  name: string;
  pinyin: string;
}

interface Props {
  onClose: () => void;
  onAdded: (name: string) => void;
}

export default function AddStockModal({ onClose, onAdded }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [adding, setAdding] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);

  // 输入防抖 350ms 自动搜索
  useEffect(() => {
    window.clearTimeout(timer.current);
    const q = query.trim();
    if (!q) {
      setResults([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    timer.current = window.setTimeout(async () => {
      try {
        const list = await api.searchStocks(q);
        setResults(list);
        setError("");
      } catch (e) {
        setError(String(e));
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 350);
    return () => window.clearTimeout(timer.current);
  }, [query]);

  const addBySymbol = async (symbol: string) => {
    setError("");
    try {
      const stock = await api.addLocalStock(symbol);
      onAdded(`${stock.name} 已加入本地自选`);
      onClose();
    } catch (e) {
      setError(String(e));
    }
  };

  const addItem = async (it: SearchItem) => {
    setAdding(it.market + it.code);
    await addBySymbol(`${it.market.toLowerCase()}${it.code}`);
    setAdding(null);
  };

  // 回车：有结果加第一条；无结果按代码直接加（兼容 00700/AAPL 等）
  const onEnter = async () => {
    const q = query.trim();
    if (!q) return;
    if (results.length > 0) {
      await addItem(results[0]);
    } else {
      await addBySymbol(q);
    }
  };

  return (
    <div className="modal-mask" onClick={onClose}>
      <div className="modal" style={{ width: 420 }} onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          添加自选
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          <input
            className="input"
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && onEnter()}
            placeholder="输入名称 / 简称 / 代码 / 拼音首字母，如：茅台、mt、600519"
          />
          <div className="form-error">{error}</div>
          {loading && <div className="hint" style={{ marginTop: 8 }}>搜索中…（首次加载全市场目录约需几秒）</div>}
          {!loading && query.trim() && results.length === 0 && (
            <div className="hint" style={{ marginTop: 8 }}>
              未搜到匹配股票；如果是港美股代码（如 00700 / AAPL）请直接回车添加
            </div>
          )}
          <div className="search-results" style={{ marginTop: 8 }}>
            {results.map((it) => (
              <div
                className="search-item"
                key={`${it.market}:${it.code}`}
                onClick={() => addItem(it)}
              >
                <span className="si-name">{it.name}</span>
                <span className="si-code">{it.market} · {it.code} · {it.pinyin}</span>
                <button
                  className="btn small"
                  disabled={adding === it.market + it.code}
                  onClick={(e) => { e.stopPropagation(); addItem(it); }}
                >
                  {adding === it.market + it.code ? "添加中" : "＋ 添加"}
                </button>
              </div>
            ))}
          </div>
          <p className="hint" style={{ marginTop: 10 }}>
            仅加入本地自选，不影响同花顺数据。
          </p>
        </div>
      </div>
    </div>
  );
}
