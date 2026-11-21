import { describe, expect, it } from "vitest";
import {
  folderOf,
  nextTab,
  OPEN,
  parseControl,
  resizeMessage,
  type SocketLike,
  socketUrl,
  TerminalLink,
} from "./terminal";

class FakeSocket implements SocketLike {
  binaryType = "blob";
  readyState = 0;
  sent: (string | Uint8Array<ArrayBuffer>)[] = [];
  closedByUs = false;
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  send(data: string | Uint8Array<ArrayBuffer>) {
    this.sent.push(data);
  }
  close() {
    this.closedByUs = true;
  }
  open() {
    this.readyState = OPEN;
    this.onopen?.(new Event("open"));
  }
}

function link() {
  const socket = new FakeSocket();
  const seen = { output: [] as string[], exits: 0, closes: 0 };
  const terminal = new TerminalLink(socket, {
    output: (bytes) => seen.output.push(new TextDecoder().decode(bytes)),
    exit: () => {
      seen.exits += 1;
    },
    close: () => {
      seen.closes += 1;
    },
  });
  return { socket, seen, terminal };
}

describe("a terminal's socket", () => {
  it("carries the shell's bytes to the screen and the keys back as bytes", () => {
    const { socket, seen, terminal } = link();
    socket.open();
    socket.onmessage?.({ data: new TextEncoder().encode("hi\r\n").buffer } as MessageEvent);
    expect(seen.output).toEqual(["hi\r\n"]);
    terminal.input("echo hi\r");
    expect(socket.binaryType).toBe("arraybuffer");
    const [frame] = socket.sent;
    expect(frame).toBeInstanceOf(Uint8Array);
    expect(new TextDecoder().decode(frame as Uint8Array)).toBe("echo hi\r");
  });

  it("holds what was typed before the socket opened and sends it in order", () => {
    const { socket, terminal } = link();
    terminal.input("a");
    terminal.resize(100, 30);
    expect(socket.sent).toEqual([]);
    socket.open();
    expect(socket.sent).toHaveLength(2);
    expect(socket.sent[1]).toBe(resizeMessage(100, 30));
  });

  it("tells the screen when the shell ends and when the socket closes, and sends nothing after", () => {
    const { socket, seen, terminal } = link();
    socket.open();
    socket.onmessage?.({ data: '{"type":"exit"}' } as MessageEvent);
    expect(seen.exits).toBe(1);
    socket.onmessage?.({ data: "not a control message" } as MessageEvent);
    expect(seen.exits).toBe(1);
    socket.onclose?.({} as CloseEvent);
    expect(seen.closes).toBe(1);
    terminal.input("late");
    expect(socket.sent).toEqual([]);
  });

  it("closes its socket when the tab closes", () => {
    const { socket, terminal } = link();
    terminal.close();
    expect(socket.closedByUs).toBe(true);
    terminal.input("after");
    expect(socket.sent).toEqual([]);
  });
});

describe("the terminal's parts", () => {
  it("opens the socket on the page's own origin over ws or wss with the token", () => {
    expect(socketUrl("https://antilog.example", "/api", "t-1", "a b")).toBe(
      "wss://antilog.example/api/terminals/t-1/connect?token=a+b",
    );
    expect(socketUrl("http://127.0.0.1:4173", "/api", "t-1", "x")).toBe(
      "ws://127.0.0.1:4173/api/terminals/t-1/connect?token=x",
    );
  });

  it("reads a control message only when it is one", () => {
    expect(parseControl('{"type":"exit"}')).toEqual({ type: "exit" });
    expect(parseControl('{"type":"other"}')).toBeUndefined();
    expect(parseControl("nope")).toBeUndefined();
  });

  it("shows the working folder from the shell's title, with user@machine:path in the tooltip", () => {
    expect(folderOf("agent@sandbox: ~/acme/checkout")).toEqual({
      label: "checkout",
      tooltip: "agent@sandbox:~/acme/checkout",
    });
    expect(folderOf("root@vm: /")).toEqual({ label: "/", tooltip: "root@vm:/" });
    expect(folderOf("root@vm: ~")).toEqual({ label: "~", tooltip: "root@vm:~" });
    expect(folderOf("vim notes.txt")).toBeUndefined();
  });

  it("numbers a new tab after the highest one open", () => {
    expect(nextTab([])).toBe(1);
    expect(nextTab([{ n: 1 }, { n: 3 }])).toBe(4);
  });
});
