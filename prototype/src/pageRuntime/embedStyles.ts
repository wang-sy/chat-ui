export const embedStyles = `
  :where(chat-view){display:block;min-width:0}
  :where(chat-view>.chat-view-frame){display:block;width:100%;height:var(--chat-view-height,var(--chat-view-content-height,360px));border:0;background:transparent}
  chat-view[hidden],chat-view>[hidden]{display:none!important}
  :where(chat-view>.chat-view-empty){display:grid;place-items:center;min-height:120px;box-sizing:border-box;color:#8d96a0;font:13px system-ui;padding:20px;text-align:center}
  [data-chat-fallback]{display:grid;grid-template-columns:minmax(0,1fr);gap:12px;width:100%;box-sizing:border-box;grid-column:1/-1;padding:12px 0 180px}
  [data-entity-id][data-chat-selected=true]{filter:drop-shadow(0 0 3px #3b82f6)}
`;
