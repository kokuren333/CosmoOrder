import type { ReactElement } from "react";
import type { LearningEvent, ObjectiveProgress } from "../types.ts";

/** Progress and history as the store projects them. Nothing is recomputed here. */
export function ProgressPanel({
  progress,
  objectives,
  onRebuild,
  onShowHistory,
  onBack,
  busy,
}: {
  progress: ObjectiveProgress[];
  objectives: Map<string, string>;
  onRebuild: () => void;
  onShowHistory: () => void;
  onBack: () => void;
  busy: boolean;
}): ReactElement {
  const totalAttempts = progress.reduce((sum, item) => sum + item.attempts, 0);
  const totalCorrect = progress.reduce((sum, item) => sum + item.correct, 0);
  return (
    <section className="panel" aria-labelledby="progress-heading">
      <div className="panel-head">
        <h2 id="progress-heading">進捗</h2>
        <div className="row">
          <button type="button" onClick={onBack}>
            目次へ
          </button>
          <button type="button" aria-label="履歴を表示" onClick={onShowHistory}>
            履歴
          </button>
          <button type="button" onClick={onRebuild} disabled={busy}>
            イベントから再構築
          </button>
        </div>
      </div>
      <p className="meta">
        回答 {totalAttempts} 件 · 正答 {totalCorrect} 件
      </p>
      <table className="progress-table">
        <caption>Objectiveごとの観測値</caption>
        <thead>
          <tr>
            <th scope="col">Objective</th>
            <th scope="col">試行</th>
            <th scope="col">正答</th>
            <th scope="col">正答率</th>
            <th scope="col">最終回答</th>
          </tr>
        </thead>
        <tbody>
          {progress.map((item) => (
            <tr key={item.objective_id}>
              <th scope="row">
                {objectives.get(item.objective_id) ?? item.objective_id}
                <br />
                <code>{item.objective_id}</code>
              </th>
              <td>{item.attempts}</td>
              <td>{item.correct}</td>
              <td>{item.accuracy === null ? "—" : `${Math.round(item.accuracy * 100)}%`}</td>
              <td>{item.last_timestamp ?? "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="meta">
        観測値は保存済みイベントの集計です。習得の保証ではありません。
      </p>
    </section>
  );
}

/** Recent learning events, newest first. */
export function HistoryPanel({
  events,
  objectives,
  onLoadMore,
  onShowProgress,
  onBack,
  busy,
}: {
  events: LearningEvent[];
  objectives: Map<string, string>;
  onLoadMore: () => void;
  onShowProgress: () => void;
  onBack: () => void;
  busy: boolean;
}): ReactElement {
  return (
    <section className="panel" aria-labelledby="history-heading">
      <div className="panel-head">
        <h2 id="history-heading">履歴</h2>
        <div className="row">
          <button type="button" onClick={onBack}>
            目次へ
          </button>
          <button type="button" aria-label="進捗を表示" onClick={onShowProgress}>
            進捗
          </button>
          <button type="button" onClick={onLoadMore} disabled={busy}>
            さらに読み込む
          </button>
        </div>
      </div>
      {events.length === 0 ? (
        <p className="empty">まだ学習イベントがありません。</p>
      ) : (
        <ol className="history-list">
          {events.map((event) => (
            <li key={event.event_id}>
              <div className={event.correct ? "badge correct" : "badge incorrect"}>
                {event.correct ? "正解" : "不正解"}
              </div>
              <div>
                <p>
                  <code>{event.assessment_id}</code> · revision {event.assessment_revision} ·
                  score {event.score}
                </p>
                <p className="meta">
                  {event.timestamp} · 回答 {JSON.stringify(event.response)} · package{" "}
                  {event.package_id}@{event.package_version}
                </p>
                <p className="meta">
                  objective:{" "}
                  {event.objective_ids
                    .map((id) => objectives.get(id) ?? id)
                    .join(", ")}
                </p>
              </div>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
