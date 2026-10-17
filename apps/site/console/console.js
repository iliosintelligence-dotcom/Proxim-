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
  var head = $("#f-head"), body = $("#f-body"), radius = $("#f-radius"),
      from = $("#f-from"), to = $("#f-to"), budget = $("#f-budget");

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
    $("#radius-out").textContent = radius.value + " m";
    $("#zone").setAttribute("r", String(20 + radius.value * 0.27));
    if (+to.value <= +from.value) to.value = String(+from.value + 1);
    $("#from-out").textContent = hour(from.value);
    $("#to-out").textContent = hour(to.value);
    $("#budget-out").textContent = naira(budget.value);
    $("#views-out").textContent = (budget.value * 2).toLocaleString("en-US");
    $("#s-head").textContent = head.value;
    $("#s-body").textContent = body.value;
    $("#s-area").textContent = radius.value + " m around Rosa's Café";
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
  [head, body, radius, from, to, budget].forEach(function (el) { el.addEventListener("input", update); });
  $$(".chip[data-day]").forEach(function (b) { b.addEventListener("click", update); });
  update();
  show(0);
})();
