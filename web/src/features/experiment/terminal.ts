// The terminal's wire and the small pure parts of its panel (docs/V0-contracts.md, Terminal).
// On the socket a binary frame is a person's keys (up) or the shell's output (down); a text
// frame is a JSON control message: `resize` up, `exit` down.

/** The part of a WebSocket the link uses, so a test can stand in for one. */
export interface SocketLike {
  binaryType: string;
  readonly readyState: number;
  send(data: string | Uint8Array<ArrayBuffer>): void;
  close(): void;
  onopen: ((event: Event) => void) | null;
  onmessage: ((event: MessageEvent) => void) | null;
  onclose: ((event: CloseEvent) => void) | null;
  onerror: ((event: Event) => void) | null;
}

export const OPEN = 1;

/** Where the browser opens the socket for a terminal it was given a token for. */
export function socketUrl(
  origin: string,
  apiBase: string,
  terminal: string,
  token: string,
): string {
  const url = new URL(`${apiBase}/terminals/${encodeURIComponent(terminal)}/connect`, origin);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.searchParams.set("token", token);
  return url.toString();
}

export function resizeMessage(cols: number, rows: number): string {
  return JSON.stringify({ type: "resize", cols, rows });
}

export type Control = { type: "exit" };

export function parseControl(text: string): Control | undefined {
  try {
    const value: unknown = JSON.parse(text);
    if (
      typeof value === "object" &&
      value !== null &&
      (value as { type?: unknown }).type === "exit"
    )
      return { type: "exit" };
  } catch {
    // Not a control message.
  }
  return undefined;
}

export interface LinkEvents {
  /** Bytes the shell wrote. */
  output(bytes: Uint8Array): void;
  /** The shell ended. */
  exit(): void;
  /** The socket closed, for whatever reason. */
  close(): void;
}

/** One terminal's socket: keys up, output down, resize up, nothing kept. */
export class TerminalLink {
  private readonly socket: SocketLike;
  private readonly queue: (string | Uint8Array<ArrayBuffer>)[] = [];
  private closed = false;

  constructor(socket: SocketLike, events: LinkEvents) {
    this.socket = socket;
    socket.binaryType = "arraybuffer";
    socket.onopen = () => {
      for (const frame of this.queue.splice(0)) socket.send(frame);
    };
    socket.onmessage = (event) => {
      const { data } = event;
      if (typeof data === "string") {
        if (parseControl(data) !== undefined) events.exit();
      } else if (data instanceof ArrayBuffer) {
        events.output(new Uint8Array(data));
      }
    };
    socket.onclose = () => {
      this.closed = true;
      events.close();
    };
    socket.onerror = () => undefined;
  }

  private send(frame: string | Uint8Array<ArrayBuffer>): void {
    if (this.closed) return;
    if (this.socket.readyState === OPEN) this.socket.send(frame);
    else this.queue.push(frame);
  }

  /** What a person typed or pasted. */
  input(text: string): void {
    this.send(new TextEncoder().encode(text));
  }

  resize(cols: number, rows: number): void {
    this.send(resizeMessage(cols, rows));
  }

  close(): void {
    this.closed = true;
    this.socket.close();
  }
}

/**
 * What the shell's title says, `user@machine: ~/path` (bash sets it at every prompt), as the
 * working folder beside the panel's title and the full `user@machine:path` for its tooltip.
 */
export function folderOf(title: string): { label: string; tooltip: string } | undefined {
  const match = /^(\S+@\S+):\s*(.+)$/.exec(title.trim());
  if (match === null) return undefined;
  const [, who, path] = match;
  if (who === undefined || path === undefined) return undefined;
  const tooltip = `${who}:${path}`;
  const parts = path.split("/").filter((part) => part !== "");
  const label = parts.at(-1) ?? (path.startsWith("/") ? "/" : path);
  return { label, tooltip };
}

/** The next tab number: one more than the highest in use, so a closed tab's number is not reused beside a live one. */
export function nextTab(tabs: readonly { n: number }[]): number {
  return tabs.reduce((most, tab) => Math.max(most, tab.n), 0) + 1;
}
