interface Props {
  onAdd: () => void;
  onSync: () => void;
  syncing: boolean;
  editMode: boolean;
  onToggleEdit: () => void;
}

export default function Sidebar({ onAdd, onSync, syncing, editMode, onToggleEdit }: Props) {
  return (
    <aside className="sidebar">
      <button className="tool-btn" title="添加自选" onClick={onAdd}>
        ＋
      </button>
      <button
        className={`tool-btn ${syncing ? "spinning" : ""}`}
        title="同步同花顺分组"
        onClick={onSync}
        disabled={syncing}
      >
        ↻
      </button>
      <button
        className={`tool-btn ${editMode ? "active" : ""}`}
        title="编辑本地自选"
        onClick={onToggleEdit}
      >
        ✎
      </button>
    </aside>
  );
}
