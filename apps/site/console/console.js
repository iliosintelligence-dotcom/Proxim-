// Dashboard previews: sizes come from data-* attributes (the site's CSP forbids inline
// styles), plus the small amount of state the example screens need. Nothing is sent anywhere.
(function () {
  var $ = function (s, r) { return (r || document).querySelector(s); };
  var $$ = function (s, r) { return Array.prototype.slice.call((r || document).querySelectorAll(s)); };

  $$(".bar .col[data-h]").forEach(function (el) { el.style.height = el.dataset.h + "%"; });
  $$(".split-bar [data-w]").forEach(function (el) { el.style.width = el.dataset.w + "%"; });

  // Switches and pressable chips
  $$(".switch-btn").forEach(function (b) {
    b.addEventListener("click", function () { b.setAttribute("aria-checked", b.getAttribute("aria-checked") === "true" ? "false" : "true"); });
  });
  $$(".chip[data-day]").forEach(function (b) {
    b.addEventListener("click", function () { b.setAttribute("aria-pressed", b.getAttribute("aria-pressed") === "true" ? "false" : "true"); });
  });

  // Toolkit code tabs
  $$(".chip[data-code]").forEach(function (b) {
    b.addEventListener("click", function () {
      $$(".chip[data-code]").forEach(function (o) { o.setAttribute("aria-pressed", String(o === b)); });
      $$("pre[data-panel]").forEach(function (p) { p.hidden = p.dataset.panel !== b.dataset.code; });
    });
  });

  // New message flow
  var tabs = $$('[role="tab"][aria-controls]');
  if (!tabs.length) return;
  var step = 0;
  var head = $("#f-head"), body = $("#f-body"),
      from = $("#f-from"), to = $("#f-to"), budget = $("#f-budget");

  // Area: the business draws a shape (a road, an entrance, a junction) instead of a circle.
  var SHAPES = {
    road: { label: "Road outside your café", pts: [[200, 178], [292, 178], [298, 207], [194, 207]] },
    door: { label: "Your front door", pts: [[226, 156], [266, 156], [272, 184], [220, 184]] },
    junction: { label: "The junction by your café", pts: [[185, 168], [210, 168], [222, 180], [222, 205], [210, 217], [185, 217], [173, 205], [173, 180]] }
  };
  var shape = "road", pts = SHAPES.road.pts.slice();
  var svgNS = "http://www.w3.org/2000/svg";
  function shapeLabel() {
    if (shape !== "own") return SHAPES[shape].label;
    return pts.length < 3 ? "Tap the map to add at least 3 corners" : "Your own shape, " + pts.length + " corners";
  }
  function drawShape() {
    $("#zone").setAttribute("points", pts.map(function (p) { return p.join(","); }).join(" "));
    var g = $("#vtx");
    while (g.firstChild) g.removeChild(g.firstChild);
    pts.forEach(function (p) {
      var c = document.createElementNS(svgNS, "circle");
      c.setAttribute("cx", p[0]); c.setAttribute("cy", p[1]); c.setAttribute("r", "6"); c.setAttribute("class", "vtx");
      g.appendChild(c);
    });
    $("#shape-out").textContent = shapeLabel();
  }
  function pick(name) {
    shape = name;
    pts = name === "own" ? [] : SHAPES[name].pts.slice();
    $$(".chip[data-shape]").forEach(function (b) { b.setAttribute("aria-pressed", String(b.dataset.shape === name)); });
    drawShape(); update();
  }
  $$(".chip[data-shape]").forEach(function (b) { b.addEventListener("click", function () { pick(b.dataset.shape); }); });
  $("#clear-shape").addEventListener("click", function () { pick("own"); });
  $("#map svg").addEventListener("click", function (e) {
    var svg = e.currentTarget, pt = svg.createSVGPoint();
    pt.x = e.clientX; pt.y = e.clientY;
    var p = pt.matrixTransform(svg.getScreenCTM().inverse());
    if (shape !== "own") { shape = "own"; pts = []; $$(".chip[data-shape]").forEach(function (b) { b.setAttribute("aria-pressed", String(b.dataset.shape === "own")); }); }
    pts.push([Math.round(p.x), Math.round(p.y)]);
    drawShape(); update();
  });

  function hour(h) { return h + ":00"; }
  // Example price: ₦500 per 1,000 views, so each naira buys two views.
  function naira(v) { return "₦" + (+v).toLocaleString("en-US"); }
  function days() {
    var on = $$(".chip[data-day]").filter(function (b) { return b.getAttribute("aria-pressed") === "true"; })
      .map(function (b) { return b.dataset.day; });
    return on.length ? on.join(", ") : "No days chosen";
  }
  function update() {
    $("#pv-head").textContent = head.value || "Your headline";
    $("#pv-body").textContent = body.value;
    $$(".count").forEach(function (c) {
      var f = document.getElementById(c.dataset.for);
      c.textContent = f.value.length + " / " + f.maxLength;
    });
    if (+to.value <= +from.value) to.value = String(+from.value + 1);
    $("#from-out").textContent = hour(from.value);
    $("#to-out").textContent = hour(to.value);
    $("#budget-out").textContent = naira(budget.value);
    $("#views-out").textContent = (budget.value * 2).toLocaleString("en-US");
    $("#s-head").textContent = head.value;
    $("#s-body").textContent = body.value;
    $("#s-area").textContent = shapeLabel();
    $("#s-when").textContent = days() + ", " + hour(from.value) + " to " + hour(to.value);
    $("#s-budget").textContent = naira(budget.value) + " a week, about " + (budget.value * 2).toLocaleString("en-US") + " views";
  }
  function show(i) {
    step = Math.max(0, Math.min(tabs.length - 1, i));
    tabs.forEach(function (t, k) {
      t.setAttribute("aria-selected", String(k === step));
      document.getElementById(t.getAttribute("aria-controls")).hidden = k !== step;
    });
    $("#prev").disabled = step === 0;
    $("#next").innerHTML = step === tabs.length - 1 ? "Send for review" : 'Next <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 6l6 6-6 6"/></svg>';
    $("#sent-msg").hidden = true;
  }
  tabs.forEach(function (t, k) { t.addEventListener("click", function () { show(k); }); });
  $("#prev").addEventListener("click", function () { show(step - 1); });
  $("#next").addEventListener("click", function () {
    if (step === tabs.length - 1) { $("#sent-msg").hidden = false; return; }
    show(step + 1);
  });
  [head, body, from, to, budget].forEach(function (el) { el.addEventListener("input", update); });
  $$(".chip[data-day]").forEach(function (b) { b.addEventListener("click", update); });
  drawShape();
  update();
  show(0);
})();
