import { executeAction } from "../services/api";
import { logError, logInfo } from "../services/logger";

export async function runDeckAction(
  action: string,
  label: string,
  source: string
): Promise<void> {
  window.dispatchEvent(
    new CustomEvent("astrodeck-action-started", {
      detail: { action, label },
    })
  );
  if (action.startsWith("core.")) {
    window.dispatchEvent(
      new CustomEvent("astrodeck-core-action", {
        detail: { action, label },
      })
    );
  }
  try {
    logInfo(`Clicked ${label} (${action})`, source);
    await executeAction(action);
    window.dispatchEvent(
      new CustomEvent("astrodeck-action-executed", {
        detail: { action, label },
      })
    );
  } catch (e) {
    window.dispatchEvent(
      new CustomEvent("astrodeck-action-failed", {
        detail: { action, label, error: String(e) },
      })
    );
    logError(`Action failed: ${action} (${String(e)})`, source);
  }
}
