import type { ObjectiveProgress } from "../types.ts";
import { ArrowLeft, ChartNoAxesColumnIncreasing } from "lucide-react";
import { DeveloperDetails } from "./DeveloperDetails.tsx";

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
  const totalAttempts = progress.reduce((sum, item) => sum + item.attempts, 0);
  const totalCorrect = progress.reduce((sum, item) => sum + item.correct, 0);
  return (
    <section aria-labelledby="progress-heading">
      <div className="panel-head">
        <div>
          <p className="eyebrow">YOUR PROGRESS</p>
          <h1 id="progress-heading">学びの進捗</h1>
          <p className="lede">これまでの回答を、学習目標ごとに振り返ります。</p>
        </div>
        <button onClick={onBack} disabled={busy}>
          <ArrowLeft size={16} aria-hidden="true" />目次へ
        </button>
      </div>
      <div className="stats">
        <div className="stat">
          <span><ChartNoAxesColumnIncreasing size={16} aria-hidden="true" />回答数（延べ）</span>
          <strong>
            {totalAttempts}
            <small> 件</small>
          </strong>
        </div>
        <div className="stat">
          <span>正答数（延べ）</span>
          <strong>
            {totalCorrect}
            <small> 件</small>
          </strong>
        </div>
      </div>
      <p className="meta">
        学習目標ごとの観測値の合計です。複数の目標に対応する回答は、それぞれに数えられます。
      </p>
      <h2 className="section-heading">学習目標ごとの記録</h2>
      {progress.length === 0 ? (
        <p className="empty card">
          まだ進捗の記録がありません。問題に回答すると、ここで振り返れます。
        </p>
      ) : (
        <ul className="progress-list">
          {progress.map((item) => (
            <li className="card" key={item.objective_id}>
              <h3>{objectives.get(item.objective_id) ?? "学習目標"}</h3>
              <div className="progress-summary">
                <strong className="accuracy">
                  {item.accuracy === null
                    ? "—"
                    : `${Math.round(item.accuracy * 100)}%`}
                  <small> 正答率</small>
                </strong>
                <span>
                  {item.correct}/{item.attempts} 正答
                </span>
              </div>
              {item.accuracy !== null ? (
                <meter
                  min={0}
                  max={1}
                  value={item.accuracy}
                  aria-label={`${objectives.get(item.objective_id) ?? "学習目標"}の正答率`}
                >
                  {Math.round(item.accuracy * 100)}%
                </meter>
              ) : (
                <p className="meta">まだ回答なし</p>
              )}
              <p className="meta">
                最終回答:{" "}
                {item.last_timestamp ? (
                  <time dateTime={item.last_timestamp}>
                    {new Date(item.last_timestamp).toLocaleString("ja-JP")}
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
        保存済みの回答を集計した記録です。習得を保証するものではありません。
      </p>
      <DeveloperDetails label="メンテナンス">
        <p>保存済みイベントから進捗表示を再構築します。</p>
        <button onClick={onRebuild} disabled={busy}>
          イベントから再構築
        </button>
      </DeveloperDetails>
    </section>
  );
}
