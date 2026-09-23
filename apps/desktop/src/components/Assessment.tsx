import { useEffect, useRef, useState } from "react";
import { ArrowLeft, CircleCheck, CircleX, ArrowRight } from "lucide-react";
import { Markdown } from "./Markdown.ts";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import type { Assessment, AttemptView, ContentView } from "../types.ts";
import { localize, useUiLanguage } from "../i18n.ts";

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
  stimulus: ContentView | null;
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
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const feedbackHeading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (attempt !== null) feedbackHeading.current?.focus();
  }, [attempt]);
  const answered = attempt !== null;
  const options =
    assessment.response.type === "single_select"
      ? assessment.response.options
      : [
          { id: "true", text: tr("はい", "Yes") },
          { id: "false", text: tr("いいえ", "No") },
        ];
  return (
    <article className="assessment" aria-labelledby="assessment-heading">
      <div className="panel-head">
        <button className="ghost" onClick={onBack} disabled={busy}>
          <ArrowLeft size={16} aria-hidden="true" />{tr("目次へ", "Back to contents")}
        </button>
        <p className="meta">
          {tr("問題", "Question")} {position} / {total}
        </p>
      </div>
      <div className="question-surface">
        <p className="eyebrow">PRACTICE</p>
        <h1 id="assessment-heading">{tr("問題で確かめる", "Assessment")}</h1>
        <div className="stimulus">
          {stimulus === null ? (
            <p className="empty">{tr("問題文を読み込めませんでした。", "Could not load the question.")}</p>
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
            <legend>{tr("回答をひとつ選んでください", "Choose one answer")}</legend>
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
                  <span className="selection-label">{tr("選択中", "Selected")}</span>
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
                {busy ? tr("採点中…", "Checking…") : tr("採点する", "Submit answer")}
              </button>
              <span className="meta" role="status">
                {selected === null
                  ? tr("選択してから採点できます", "Choose an answer to continue")
                  : busy
                    ? tr("回答を保存しています", "Saving answer")
                    : tr("選択した回答を送信します", "Submit the selected answer")}
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
              {attempt.correct ? <CircleCheck size={22} aria-hidden="true" /> : <CircleX size={22} aria-hidden="true" />}
              {attempt.correct ? tr("正解", "Correct") : tr("不正解", "Incorrect")}
            </h2>
            <Markdown content={attempt.feedback_content.content} />
            <p>
              {attempt.replayed
                ? tr("記録済みの回答を表示しています。", "Showing the recorded answer.")
                : tr("回答を保存しました。", "Answer saved.")}
            </p>
            <button
              className="primary"
              disabled={busy}
              onClick={hasNext ? onNext : onShowProgress}
            >
              {hasNext ? <>{tr("次の問題", "Next question")} <ArrowRight size={16} aria-hidden="true" /></> : <>{tr("進捗を確認する", "View progress")} <ArrowRight size={16} aria-hidden="true" /></>}
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
