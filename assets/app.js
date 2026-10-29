(function () {
  "use strict";

  /* ---------- 主题切换：跟随系统 → 深色 → 浅色 ---------- */
  var root = document.documentElement;
  var toggle = document.getElementById("theme-toggle");
  var order = [null, "dark", "light"];
  var labels = { null: "跟随系统", dark: "深色", light: "浅色" };

  function current() {
    return localStorage.getItem("docserver-theme") || null;
  }

  function apply(theme) {
    if (theme) {
      root.setAttribute("data-theme", theme);
    } else {
      root.removeAttribute("data-theme");
    }
    if (toggle) {
      toggle.title = "主题：" + labels[String(theme)];
      toggle.setAttribute("aria-label", "主题：" + labels[String(theme)]);
    }
  }

  apply(current());

  if (toggle) {
    toggle.addEventListener("click", function () {
      var idx = order.indexOf(current());
      var next = order[(idx + 1) % order.length];
      if (next) {
        localStorage.setItem("docserver-theme", next);
      } else {
        localStorage.removeItem("docserver-theme");
      }
      apply(next);
    });
  }

  /* ---------- 目录过滤 ---------- */
  var filter = document.getElementById("filter");
  if (filter) {
    filter.addEventListener("input", function () {
      var q = filter.value.trim().toLowerCase();
      var rows = document.querySelectorAll("table.listing tbody tr");
      var shown = 0;
      rows.forEach(function (row) {
        if (row.classList.contains("row-parent")) return;
        var name = row.getAttribute("data-name") || "";
        var hit = !q || name.indexOf(q) !== -1;
        row.style.display = hit ? "" : "none";
        if (hit) shown++;
      });
      var count = document.querySelector(".count");
      if (count) count.textContent = shown + " 项";
    });
  }

  /* ---------- 代码块复制按钮 ---------- */
  document.querySelectorAll(".markdown-body pre").forEach(function (pre) {
    var code = pre.querySelector("code");
    if (!code) return;
    var btn = document.createElement("button");
    btn.className = "icon-btn copy";
    btn.type = "button";
    btn.textContent = "复制";
    btn.addEventListener("click", function () {
      navigator.clipboard.writeText(code.innerText).then(function () {
        btn.textContent = "已复制";
        setTimeout(function () {
          btn.textContent = "复制";
        }, 1500);
      });
    });
    pre.appendChild(btn);
  });

  /* ---------- TOC 滚动高亮 ---------- */
  var tocLinks = document.querySelectorAll(".toc a");
  if (tocLinks.length && "IntersectionObserver" in window) {
    var byId = {};
    tocLinks.forEach(function (a) {
      byId[a.getAttribute("href").slice(1)] = a;
    });
    var visible = new Set();
    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) visible.add(entry.target.id);
          else visible.delete(entry.target.id);
        });
        var first = null;
        document.querySelectorAll(".markdown-body h2, .markdown-body h3").forEach(function (h) {
          if (first === null && visible.has(h.id)) first = h.id;
        });
        tocLinks.forEach(function (a) {
          a.classList.toggle("active", a.getAttribute("href").slice(1) === first);
        });
      },
      { rootMargin: "-64px 0px -70% 0px", threshold: 0 }
    );
    document.querySelectorAll(".markdown-body h2, .markdown-body h3").forEach(function (h) {
      if (byId[h.id]) observer.observe(h);
    });
  }
})();