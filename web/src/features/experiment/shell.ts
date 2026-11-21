// One shell on screen: xterm.js drawing a terminal asked of the session's machine, carried by a
// TerminalLink. A class driven from a ref callback, so no effect is needed (stack/typescript).
import { FitAddon } from "@xterm/addon-fit";
import { type ITheme, Terminal } from "@xterm/xterm";
import { API_BASE, ApiFailure, api, unwrap } from "../../api/client";
import { w } from "../../words";
import { folderOf, socketUrl, TerminalLink } from "./terminal";

export type Phase =
  | "opening"
  | "live"
  | "shellEnded"
  | "closed"
  | "notRunning"
  | "resuming"
  | "ended"
  | "readOnly"
  | "failed";

const css = (name: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim();

/** The terminal's colours are the design tokens, read from the page so a theme change follows. */
function theme(): ITheme {
  const ink = css("--ink");
  return {
    background: css("--sunk"),
    foreground: ink,
    cursor: ink,
    cursorAccent: css("--sunk"),
    selectionBackground: css("--sel"),
    black: css("--ink3"),
    red: css("--bad"),
    green: css("--good"),
    yellow: css("--warn"),
    blue: css("--infra"),
    magenta: css("--accent"),
    cyan: css("--infra"),
    white: css("--ink2"),
    brightBlack: css("--ink3"),
    brightRed: css("--bad"),
    brightGreen: css("--good"),
    brightYellow: css("--warn"),
    brightBlue: css("--infra"),
    brightMagenta: css("--accent"),
    brightCyan: css("--infra"),
    brightWhite: ink,
  };
}

export interface ShellHooks {
  phase(phase: Phase): void;
  folder(folder: { label: string; tooltip: string } | undefined): void;
  /** Try again: the machine was not up yet and a resume is being waited for. */
  retry(): void;
  /** True while this shell is the one on screen. */
  isActive(): boolean;
}

/** How long, in tries two seconds apart, a resumed session's machine is waited for. */
export const RESUME_TRIES = 45;

export class Shell {
  private readonly screen: Terminal;
  private readonly fit = new FitAddon();
  private link: TerminalLink | undefined;
  private cancelled = false;
  private readonly observers: (() => void)[] = [];

  /** `waiting`: tries left while a resumed machine comes up; zero when nobody is waiting. */
  constructor(
    element: HTMLElement,
    private readonly sessionId: string,
    private readonly hooks: ShellHooks,
    private readonly waiting: { tries: number },
  ) {
    this.screen = new Terminal({
      fontFamily: css("--font-mono"),
      fontSize: 13,
      lineHeight: 1.35,
      cursorBlink: true,
      scrollback: 5000,
      theme: theme(),
    });
    this.screen.loadAddon(this.fit);
    this.screen.open(element);
    this.fit.fit();
    this.screen.onTitleChange((title) => hooks.folder(folderOf(title)));
    const restyle = () => {
      this.screen.options.theme = theme();
    };
    const mutations = new MutationObserver(restyle);
    mutations.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    const scheme = window.matchMedia("(prefers-color-scheme: dark)");
    scheme.addEventListener("change", restyle);
    const resized = new ResizeObserver(() => {
      if (element.offsetParent !== null) this.fit.fit();
    });
    resized.observe(element);
    this.observers.push(
      () => mutations.disconnect(),
      () => scheme.removeEventListener("change", restyle),
      () => resized.disconnect(),
    );
  }

  clear(): void {
    this.screen.clear();
  }

  focus(): void {
    this.screen.focus();
  }

  /** Asks for the terminal and connects to it. */
  async open(): Promise<void> {
    this.hooks.phase(this.waiting.tries > 0 ? "resuming" : "opening");
    try {
      const ticket = unwrap(
        await api.POST("/sessions/{id}/terminals", {
          params: { path: { id: this.sessionId } },
          body: { cols: this.screen.cols, rows: this.screen.rows },
        }),
      );
      if (this.cancelled) return;
      this.waiting.tries = 0;
      this.connect(ticket.terminal, ticket.token);
    } catch (error) {
      if (this.cancelled) return;
      const code = error instanceof ApiFailure ? error.code : "";
      if (code === "not_running") {
        if (this.waiting.tries > 0) {
          this.waiting.tries -= 1;
          setTimeout(() => this.cancelled || this.hooks.retry(), 2000);
          this.hooks.phase("resuming");
        } else this.hooks.phase("notRunning");
      } else if (code === "session_ended") this.hooks.phase("ended");
      else if (error instanceof ApiFailure && error.status === 403) this.hooks.phase("readOnly");
      else this.hooks.phase("failed");
    }
  }

  private connect(terminal: string, token: string): void {
    const socket = new WebSocket(socketUrl(window.location.origin, API_BASE, terminal, token));
    let ended = false;
    let opened = false;
    const link = new TerminalLink(socket, {
      output: (bytes) => this.screen.write(bytes),
      exit: () => {
        ended = true;
        this.screen.write(`\r\n\x1b[2m${w.terminal.shellEnded}\x1b[0m\r\n`);
        this.hooks.phase("shellEnded");
      },
      close: () => {
        if (this.cancelled || ended) return;
        if (opened) this.screen.write(`\r\n\x1b[2m${w.terminal.closed}\x1b[0m\r\n`);
        this.hooks.phase(opened ? "closed" : "failed");
      },
    });
    this.link = link;
    this.screen.onData((data) => link.input(data));
    this.screen.onResize(({ cols, rows }) => link.resize(cols, rows));
    socket.addEventListener(
      "open",
      () => {
        opened = true;
        this.hooks.phase("live");
        if (this.hooks.isActive()) this.screen.focus();
      },
      { once: true },
    );
  }

  /** Closes the socket (the machine removes the shell) and the screen. */
  dispose(): void {
    this.cancelled = true;
    this.link?.close();
    for (const stop of this.observers) stop();
    this.screen.dispose();
  }
}
