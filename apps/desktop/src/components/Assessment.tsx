import { useEffect, useRef, useState } from "react";
import { Markdown } from "./Markdown.ts";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import type { Assessment, AttemptView, MarkdownView } from "../types.ts";

export function AssessmentView({
  assessment,
  stimulus,
  attempt,
  busy,
  onAnswer,
  onBack,
  onNext,
  onShowProgress,
  hasNext,
  position,
  total,
}: {
  assessment: Assessment;
  stimulus: MarkdownView | null;
  attempt: AttemptView | null;
  busy: boolean;
  onAnswer: (response: string | boolean) => void;
  onBack: () => void;
  onNext: () => void;
  onShowProgress: () => void;
  hasNext: boolean;
  position: number;
  total: number;
}) {
  const [selected, setSelected] = useState<string | null>(null);
  const feedbackHeading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (attempt !== null) feedbackHeading.current?.focus();
  }, [attempt]);
  const answered = attempt !== null;
  const options =
    assessment.response.type === "single_select"
      ? assessment.response.options
      : [
          { id: "true", text: "はい" },
          { id: "false", text: "いいえ" },
        ];
  return (
    <article className="assessment" aria-labelledby="assessment-heading">
      <div className="panel-head">
        <button className="ghost" onClick={onBack} disabled={busy}>
          ← 目次へ
        </button>
        <p className="meta">
          問題 {position} / {total}
        </p>
      </div>
      <div className="question-surface">
        <p className="eyebrow">PRACTICE</p>
        <h1 id="assessment-heading">問題で確かめる</h1>
        <div className="stimulus">
          {stimulus === null ? (
            <p className="empty">問題文を読み込めませんでした。</p>
          ) : (
            <Markdown content={stimulus.content} />
          )}
        </div>
        <form
          className="response"
          onSubmit={(event) => {
            event.preventDefault();
            if (selected !== null && !answered && !busy)
              onAnswer(
                assessment.response.type === "boolean"
                  ? selected === "true"
                  : selected,
              );
          }}
        >
          <fieldset disabled={answered || busy}>
            <legend>回答をひとつ選んでください</legend>
            {options.map((option, index) => (
              <label
                className={`option${selected === option.id ? " selected" : ""}`}
                key={option.id}
              >
                <input
                  type="radio"
                  name="response"
                  value={option.id}
                  checked={selected === option.id}
                  onChange={() => setSelected(option.id)}
                />
                <span className="option-number" aria-hidden="true">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <span className="option-text">{option.text}</span>
                {selected === option.id ? (
                  <span className="selection-label">選択中</span>
                ) : null}
              </label>
            ))}
          </fieldset>
          {!answered ? (
            <div className="answer-action">
              <button
                className="primary"
                type="submit"
                disabled={selected === null || busy || stimulus === null}
              >
                {busy ? "採点中…" : "採点する"}
              </button>
              <span className="meta" role="status">
                {selected === null
                  ? "選択してから採点できます"
                  : busy
                    ? "回答を保存しています"
                    : "選択した回答を送信します"}
              </span>
            </div>
          ) : null}
        </form>
        {attempt !== null ? (
          <section
            className={
              attempt.correct ? "feedback correct" : "feedback incorrect"
            }
            aria-live="polite"
            aria-labelledby="feedback-heading"
          >
            <h2 id="feedback-heading" ref={feedbackHeading} tabIndex={-1}>
              {attempt.correct ? "正解" : "不正解"}
            </h2>
            <Markdown content={attempt.feedback_content.content} />
            <p>
              {attempt.replayed
                ? "記録済みの回答を表示しています。"
                : "回答を保存しました。"}
            </p>
            <button
              className="primary"
              disabled={busy}
              onClick={hasNext ? onNext : onShowProgress}
            >
              {hasNext ? "次の問題 →" : "進捗を確認する →"}
            </button>
          </section>
        ) : null}
        <DeveloperDetails>
          <dl>
            <dt>Assessment ID</dt>
            <dd>{assessment.id}</dd>
            <dt>Revision</dt>
            <dd>{assessment.revision}</dd>
            <dt>Measures</dt>
            <dd>{assessment.measures.join(", ")}</dd>
            {attempt ? (
              <>
                <dt>Score</dt>
                <dd>{attempt.score}</dd>
                <dt>Evaluator</dt>
                <dd>
                  {attempt.evaluator.id} v{attempt.evaluator.version}
                </dd>
                <dt>Event ID</dt>
                <dd>{attempt.event.event_id}</dd>
                <dt>Timestamp</dt>
                <dd>{attempt.event.timestamp}</dd>
              </>
            ) : null}
          </dl>
        </DeveloperDetails>
      </div>
    </article>
  );
}
