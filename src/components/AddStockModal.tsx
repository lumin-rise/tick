import { useState } from "react";
import { api } from "../api";

interface Props {
  onClose: () => void;
  onAdded: (name: string) => void;
}

export default function AddStockModal({ onClose, onAdded }: Props) {
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const submit = async () => {
    if (!input.trim()) return setError("请输入代码");
    setBusy(true); setError("");
    try {
      const stock = await api.addLocalStock(input.trim());
      onAdded(`${stock.name} 已加入本地自选`);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-mask" onClick={onClose}>
      <div className="modal" style={{ minWidth: 380 }} onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          添加自选
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          <div className="field">
            <label>股票代码</label>
            <input
              className="input"
              autoFocus
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && submit()}
              placeholder="如 600519 / 000001 / 430047 / 00700 / AAPL"
            />
          </div>
          <div className="form-error">{error}</div>
          <p className="hint">
            自动识别市场（沪/深/北/港/美），仅加入本地自选，不会写回同花顺。
          </p>
          <div style={{ marginTop: 12, display: "flex", justifyContent: "flex-end" }}>
            <button className="btn primary" onClick={submit} disabled={busy}>添加</button>
          </div>
        </div>
      </div>
    </div>
  );
}
