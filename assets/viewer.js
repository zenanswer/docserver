(function () {
  "use strict";

  var stage = document.getElementById("stage");
  var pic = document.getElementById("pic");
  var scaleEl = document.getElementById("scale");
  if (!stage || !pic) return;

  var MIN = 0.05;
  var MAX = 12;
  var scale = 1;
  var tx = 0;
  var ty = 0;
  var fitScale = 1;
  var fitMode = true;
  var dragging = false;
  var lastX = 0;
  var lastY = 0;

  function clamp(v, a, b) {
    return Math.max(a, Math.min(b, v));
  }

  function apply() {
    pic.style.transform =
      "translate(" + tx + "px," + ty + "px) scale(" + scale + ")";
    if (scaleEl) scaleEl.textContent = Math.round(scale * 100) + "%";
  }

  function fit() {
    var sw = stage.clientWidth;
    var sh = stage.clientHeight;
    var iw = pic.naturalWidth || 1;
    var ih = pic.naturalHeight || 1;
    var k = Math.min((sw - 24) / iw, (sh - 24) / ih);
    fitScale = k > 1 ? 1 : k;
    if (!(fitScale > 0)) fitScale = 1;
    scale = fitScale;
    tx = (sw - iw * scale) / 2;
    ty = (sh - ih * scale) / 2;
    fitMode = true;
    apply();
  }

  function zoomAt(cx, cy, factor) {
    var ns = clamp(scale * factor, MIN, MAX);
    var k = ns / scale;
    tx = cx - k * (cx - tx);
    ty = cy - k * (cy - ty);
    scale = ns;
    fitMode = false;
    apply();
  }

  function centerZoom(factor) {
    zoomAt(stage.clientWidth / 2, stage.clientHeight / 2, factor);
  }

  function actualSize() {
    var sw = stage.clientWidth;
    var sh = stage.clientHeight;
    var iw = pic.naturalWidth || 1;
    var ih = pic.naturalHeight || 1;
    scale = 1;
    tx = (sw - iw) / 2;
    ty = (sh - ih) / 2;
    fitMode = false;
    apply();
  }

  stage.addEventListener(
    "wheel",
    function (e) {
      e.preventDefault();
      var rect = stage.getBoundingClientRect();
      var factor = Math.exp(-e.deltaY * 0.0016);
      zoomAt(e.clientX - rect.left, e.clientY - rect.top, factor);
    },
    { passive: false }
  );

  stage.addEventListener("pointerdown", function (e) {
    if (e.button !== 0) return;
    dragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
    stage.classList.add("dragging");
    stage.setPointerCapture(e.pointerId);
  });

  stage.addEventListener("pointermove", function (e) {
    if (!dragging) return;
    tx += e.clientX - lastX;
    ty += e.clientY - lastY;
    lastX = e.clientX;
    lastY = e.clientY;
    fitMode = false;
    apply();
  });

  function endDrag(e) {
    dragging = false;
    stage.classList.remove("dragging");
    if (e && e.pointerId !== undefined && stage.hasPointerCapture(e.pointerId)) {
      stage.releasePointerCapture(e.pointerId);
    }
  }

  stage.addEventListener("pointerup", endDrag);
  stage.addEventListener("pointercancel", endDrag);

  stage.addEventListener("dblclick", function () {
    if (Math.abs(scale - fitScale) < 0.001 && fitMode) actualSize();
    else fit();
  });

  document.querySelectorAll("[data-zoom]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var mode = btn.getAttribute("data-zoom");
      if (mode === "in") centerZoom(1.25);
      else if (mode === "out") centerZoom(1 / 1.25);
      else if (mode === "fit") fit();
      else if (mode === "one") actualSize();
    });
  });

  document.addEventListener("keydown", function (e) {
    var tag = (e.target && e.target.tagName) || "";
    if (tag === "INPUT" || tag === "TEXTAREA") return;
    if (e.key === "+" || e.key === "=") {
      centerZoom(1.25);
      e.preventDefault();
    } else if (e.key === "-") {
      centerZoom(1 / 1.25);
      e.preventDefault();
    } else if (e.key === "0") {
      fit();
      e.preventDefault();
    } else if (e.key === "1") {
      actualSize();
      e.preventDefault();
    } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      var sel =
        e.key === "ArrowLeft"
          ? 'a.nav-btn[title^="上一张"]'
          : 'a.nav-btn[title^="下一张"]';
      var link = document.querySelector(sel);
      if (link) window.location.href = link.getAttribute("href");
    }
  });

  var resizeTimer = null;
  window.addEventListener("resize", function () {
    if (!fitMode) return;
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(fit, 80);
  });

  if (pic.complete) fit();
  else pic.addEventListener("load", fit);
})();