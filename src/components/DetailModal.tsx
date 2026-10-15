import type { Quote } from "../types";

interface Props {
  market: string;
  code: string;
  quote?: Quote;
  onClose: () => void;
}

const cls = (v: number) => (v > 0.0001 ? "up" : v < -0.0001 ? "down" : "flat");
const fmt = (v: number | undefined, d = 2) =>
  v === undefined || Number.isNaN(v) ? "--" : v.toFixed(d);
const fmtVol = (v: number | undefined) => {
  if (v === undefined || Number.isNaN(v)) return "--";
  if (v >= 1000000) return `${(v / 1000000).toFixed(2)}百万手`;
  if (v >= 10000) return `${(v / 10000).toFixed(2)}万手`;
  return `${Math.round(v)}手`;
};
const fmtAmt = (v: number | undefined) => {
  if (v === undefined || Number.isNaN(v)) return "--";
  if (v >= 100000000) return `${(v / 100000000).toFixed(2)}亿`;
  if (v >= 10000) return `${(v / 10000).toFixed(2)}万`;
  return v.toFixed(0);
};

export default function DetailModal({ market, code, quote, onClose }: Props) {
  const pct = quote?.change_pct ?? 0;
  const name = quote?.name || code;

  const items: [string, string, string][] = [
    ["今开", fmt(quote?.open), cls((quote?.open ?? 0) - (quote?.prev_close ?? 0))],
    ["昨收", fmt(quote?.prev_close), ""],
    ["最高", fmt(quote?.high), cls((quote?.high ?? 0) - (quote?.prev_close ?? 0))],
    ["最低", fmt(quote?.low), cls((quote?.low ?? 0) - (quote?.prev_close ?? 0))],
    ["涨跌", `${(quote?.change ?? 0) > 0 ? "+" : ""}${fmt(quote?.change)}`, cls(quote?.change ?? 0)],
    ["成交量", fmtVol(quote?.volume), ""],
    ["成交额", `${fmtAmt(quote?.turnover)}元`, ""],
    ["行情时间", quote?.time || "--", ""],
  ];

  return (
    <div className="modal-mask" onClick={onClose}>
      <div className="modal" style={{ width: 360 }} onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div className="k-title">
            <span className={`kp ${cls(pct)}`}>{quote?.price?.toFixed(2) ?? "--"}</span>
            <span className={`kn ${cls(pct)}`}>
              {pct > 0 ? "+" : ""}{pct.toFixed(2)}%
            </span>
            <span className="kn" style={{ fontWeight: 400 }}>{name} · {market}{code}</span>
          </div>
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          <div className="quote-grid">
            {items.map(([label, value, color]) => (
              <div className="quote-cell" key={label}>
                <span className="q-label">{label}</span>
                <span className={`q-value ${color}`}>{value}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
