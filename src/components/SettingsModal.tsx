import { useEffect, useState } from "react";
import { api } from "../api";
import type { Settings, StateSnapshot } from "../types";

interface Props {
  snapshot: StateSnapshot | null;
  onUpdated: (snap: StateSnapshot) => void;
  onClose: () => void;
  onToast: (msg: string) => void;
}

type LoginTab = "easy" | "password";

export default function SettingsModal({ snapshot, onUpdated, onClose, onToast }: Props) {
  const [loginTab, setLoginTab] = useState<LoginTab>("easy");
  const [account, setAccount] = useState("");
  const [passwd, setPasswd] = useState("");
  const [cookieText, setCookieText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [okMsg, setOkMsg] = useState("");
  const [loginWinOpened, setLoginWinOpened] = useState(false);

  const s: Settings = snapshot?.settings ?? { quote_interval_secs: 3, sync_interval_secs: 300 };
  const [quoteSec, setQuoteSec] = useState(s.quote_interval_secs);
  const [syncMin, setSyncMin] = useState(Math.round(s.sync_interval_secs / 60));

  // 自动同步成功时（登录窗口跳转到自选股页触发）关闭弹窗
  useEffect(() => {
    const t = window.setTimeout(() => {
      if (snapshot?.logged_in) {
        setLoginWinOpened(false);
      }
    }, 400);
    return () => window.clearTimeout(t);
  }, [snapshot?.logged_in]);

  const doLogin = async () => {
    if (!account.trim() || !passwd) return setError("请输入手机号和密码");
    setBusy(true); setError(""); setOkMsg("");
    try {
      const snap = await api.loginThs(account.trim(), passwd);
      onUpdated(snap);
      onToast("登录成功，已同步同花顺自选分组");
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doOpenLoginWin = async () => {
    setError(""); setBusy(true);
    try {
      await api.openThsLogin();
      setLoginWinOpened(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doConfirmLogin = async () => {
    setError(""); setBusy(true);
    try {
      const snap = await api.confirmThsLogin();
      onUpdated(snap);
      onToast("登录成功，已同步同花顺自选分组");
      setLoginWinOpened(false);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doImportCookie = async () => {
    if (!cookieText.trim()) return setError("请粘贴 cookie");
    setBusy(true); setError(""); setOkMsg("");
    try {
      const snap = await api.importCookie(cookieText.trim());
      onUpdated(snap);
      onToast("cookie 导入成功，已同步分组");
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doLogout = async () => {
    setBusy(true);
    await api.logoutThs();
    const snap = await api.getState();
    onUpdated(snap);
    setBusy(false);
    onToast("已退出登录");
  };

  const doSaveSettings = async () => {
    setBusy(true);
    try {
      await api.setSettings({
        quote_interval_secs: Math.max(1, Math.round(quoteSec)),
        sync_interval_secs: Math.max(1, Math.round(syncMin)) * 60,
      });
      const snap = await api.getState();
      onUpdated(snap);
      setOkMsg("设置已保存");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-mask" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          设置
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          <div className="tabs" style={{ marginBottom: 16 }}>
            <button className={`tab ${loginTab === "easy" ? "active" : ""}`} onClick={() => setLoginTab("easy")}>
              验证码 / 扫码登录（推荐）
            </button>
            <button className={`tab ${loginTab === "password" ? "active" : ""}`} onClick={() => setLoginTab("password")}>
              密码登录
            </button>
          </div>

          {loginTab === "easy" && (
            <>
              <p className="hint" style={{ marginBottom: 12 }}>
                打开同花顺官方登录页，两种方式任选，无需同花顺密码：<br />
                · <b>短信验证码</b>：输入手机同花顺绑定的手机号收验证码<br />
                · <b>扫码</b>：直接用手机同花顺 App 扫一扫
              </p>
              {!loginWinOpened ? (
                <button className="btn primary" onClick={doOpenLoginWin} disabled={busy}>
                  {busy ? "正在打开登录页…" : "打开登录窗口"}
                </button>
              ) : (
                <>
                  <button className="btn primary" onClick={doConfirmLogin} disabled={busy}>
                    {busy ? "正在同步…" : "✓ 我已完成登录，立即同步"}
                  </button>
                  <button className="btn" onClick={doOpenLoginWin} disabled={busy} style={{ marginLeft: 10 }}>
                    重新打开登录页
                  </button>
                  <p className="hint" style={{ marginTop: 8 }}>
                    登录成功（登录窗口自动跳到自选股页并关闭）后一般无需手动点击；
                    若未自动同步，点上面按钮即可。
                  </p>
                </>
              )}
              <div className="form-error" style={{ marginTop: 8 }}>{error}</div>
              {snapshot?.logged_in && (
                <div style={{ marginTop: 10 }}>
                  <button className="btn" onClick={doLogout} disabled={busy}>退出登录</button>
                </div>
              )}
            </>
          )}

          {loginTab === "password" && (
            <>
              <div className="field">
                <label>同花顺账号（手机号）</label>
                <input className="input" value={account} onChange={(e) => setAccount(e.target.value)} placeholder="13800000000" />
              </div>
              <div className="field">
                <label>密码</label>
                <input className="input" type="password" value={passwd} onChange={(e) => setPasswd(e.target.value)} placeholder="同花顺密码（仅发往同花顺官方服务器）" />
              </div>
              <div className="form-error">{error}</div>
              <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                <button className="btn primary" onClick={doLogin} disabled={busy}>登录并同步</button>
                <button className="link-btn" onClick={() => api.openThsForgot()}>
                  忘记密码？
                </button>
                {snapshot?.logged_in && (
                  <button className="btn" onClick={doLogout} disabled={busy}>退出登录</button>
                )}
              </div>
              <p className="hint" style={{ marginTop: 8 }}>
                手机上用券商账户登录、没有同花顺密码？请用「验证码 / 扫码登录」，
                或点「忘记密码」用手机号验证码重置。
              </p>
            </>
          )}

          <hr className="divider" />

          <div className="field">
            <label>手动导入 Cookie（高级，其他登录方式失效时的兜底）</label>
            <textarea
              className="input"
              rows={3}
              value={cookieText}
              onChange={(e) => setCookieText(e.target.value)}
              placeholder="浏览器登录 stock.10jqka.com.cn 后，F12 → Network → 任意请求 → Cookie，整段粘贴"
            />
          </div>
          <button className="btn" onClick={doImportCookie} disabled={busy}>导入 Cookie</button>

          <hr className="divider" />

          <div style={{ display: "flex", gap: 16, alignItems: "flex-end" }}>
            <div className="field" style={{ marginBottom: 0 }}>
              <label>行情刷新间隔（秒）</label>
              <input className="input" style={{ width: 100 }} type="number" min={1} value={quoteSec} onChange={(e) => setQuoteSec(+e.target.value)} />
            </div>
            <div className="field" style={{ marginBottom: 0 }}>
              <label>分组同步间隔（分钟）</label>
              <input className="input" style={{ width: 100 }} type="number" min={1} value={syncMin} onChange={(e) => setSyncMin(+e.target.value)} />
            </div>
            <button className="btn" onClick={doSaveSettings} disabled={busy}>保存</button>
          </div>
          <div className="form-ok" style={{ marginTop: 10 }}>{okMsg}</div>
        </div>
      </div>
    </div>
  );
}
