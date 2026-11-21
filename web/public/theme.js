// Runs before React: the member's Appearance choice, if any, sets data-theme. With none, the page follows the system.
(() => {
  var mq = window.matchMedia("(prefers-color-scheme: dark)");
  var choice = null;
  try {
    choice = JSON.parse(localStorage.getItem("antilog.appearance.v1") || "null");
  } catch {
    choice = null;
  }
  var dark = choice === "dark" || (choice !== "light" && mq.matches);
  if (dark) document.documentElement.setAttribute("data-theme", "dark");
  else document.documentElement.removeAttribute("data-theme");
  mq.addEventListener("change", (e) => {
    var c = null;
    try {
      c = JSON.parse(localStorage.getItem("antilog.appearance.v1") || "null");
    } catch {
      c = null;
    }
    if (c === "light" || c === "dark") return;
    if (e.matches) document.documentElement.setAttribute("data-theme", "dark");
    else document.documentElement.removeAttribute("data-theme");
  });
})();
