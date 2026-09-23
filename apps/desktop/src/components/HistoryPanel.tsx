import type { LearningEvent, ContentView } from "../types.ts";
import { ArrowLeft, Clock3 } from "lucide-react";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import { localize, useUiLanguage } from "../i18n.ts";

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
  stimuli: Record<string, ContentView>;
  onLoadMore: () => void;
  onBack: () => void;
  busy: boolean;
}) {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  return (
    <section aria-labelledby="history-heading">
      <div className="panel-head">
        <div>
          <p className="eyebrow">LEARNING HISTORY</p>
          <h1 id="history-heading">{tr("履歴", "History")}</h1>
          <p className="lede">{tr("保存された回答記録", "Saved answer records")}</p>
        </div>
        <button onClick={onBack} disabled={busy}>
          <ArrowLeft size={16} aria-hidden="true" />{tr("目次へ", "Back to contents")}
        </button>
      </div>
      {events.length === 0 ? (
        <div className="empty card">
          <h2>{tr("記録はまだありません", "No history yet")}</h2>
          <p>{tr("問題に回答すると、ここに表示されます。", "Answer a question to add a record here.")}</p>
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
                  {event.correct ? tr("正解", "Correct") : tr("不正解", "Incorrect")}
                </span>
                <time className="meta" dateTime={event.timestamp}>
                  <Clock3 size={14} aria-hidden="true" />
                  {new Date(event.timestamp).toLocaleString(language === "ja" ? "ja-JP" : "en-US")}
                </time>
              </div>
              <h2>{stimuli[event.assessment_id]?.text || tr("練習問題", "Practice question")}</h2>
              <p className="meta">
                {event.objective_ids
                  .map((id) => objectives.get(id) ?? tr("学習目標", "Objective"))
                  .join(language === "ja" ? "、" : ", ")}
              </p>
              <DeveloperDetails label={tr("回答の詳細", "Answer details")}>
                <dl>
                  <dt>{tr("回答値", "Response")}</dt>
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
        {tr("さらに読み込む", "Load more")}
      </button>
    </section>
  );
}
