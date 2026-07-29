import type { Stage } from "../api";

export function StagePanel({ stage }: { stage: Stage }) {
  return (
    <div className="stage-panel">
      <h2>{stage.label}</h2>
      {stage.kind === "text" ? (
        <pre className="stage-text">{stage.content}</pre>
      ) : (
        <div className="stage-functions">
          {stage.functions.map((fn) => (
            <div key={fn.name} className={fn.ok ? "function-ok" : "function-error"}>
              <div className="function-name">
                {fn.name} {fn.ok ? "✓" : "✗"}
              </div>
              <pre className="stage-text">{fn.content}</pre>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
