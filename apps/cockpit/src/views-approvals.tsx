import { History } from "lucide-react";
import type { ApprovalRequest, ReplayEvent } from "./types";
import { EmptyCard, EmptyRow, PanelHeader, classForRisk } from "./views-primitives";

export function ReplayView({ replayEvents }: { replayEvents: ReplayEvent[] }) {
  return (
    <section className="list-panel">
      <PanelHeader icon={History} title="Replay Ledger" meta="JPCM events reconstruct state" />
      <div className="timeline">
        {replayEvents.length === 0 && <EmptyRow label="No replay events" />}
        {replayEvents.map((event) => (
          <article className="timeline-item" key={event.sequence}>
            <span className="sequence">{event.sequence}</span>
            <div>
              <strong>{event.family}</strong>
              <span>{event.subject}</span>
            </div>
            <span>{event.producer}</span>
            <time>{event.timestamp}</time>
          </article>
        ))}
      </div>
    </section>
  );
}

export function ApprovalsView({ approvalRequests }: { approvalRequests: ApprovalRequest[] }) {
  const telegramChallenges = approvalRequests.filter((approval) => approval.channel === "telegram");
  const voiceBound = approvalRequests.filter((approval) => approval.voiceThreadId).length;

  return (
    <div className="approval-backplane">
      <section className="approval-hero">
        <div>
          <p className="eyebrow">Telegram backplane</p>
          <h3>
            {telegramChallenges.length} Telegram challenges, {voiceBound} voice/text confirmations, 1 approval lineage.
          </h3>
          <p>
            The approval surface exposes the token fingerprint, user/chat binding, voice or text confirmation, and work-order lineage instead of hiding it behind a single pending badge.
          </p>
        </div>
        <div className="approval-dial" aria-label={`${voiceBound} bound confirmations`}>
          <strong>{voiceBound}</strong>
          <span>bound confirmations</span>
        </div>
      </section>

      <section className="card-grid approval-grid">
        {approvalRequests.length === 0 && <EmptyCard label="No approvals pending" />}
        {approvalRequests.map((approval) => (
          <article className={`approval-card approval-card-${approval.channel}`} key={approval.id}>
            <div className="system-card-head">
              <strong>{approval.id}</strong>
              <span className={classForRisk(approval.risk)}>{approval.state}</span>
            </div>
            <h3>{approval.decision}</h3>
            <p>{approval.reason}</p>
            <div className="chip-list approval-binding">
              <span className="chip">
                <strong>Approver</strong>
                <small>{approval.approver}</small>
              </span>
              <span className="chip">
                <strong>Token</strong>
                <small>{approval.tokenHash}</small>
              </span>
              <span className="chip">
                <strong>User</strong>
                <small>{approval.targetUserId ?? "unbound"}</small>
              </span>
              <span className="chip">
                <strong>Chat</strong>
                <small>{approval.targetChatId ?? "unbound"}</small>
              </span>
            </div>
            <dl className="approval-meta">
              <div>
                <dt>Work</dt>
                <dd>{approval.workOrderId}</dd>
              </div>
              <div>
                <dt>Task</dt>
                <dd>{approval.currentTask}</dd>
              </div>
              <div>
                <dt>Branch</dt>
                <dd>{approval.branch}</dd>
              </div>
              <div>
                <dt>Pool</dt>
                <dd>{approval.pool}</dd>
              </div>
            </dl>
            <section className="approval-lineage">
              <span className="eyebrow">Lineage</span>
              <div className="chip-list">
                {approval.lineage.map((step) => (
                  <span className="chip" key={step}>
                    {step}
                  </span>
                ))}
              </div>
            </section>
            <section className="approval-evidence">
              <span className="eyebrow">Voice / text</span>
              <p>{approval.voiceTranscript ?? "No voice transcript observed."}</p>
              <div className="chip-list">
                <span className="chip chip-confirmation">
                  <strong>Confirmation</strong>
                  <small>{approval.voiceConfirmationPhrase ?? "none"}</small>
                </span>
                <span className="chip">
                  <strong>Placement</strong>
                  <small>{approval.placement}</small>
                </span>
                <span className="chip">
                  <strong>State</strong>
                  <small>{approval.voiceThreadState ?? "unobserved"}</small>
                </span>
              </div>
            </section>
            <span className="expires">{approval.expires === "expired" ? "Expired" : `Expires in ${approval.expires}`}</span>
          </article>
        ))}
      </section>
    </div>
  );
}
