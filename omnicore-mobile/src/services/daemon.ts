/**
 * OmniCore daemon client (mobile).
 *
 * Speaks the engine daemon's JSON-RPC over WebSocket (omnicore-engine
 * src/server.rs DaemonRequest/DaemonResponse). The engine daemon currently
 * exposes session/RPC logic; when no socket answers, callers fall back to
 * local state and this client reports `connected === false` honestly
 * instead of pretending messages were delivered by the engine.
 */
export type DaemonEventHandler = (eventType: string, payload: unknown) => void;

interface DaemonEnvelope {
  // Engine DaemonResponse is serde-tagged: { status: "Ok"|"Error"|"EventNotification", data: ... }
  status?: string;
  data?: { message?: string; details?: unknown; event_type?: string; payload?: unknown };
  event_type?: string;
  payload?: unknown;
}

class DaemonClient {
  private socket: WebSocket | null = null;
  private endpoint = 'ws://127.0.0.1:8765';
  connected = false;
  private handlers: DaemonEventHandler[] = [];

  setEndpoint(url: string) {
    this.endpoint = url;
  }
  getEndpoint(): string {
    return this.endpoint;
  }
  onEvent(handler: DaemonEventHandler): void {
    this.handlers.push(handler);
  }

  connect(): Promise<boolean> {
    return new Promise((resolve) => {
      try {
        const ws = new WebSocket(this.endpoint);
        const timer = setTimeout(() => resolve(false), 3000);
        ws.onopen = () => {
          clearTimeout(timer);
          this.socket = ws;
          this.connected = true;
          resolve(true);
        };
        ws.onerror = () => {
          clearTimeout(timer);
          this.connected = false;
          resolve(false);
        };
        ws.onclose = () => {
          this.connected = false;
          this.socket = null;
        };
        ws.onmessage = (msg) => {
          try {
            const env = JSON.parse(String(msg.data)) as DaemonEnvelope;
            const eventType = env.event_type ?? (env.status === 'EventNotification' ? env.data?.event_type : undefined);
            const payload = env.payload ?? env.data?.payload;
            if (eventType) {
              this.handlers.forEach((h) => h(eventType, payload));
            }
          } catch {
            // Non-JSON frames are ignored; delivery state is unaffected.
          }
        };
      } catch {
        resolve(false);
      }
    });
  }

  private sendRaw(obj: unknown): boolean {
    if (!this.socket || !this.connected) return false;
    try {
      this.socket.send(JSON.stringify(obj));
      return true;
    } catch {
      return false;
    }
  }

  /** SubscribeEvents RPC: receive events for one channel (or all with null). */
  subscribe(channelId: string | null): boolean {
    return this.sendRaw({ type: 'SubscribeEvents', payload: { channel_id: channelId } });
  }

  /** SendMessage RPC. Returns false when offline so UI can queue locally. */
  sendMessage(recipientId: string, content: string, primaryProtocol: string, fallbackProtocol?: string): boolean {
    return this.sendRaw({
      type: 'SendMessage',
      payload: { recipient_id: recipientId, content, primary_protocol: primaryProtocol, fallback_protocol: fallbackProtocol ?? null },
    });
  }

  disconnect(): void {
    try {
      this.socket?.close();
    } catch {
      // Closing a never-opened socket is a no-op.
    }
    this.socket = null;
    this.connected = false;
  }
}

/** Shared singleton so Settings and Conversation use the same endpoint. */
export const daemon = new DaemonClient();
