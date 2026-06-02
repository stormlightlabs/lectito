(() => {
  const themes = ["light", "rust", "coal", "navy", "ayu"];
  document.documentElement.classList.remove(...themes);
  document.documentElement.classList.add("coal");

  const script = document.currentScript ?? document.querySelector('script[src$="assets/js/theme.js"]');
  const root = script ? new URL("../../../", script.src) : new URL("./", window.location.href);
  const favicon = new URL("assets/images/favicon.svg", root).href;

  for (const rel of ["icon", "shortcut icon"]) {
    let link = document.querySelector(`link[rel="${rel}"]`);

    if (!link) {
      link = document.createElement("link");
      link.rel = rel;
      document.head.appendChild(link);
    }

    link.href = favicon;
  }

  try {
    localStorage.setItem("mdbook-theme", "coal");
  } catch (_) {
    // Ignore storage failures. The CSS still forces the Coal palette.
  }
})();
