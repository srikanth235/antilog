import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Markdown } from "./markdown";

const html = (text: string) => renderToStaticMarkup(createElement(Markdown, { text }));

describe("markdown", () => {
  it("renders a bulleted list with inline code", () => {
    expect(html("I can:\n- run `seal`\n- run `measure`")).toBe(
      "<p>I can:</p><ul><li>run <code>seal</code></li><li>run <code>measure</code></li></ul>",
    );
  });
  it("renders ordered lists, paragraphs and line breaks", () => {
    expect(html("1. one\n2. two\n\nfirst\nsecond")).toBe(
      "<ol><li>one</li><li>two</li></ol><p>first<br/>second</p>",
    );
  });
  it("renders bold, italic and code blocks", () => {
    expect(html("**b** and *i*\n\n```\na < b\n- x\n```")).toBe(
      "<p><strong>b</strong> and <em>i</em></p><pre><code>a &lt; b\n- x</code></pre>",
    );
  });
  it("opens links in a new tab safely", () => {
    expect(html("[docs](https://example.com/a)")).toBe(
      '<p><a href="https://example.com/a" target="_blank" rel="noopener noreferrer">docs</a></p>',
    );
  });
  it("never makes a link of a script url", () => {
    const out = html("[x](javascript:alert(1))");
    expect(out).not.toContain("<a");
  });
  it("shows html as text", () => {
    const out = html("<script>alert(1)</script> <img src=x onerror=y>\n\n- <b>hi</b>");
    expect(out).not.toContain("<script");
    expect(out).not.toContain("<img");
    expect(out).not.toContain("<b>");
    expect(out).toContain("&lt;script&gt;alert(1)&lt;/script&gt;");
    expect(out).toContain("&lt;b&gt;hi&lt;/b&gt;");
  });
});
