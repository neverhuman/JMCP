export function AnswerCaptionStream({ caption }: { caption: string }) {
  return (
    <section className="answer-caption-stream" aria-live="polite" aria-label="Answer caption">
      <span>{caption}</span>
    </section>
  );
}
