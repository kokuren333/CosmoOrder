import type { LearningEvent, MarkdownView } from "../types.ts";
import { ArrowLeft, Clock3 } from "lucide-react";
import { DeveloperDetails } from "./DeveloperDetails.tsx";

export function HistoryPanel({
  events,
  objectives,
  stimuli,
  onLoadMore,
  onBack,
  busy,
}: {
  events: LearningEvent[];
  objectives: Map<string, string>;
  stimuli: Record<string, MarkdownView>;
  onLoadMore: () => void;
  onBack: () => void;
  busy: boolean;
}) {
  return (
    <section aria-labelledby="history-heading">
      <div className="panel-head">
        <div>
          <p className="eyebrow">LEARNING HISTORY</p>
          <h1 id="history-heading">学習の履歴</h1>
          <p className="lede">一問ずつ積み重ねた、あなたの学び。</p>
        </div>
        <button onClick={onBack} disabled={busy}>
          <ArrowLeft size={16} aria-hidden="true" />目次へ
        </button>
      </div>
      {events.length === 0 ? (
        <div className="empty card">
          <h2>最初の一問から、記録が始まります</h2>
          <p>問題に回答すると、新しい順にここに並びます。</p>
        </div>
      ) : (
        <ol className="history-list">
          {events.map((event) => (
            <li className="card" key={event.event_id}>
              <div className="history-head">
                <span
                  className={
                    event.correct ? "badge correct" : "badge incorrect"
                  }
                >
                  {event.correct ? "正解" : "不正解"}
                </span>
                <time className="meta" dateTime={event.timestamp}>
                  <Clock3 size={14} aria-hidden="true" />
                  {new Date(event.timestamp).toLocaleString("ja-JP")}
                </time>
              </div>
              <h2>{stimuli[event.assessment_id]?.text || "練習問題"}</h2>
              <p className="meta">
                {event.objective_ids
                  .map((id) => objectives.get(id) ?? "学習目標")
                  .join("、")}
              </p>
              <DeveloperDetails label="回答の詳細">
                <dl>
                  <dt>回答値</dt>
                  <dd>{JSON.stringify(event.response)}</dd>
                  <dt>Package</dt>
                  <dd>
                    {event.package_id}@{event.package_version}
                  </dd>
                  <dt>Assessment ID / Revision</dt>
                  <dd>
                    {event.assessment_id} / {event.assessment_revision}
                  </dd>
                  <dt>Event ID</dt>
                  <dd>{event.event_id}</dd>
                  <dt>Objective IDs</dt>
                  <dd>{event.objective_ids.join(", ")}</dd>
                  <dt>Score</dt>
                  <dd>{event.score}</dd>
                  <dt>Evaluator</dt>
                  <dd>
                    {event.evaluator.id} v{event.evaluator.version}
                  </dd>
                </dl>
              </DeveloperDetails>
            </li>
          ))}
        </ol>
      )}
      <button onClick={onLoadMore} disabled={busy}>
        さらに読み込む
      </button>
    </section>
  );
}
