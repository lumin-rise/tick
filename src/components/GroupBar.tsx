import type { Group } from "../types";

export const GROUP_ALL = "__all__";
export const GROUP_LOCAL = "__local__";

interface Props {
  groups: Group[];
  localCount: number;
  loggedIn: boolean;
  active: string;
  onSelect: (id: string) => void;
  onLogin: () => void;
}

export default function GroupBar({ groups, localCount, loggedIn, active, onSelect, onLogin }: Props) {
  return (
    <div className="groupbar">
      {!loggedIn && (
        <button className="chip login-hint" onClick={onLogin}>
          ⬇ 登录同花顺，同步手机自选分组
        </button>
      )}
      <button
        className={`chip ${active === GROUP_ALL ? "active" : ""}`}
        onClick={() => onSelect(GROUP_ALL)}
      >
        全部
      </button>
      {loggedIn && (
        <>
          {groups
            .filter((g) => g.id !== "default")
            .map((g) => (
              <button
                key={g.id}
                className={`chip ${active === g.id ? "active" : ""}`}
                onClick={() => onSelect(g.id)}
              >
                {g.name}
                <span className="count">{g.stocks.length}</span>
              </button>
            ))}
          <button
            className={`chip ${active === "default" ? "active" : ""}`}
            onClick={() => onSelect("default")}
          >
            我的自选
            <span className="count">{groups.find((g) => g.id === "default")?.stocks.length ?? 0}</span>
          </button>
          <button
            className={`chip ${active === GROUP_LOCAL ? "active" : ""}`}
            onClick={() => onSelect(GROUP_LOCAL)}
          >
            本地
            <span className="count">{localCount}</span>
          </button>
        </>
      )}
      {!loggedIn && localCount > 0 && (
        <button
          className={`chip ${active === GROUP_LOCAL ? "active" : ""}`}
          onClick={() => onSelect(GROUP_LOCAL)}
        >
          本地
          <span className="count">{localCount}</span>
        </button>
      )}
    </div>
  );
}
