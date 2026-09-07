export interface LogEntry {
  id: string;
  at: Date;
  /** blocked = the program refused it. breach = an attack that got through. */
  kind: "owner" | "agent" | "blocked" | "breach" | "error";
  title: string;
  detail?: string;
  errorName?: string;
  errorCode?: number;
  signature?: string;
}

const LABEL: Record<LogEntry["kind"], string> = {
  owner: "owner",
  agent: "agent",
  blocked: "blocked on-chain",
  breach: "attempt",
  error: "error",
};

export function ActivityLog({ entries }: { entries: LogEntry[] }) {
  return (
    <section className="card log">
      <h2>Activity</h2>
      {entries.length === 0 ? (
        <p className="hint">Nothing yet.</p>
      ) : (
        <ol>
          {entries.map((e) => (
            <li key={e.id} className={`entry ${e.kind}`}>
              <div className="entry-head">
                <span className={`tag ${e.kind}`}>{LABEL[e.kind]}</span>
                <span className="title">{e.title}</span>
                <time>{e.at.toLocaleTimeString()}</time>
              </div>
              {e.detail && <p className="detail">{e.detail}</p>}
              {e.errorName && (
                <p className="code mono">
                  {e.errorName}
                  {e.errorCode !== undefined ? ` · ${e.errorCode}` : ""}
                </p>
              )}
              {e.signature && (
                <p className="sig mono" title={e.signature}>
                  {e.signature.slice(0, 16)}…
                </p>
              )}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
