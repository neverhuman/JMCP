import { Eye, LockKeyhole, PlayCircle, ShieldCheck } from "lucide-react";
import type { PreparedAction } from "../types";

function actionIcon(action: PreparedAction) {
  if (action.safety === "read_only") {
    return Eye;
  }
  if (action.safety === "bounded_auto") {
    return PlayCircle;
  }
  if (action.safety === "approval_required") {
    return ShieldCheck;
  }
  return LockKeyhole;
}

function isActionExecutable(action: PreparedAction): boolean {
  return action.ready && !action.requiresApproval;
}

export function PreparedActionRail({ actions }: { actions: PreparedAction[] }) {
  return (
    <section className="prepared-action-rail" aria-label="Prepared actions">
      {actions.map((action) => {
        const Icon = actionIcon(action);
        const executable = isActionExecutable(action);
        return (
          <button
            className={`prepared-action prepared-action-${action.safety}`}
            disabled={!executable}
            key={action.id}
            title={action.reason}
            type="button"
          >
            <Icon size={16} aria-hidden="true" />
            <span>{action.label}</span>
            <small>{executable ? "ready" : action.safety.replace("_", " ")}</small>
          </button>
        );
      })}
    </section>
  );
}
