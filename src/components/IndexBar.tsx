import type { Quote } from "../types";

const INDEX_NAMES: Record<string, string> = {
  "000001": "上证指数",
  "399001": "深证成指",
  "399006": "创业板指",
  "000688": "科创50",
};

interface Props {
  quotes: Quote[];
  loggedIn: boolean;
  lastSync: string | null;
}

export default function IndexBar({ quotes, loggedIn, lastSync }: Props) {
  const indexes = quotes.filter((q) => q.kind === "index");
  const cls = (v: number) => (v > 0.0001 ? "up" : v < -0.0001 ? "down" : "flat");
  return (
    <footer className="indexbar">
      {indexes.map((q) => (
        <span className="idx" key={q.code}>
          <span className="iname">{INDEX_NAMES[q.code] ?? q.name}</span>
          <span className={`ipx ${cls(q.change)}`}>{q.price.toFixed(2)}</span>
          <span className={cls(q.change)}>
            {q.change_pct > 0 ? "+" : ""}{q.change_pct.toFixed(2)}%
          </span>
        </span>
      ))}
      <div className="status-right">
        <span>
          <span className={`sync-dot ${loggedIn ? "" : "off"}`} />
          {loggedIn ? `已同步 ${lastSync ?? "--"}` : "未登录同花顺"}
        </span>
        <span>行情源：腾讯</span>
      </div>
    </footer>
  );
}
