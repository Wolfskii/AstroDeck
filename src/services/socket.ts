/**
 * WebSocket client stub for future mobile companion support.
 *
 * When implemented, this module will:
 * - Connect to the AstroDeck backend WS server
 * - Receive scene state updates
 * - Forward button actions from remote clients
 */

export function connectSocket(_url?: string): void {
  // Future: establish WebSocket connection to backend
}

export function sendAction(_action: string): void {
  // Future: send action over WebSocket to backend
}

export function disconnectSocket(): void {
  // Future: close WebSocket connection
}
