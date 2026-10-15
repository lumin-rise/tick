import { getCurrentWindow } from "@tauri-apps/api/window";
import { useState } from "react";
import type { MarketTab } from "../types";

const TABS: { key: MarketTab; label: string; disabled?: boolean }[] = [
  { key: "CN", label: "CN" },
];

interface Props {
  tab: MarketTab;
  onTab: (t: MarketTab) => void;
  onOpenSettings: () => void;
}

export default function TopBar({ tab, onTab, onOpenSettings }: Props) {
  const [pinned, setPinned] = useState(false);
  const win = getCurrentWindow();

  const togglePin = async () => {
    const next = !pinned;
    setPinned(next);
    await win.setAlwaysOnTop(next);
  };

  return (
    <header className="topbar" data-tauri-drag-region>
      <div className="logo" data-tauri-drag-region>
        <span className="logo-mark">T</span>
        TICK
      </div>
      <nav className="tabs">
        {TABS.map((t) => (
          <button
            key={t.key}
            className={`tab ${t.key === tab ? "active" : ""} ${t.disabled ? "disabled" : ""}`}
            title={t.disabled ? "加密货币暂未支持" : undefined}
            disabled={t.disabled}
            onClick={() => onTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </nav>
      <div className="win-controls">
        <button
          className={`win-btn ${pinned ? "active" : ""}`}
          title={pinned ? "取消置顶" : "窗口置顶"}
          onClick={togglePin}
        >
          📌
        </button>
        <button className="win-btn" title="设置" onClick={onOpenSettings}>
          ⚙
        </button>
        <button className="win-btn" title="最小化" onClick={() => win.minimize()}>
          —
        </button>
        <button className="win-btn close" title="关闭" onClick={() => win.close()}>
          ✕
        </button>
      </div>
    </header>
  );
}
