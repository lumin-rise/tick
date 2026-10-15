import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef } from "react";
import type { Quote, StockRef } from "../types";

export type SortKey = "name" | "price" | "low" | "high" | "pct" | "auction";
export interface SortState { key: SortKey; desc: boolean; }

export interface Row {
  stock: StockRef;
  quote?: Quote;
}

interface Props {
  rows: Row[];
  sort: SortState;
  onSort: (key: SortKey) => void;
  editMode: boolean;
  onOpenDetail: (market: string, code: string) => void;
  onDelete: (stock: StockRef) => void;
}

const cls = (v: number) => (v > 0.0001 ? "up" : v < -0.0001 ? "down" : "flat");
const bg = (v: number) => (v > 0.0001 ? "bg-up" : v < -0.0001 ? "bg-down" : "bg-flat");
const fmt = (v: number | undefined, digits = 2) =>
  v === undefined || Number.isNaN(v) ? "--" : v.toFixed(digits);

const fmtAuction = (v: number | null | undefined) => {
  if (v === undefined || v === null || Number.isNaN(v)) return "--";
  if (v >= 10000) return `${(v / 10000).toFixed(2)}万手`;
  return `${Math.round(v)}手`;
};

function Th({ label, k, sort, onSort, flex }: { label: string; k?: SortKey; sort?: SortState; onSort?: (key: SortKey) => void; flex?: number }) {
  if (!k) return <div style={{ flex: flex ?? 1 }}>{label}</div>;
  const active = sort?.key === k;
  return (
    <div style={{ flex, display: "flex", justifyContent: "flex-end" }}>
      <button className={`col-sort ${active ? "sorted" : ""}`} onClick={() => onSort!(k)}>
        {label}
        {active ? (sort!.desc ? " ↓" : " ↑") : ""}
      </button>
    </div>
  );
}

export default function StockList({ rows, sort, onSort, editMode, onOpenDetail, onDelete }: Props) {
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 55,
    overscan: 12,
  });

  return (
    <div className="list-wrap">
      <div className="list-header">
        <Th label="名称" flex={2} />
        <Th label="现价" k="price" sort={sort} onSort={onSort} flex={1} />
        <Th label="最低" k="low" sort={sort} onSort={onSort} flex={0.9} />
        <Th label="最高" k="high" sort={sort} onSort={onSort} flex={0.9} />
        <Th label="竞价量" k="auction" sort={sort} onSort={onSort} flex={0.9} />
        <Th label="涨跌幅" k="pct" sort={sort} onSort={onSort} flex={1.1} />
        {editMode && <div style={{ width: 34 }} />}
      </div>
      <div className="list-scroll" ref={parentRef}>
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((vi) => {
            const { stock, quote } = rows[vi.index];
            const pct = quote?.change_pct ?? 0;
            return (
              <div
                className="row"
                key={`${stock.market}:${stock.code}`}
                style={{ position: "absolute", top: 0, left: 0, width: "100%", transform: `translateY(${vi.start}px)` }}
                onClick={() => onOpenDetail(stock.market, stock.code)}
              >
                <div className="name-block" style={{ flex: 2 }}>
                  <span className="sname">{quote?.name || stock.name || stock.code}</span>
                  <span className="scode">
                    {stock.market} · {stock.code}
                    {stock.source === "local" && <span className="badge-src local">本地</span>}
                  </span>
                </div>
                <div className={`price-block price ${cls(quote?.change ?? 0)}`} style={{ flex: 1 }}>
                  {fmt(quote?.price)}
                </div>
                <div style={{ flex: 0.9, textAlign: "right", fontSize: 12 }} className={cls(quote?.change ?? 0)}>
                  {fmt(quote?.low)}
                </div>
                <div style={{ flex: 0.9, textAlign: "right", fontSize: 12 }} className={cls(quote?.change ?? 0)}>
                  {fmt(quote?.high)}
                </div>
                <div style={{ flex: 0.9, textAlign: "right", fontSize: 12, color: "var(--text-dim)" }}>
                  {fmtAuction(quote?.auction_volume)}
                </div>
                <div className="pct-block" style={{ flex: 1.1 }}>
                  <span className={`pct-pill ${bg(pct)} ${cls(pct)}`}>
                    {pct > 0 ? "+" : ""}{fmt(pct)}%
                  </span>
                </div>
                {editMode && (
                  <button
                    className="del-btn"
                    title={stock.source === "local" ? "从本地自选删除" : "同花顺分组为只读，请在手机 App 中删除"}
                    disabled={stock.source !== "local"}
                    style={stock.source !== "local" ? { visibility: "visible", opacity: 0.3, cursor: "not-allowed" } : undefined}
                    onClick={(e) => {
                      e.stopPropagation();
                      if (stock.source === "local") onDelete(stock);
                    }}
                  >
                    ✕
                  </button>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
