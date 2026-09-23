import type { ObjectiveProgress } from "../types.ts";
import { ArrowLeft, ChartNoAxesColumnIncreasing } from "lucide-react";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import { localize, useUiLanguage } from "../i18n.ts";

export function ProgressPanel({
  progress,
  objectives,
  onRebuild,
  onBack,
  busy,
}: {
  progress: ObjectiveProgress[];
  objectives: Map<string, string>;
  onRebuild: () => void;
  onBack: () => void;
  busy: boolean;
}) {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const totalAttempts = progress.reduce((sum, item) => sum + item.attempts, 0);
  const totalCorrect = progress.reduce((sum, item) => sum + item.correct, 0);
  return (
    <section aria-labelledby="progress-heading">
      <div className="panel-head">
        <div>
          <p className="eyebrow">{tr("学習記録", "YOUR PROGRESS")}</p>
          <h1 id="progress-heading">{tr("進捗", "Progress")}</h1>
          <p className="lede">{tr("学習目標ごとの回答記録です。", "Answer records by objective.")}</p>
        </div>
        <button onClick={onBack} disabled={busy}>
          <ArrowLeft size={16} aria-hidden="true" />{tr("目次へ", "Back to contents")}
        </button>
      </div>
      <div className="stats">
        <div className="stat">
          <span><ChartNoAxesColumnIncreasing size={16} aria-hidden="true" />{tr("回答数（延べ）", "Attempts (total)")}</span>
          <strong>
            {totalAttempts}
            <small> {tr("件", "items")}</small>
          </strong>
        </div>
        <div className="stat">
          <span>{tr("正答数（延べ）", "Correct (total)")}</span>
          <strong>
            {totalCorrect}
            <small> {tr("件", "items")}</small>
          </strong>
        </div>
      </div>
      <p className="meta">
        {tr("複数の学習目標に対応する回答は、それぞれに集計されます。", "Attempts measuring multiple objectives are counted for each objective.")}
      </p>
      <h2 className="section-heading">{tr("学習目標ごとの記録", "Records by objective")}</h2>
      {progress.length === 0 ? (
        <p className="empty card">
          {tr("記録はまだありません。問題に回答すると、ここに表示されます。", "No records yet. Answer a question to see progress here.")}
        </p>
      ) : (
        <ul className="progress-list">
          {progress.map((item) => (
            <li className="card" key={item.objective_id}>
              <h3>{objectives.get(item.objective_id) ?? tr("学習目標", "Objective")}</h3>
              <div className="progress-summary">
                <strong className="accuracy">
                  {item.accuracy === null
                    ? "—"
                    : `${Math.round(item.accuracy * 100)}%`}
                  <small> {tr("正答率", "accuracy")}</small>
                </strong>
                <span>
                  {item.correct}/{item.attempts} {tr("正答", "correct")}
                </span>
              </div>
              {item.accuracy !== null ? (
                <meter
                  min={0}
                  max={1}
                  value={item.accuracy}
                  aria-label={tr(`${objectives.get(item.objective_id) ?? "学習目標"}の正答率`, `Accuracy for ${objectives.get(item.objective_id) ?? "Objective"}`)}
                >
                  {Math.round(item.accuracy * 100)}%
                </meter>
              ) : (
                <p className="meta">{tr("まだ回答なし", "No attempts")}</p>
              )}
              <p className="meta">
                {tr("最終回答:", "Last answer:")}{" "}
                {item.last_timestamp ? (
                  <time dateTime={item.last_timestamp}>
                    {new Date(item.last_timestamp).toLocaleString(language === "ja" ? "ja-JP" : "en-US")}
                  </time>
                ) : (
                  "—"
                )}
              </p>
              <DeveloperDetails>
                <p>Objective ID: {item.objective_id}</p>
                <p>Last score: {item.last_score ?? "—"}</p>
              </DeveloperDetails>
            </li>
          ))}
        </ul>
      )}
      <p className="meta">
        {tr("保存済みの回答記録です。習得を保証するものではありません。", "Observed answer records; they do not guarantee mastery.")}
      </p>
      <DeveloperDetails label={tr("メンテナンス", "Maintenance")}>
        <p>{tr("保存済みEventから進捗を再構築します。", "Rebuild progress from saved events.")}</p>
        <button onClick={onRebuild} disabled={busy}>
          {tr("イベントから再構築", "Rebuild from events")}
        </button>
      </DeveloperDetails>
    </section>
  );
}
