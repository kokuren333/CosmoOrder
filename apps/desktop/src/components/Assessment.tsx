import type { ReactElement } from "react";
import { useState } from "react";
import { Markdown } from "./Markdown.ts";
import type { Assessment, AttemptView, MarkdownView } from "../types.ts";

/** One assessment: stimulus, response control, grading and feedback. */
export function AssessmentView({
  assessment,
  stimulus,
  attempt,
  busy,
  onAnswer,
  onBack,
  onNext,
  onShowProgress,
  onShowHistory,
  hasNext,
}: {
  assessment: Assessment;
  stimulus: MarkdownView | null;
  attempt: AttemptView | null;
  busy: boolean;
  onAnswer: (response: string | boolean) => void;
  onBack: () => void;
  onNext: () => void;
  onShowProgress: () => void;
  onShowHistory: () => void;
  hasNext: boolean;
}): ReactElement {
  const [selected, setSelected] = useState<string | null>(null);
  const answered = attempt !== null;

  return (
    <article className="panel assessment" aria-labelledby="assessment-heading">
      <div className="panel-head">
        <h2 id="assessment-heading">問題</h2>
        <div className="row">
          <button type="button" onClick={onBack}>
            目次へ
          </button>
          <button type="button" aria-label="進捗を表示" onClick={onShowProgress}>
            進捗
          </button>
          <button type="button" aria-label="履歴を表示" onClick={onShowHistory}>
            履歴
          </button>
          <button type="button" onClick={onNext} disabled={!hasNext}>
            次の問題
          </button>
        </div>
      </div>
      <p className="meta">
        assessment <code>{assessment.id}</code> · revision {assessment.revision} · measures{" "}
        {assessment.measures.join(", ")}
      </p>

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
          if (assessment.response.type === "single_select" && selected !== null) {
            onAnswer(selected);
          }
        }}
      >
        <fieldset disabled={answered || busy}>
          <legend>回答</legend>
          {assessment.response.type === "single_select" ? (
            assessment.response.options.map((option) => (
              <label className="option" key={option.id}>
                <input
                  type="radio"
                  name="response"
                  value={option.id}
                  checked={selected === option.id}
                  onChange={() => setSelected(option.id)}
                />
                <span>{option.text}</span>
                <span className="option-id">
                  <code>{option.id}</code>
                </span>
              </label>
            ))
          ) : (
            <div className="row">
              <button type="button" onClick={() => onAnswer(true)}>
                はい (true)
              </button>
              <button type="button" onClick={() => onAnswer(false)}>
                いいえ (false)
              </button>
            </div>
          )}
        </fieldset>
        {assessment.response.type === "single_select" && !answered ? (
          <button type="submit" disabled={selected === null || busy}>
            採点する
          </button>
        ) : null}
      </form>

      {busy ? <p aria-live="polite">採点中…</p> : null}

      {attempt !== null ? (
        <section
          className={attempt.correct ? "feedback correct" : "feedback incorrect"}
          aria-live="polite"
          aria-labelledby="feedback-heading"
        >
          <h3 id="feedback-heading">{attempt.correct ? "正解" : "不正解"}</h3>
          <p>
            score {attempt.score} · evaluator {attempt.evaluator.id} v
            {attempt.evaluator.version}
            {attempt.replayed ? " · 記録済みの回答を再表示" : " · 新しい学習イベントを保存"}
          </p>
          <Markdown content={attempt.feedback_content.content} />
          <p className="meta">
            event <code>{attempt.event.event_id}</code> · {attempt.event.timestamp}
          </p>
        </section>
      ) : null}
    </article>
  );
}
