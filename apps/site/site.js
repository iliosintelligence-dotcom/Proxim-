// Hero tokens drop in from above, bounce on the headline words, roll off their edges
// and come to rest in their row. Without JS or with reduced motion they simply sit there.
(function () {
  if (!window.matchMedia || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  var hero = document.querySelector(".hero");
  var tokens = hero ? Array.prototype.slice.call(hero.querySelectorAll(".tokens .token")) : [];
  if (!tokens.length || !Element.prototype.animate) return;
  document.documentElement.classList.add("js-roll");

  var ready = document.fonts && document.fonts.ready ? document.fonts.ready : Promise.resolve();
  ready.then(function () { requestAnimationFrame(run); });

  function clamp(v, lo, hi) { return Math.max(lo, Math.min(hi, v)); }

  function run() {
    var hb = hero.getBoundingClientRect();
    var lines = [];
    hero.querySelectorAll(".headline .w").forEach(function (w) {
      var r = w.getBoundingClientRect();
      var word = { l: r.left - hb.left, r: r.right - hb.left, t: r.top - hb.top + r.height * 0.1 };
      var line = lines.find(function (l) { return Math.abs(l[0].t - word.t) < 12; });
      if (line) line.push(word); else lines.push([word]);
    });
    if (!lines.length) { done(); return; }

    tokens.forEach(function (tok, i) {
      var f = tok.getBoundingClientRect();
      var s = f.width, R = s / 2, deg = 180 / Math.PI / R;
      var F = { x: f.left - hb.left, y: f.top - hb.top };
      var frames = [], t = 0, rot = 0, x, y;

      function at(nx, ny, easing, ms) {
        t += ms; x = nx; y = ny;
        frames.push({ t: t, x: x, y: y, r: rot, easing: easing });
      }
      function fall(nx, ny) {
        var d = Math.max(20, ny - y);
        at(nx, ny, "cubic-bezier(.45,0,1,.55)", clamp(300 * Math.sqrt(d / 300), 140, 620));
      }
      function bounce(h) {
        var ms = clamp(170 * Math.sqrt(h / 50), 90, 220);
        var base = y;
        frames[frames.length - 1].easing = "cubic-bezier(0,.5,.5,1)";
        at(x, base - h, "cubic-bezier(.5,0,1,.5)", ms);
        at(x, base, "cubic-bezier(.45,0,1,.55)", ms);
      }
      function roll(nx, last) {
        var dx = nx - x;
        frames[frames.length - 1].easing = last ? "cubic-bezier(.2,.7,.3,1)" : "cubic-bezier(.4,0,.8,.6)";
        rot += dx * deg;
        at(nx, y, "linear", clamp(Math.abs(dx) * 1.6, 120, 900));
      }

      var la = lines[i % Math.min(2, lines.length)];
      var A = la[(i * 3) % la.length];
      var lb = lines[Math.min(lines.indexOf(la) + 1, lines.length - 1)];
      var B = lb[(i * 5 + 1) % lb.length];

      // Start above the hero, over word A.
      x = A.l + ((i * 0.37) % 1) * Math.max(0, A.r - A.l - s);
      y = -s - 60 - i * 40;
      frames.push({ t: 0, x: x, y: y, r: 0, easing: "linear" });

      fall(x, A.t - s);
      bounce(56);
      var dir = (B.l + B.r) / 2 > x + R ? 1 : -1;
      roll(dir > 0 ? A.r - R : A.l - R);
      if (B !== A) {
        rot += dir * 60;
        fall(clamp(x + dir * s * 0.8, B.l - R, B.r - R), B.t - s);
        bounce(30);
      }
      var dir2 = F.x > x ? 1 : -1;
      roll(dir2 > 0 ? B.r - R : B.l - R);
      rot += dir2 * 60;
      fall(clamp(x + dir2 * s, -R, hb.width - s), F.y);
      bounce(22);
      roll(F.x, true);
      // End upright so the hand-off to the resting state does not jump.
      frames[frames.length - 1].r = Math.round(rot / 360) * 360;

      var total = t;
      var kf = frames.map(function (k) {
        return {
          offset: total ? k.t / total : 1,
          easing: k.easing,
          transform: "translate(" + (k.x - F.x).toFixed(1) + "px," + (k.y - F.y).toFixed(1) + "px) rotate(" + k.r.toFixed(1) + "deg)"
        };
      });
      kf[kf.length - 1].transform = "translate(0px,0px) rotate(" + frames[frames.length - 1].r + "deg)";
      tok.classList.add("go");
      var anim = tok.animate(kf, { duration: total, delay: 250 + i * 320, fill: "backwards" });
      anim.onfinish = function () { tok.classList.add("settled"); };
    });
  }

  function done() { tokens.forEach(function (t) { t.classList.add("go", "settled"); }); }
})();

// Price comparison on the business page. Planning estimates for Nigeria; the Proxim
// price is an example ($0.30 per 1,000 views) until launch. Naira figures
// converted at about ₦1,500 to the dollar.
(function () {
  var calc = document.querySelector("[data-calc]");
  if (!calc) return;
  var input = calc.querySelector("#calc-budget"), out = calc.querySelector("#calc-out");
  function n(v) { return Math.floor(v).toLocaleString("en-US"); }
  function set(k, text) { calc.querySelector('[data-out="' + k + '"]').textContent = text; }
  function update() {
    var b = +input.value;
    out.textContent = "$" + n(b);
    set("proxim", n(b / 0.30 * 1000) + " views");
    set("social", n(b / 3.33 * 1000) + "–" + n(b / 0.67 * 1000) + " views");
    set("google", n(b / 0.61) + " clicks");
    set("flyers", n(b / 0.125) + " flyers");
    set("billboard", b >= 300 ? "About 1 month" : "Not enough");
  }
  input.addEventListener("input", update);
  update();
})();

// Tapping a hero icon splits it into two or three new ones (sports, shopping, school,
// office, gadgets and more), to show the many kinds of business Proxim can carry.
(function () {
  var row = document.querySelector(".hero .tokens");
  var tpl = document.getElementById("more-icons");
  if (!row || !tpl) return;
  var library = Array.prototype.slice.call(tpl.content.querySelectorAll("svg"));
  if (!library.length) return;
  var MAX = 12, VARIANTS = ["", "soft", "dark"];
  var still = !window.matchMedia || matchMedia("(prefers-reduced-motion: reduce)").matches || !Element.prototype.animate;

  function shown() {
    return Array.prototype.map.call(row.querySelectorAll(".token svg[data-name]"), function (s) { return s.getAttribute("data-name"); });
  }
  function pickIcons(n) {
    var used = shown();
    var fresh = library.filter(function (s) { return used.indexOf(s.getAttribute("data-name")) < 0; });
    var pool = fresh.length >= n ? fresh : library.slice();
    var out = [];
    while (out.length < n && pool.length) out.push(pool.splice(Math.floor(Math.random() * pool.length), 1)[0]);
    return out;
  }
  function makeToken(svg) {
    var b = document.createElement("button");
    var v = VARIANTS[Math.floor(Math.random() * VARIANTS.length)];
    b.type = "button";
    b.className = "token go settled" + (v ? " " + v : "");
    b.setAttribute("aria-label", svg.getAttribute("data-name") + ": tap to see more");
    b.appendChild(svg.cloneNode(true));
    return b;
  }

  row.addEventListener("click", function (e) {
    var tok = e.target.closest ? e.target.closest(".token") : null;
    if (!tok || !row.contains(tok)) return;
    var all = Array.prototype.slice.call(row.querySelectorAll(".token"));
    all.forEach(function (t) { t.getAnimations && t.getAnimations().forEach(function (a) {
      if (!window.CSSAnimation || !(a instanceof CSSAnimation)) a.finish();
    }); });

    var n = all.length >= MAX ? 0 : Math.min(2 + Math.floor(Math.random() * 2), MAX - all.length + 1);
    if (n < 2) {
      if (!still) tok.animate([{ transform: "rotate(0)" }, { transform: "rotate(-14deg)" }, { transform: "rotate(12deg)" }, { transform: "rotate(0)" }], { duration: 420, easing: "ease-in-out" });
      return;
    }

    var before = new Map();
    all.forEach(function (t) { before.set(t, t.getBoundingClientRect()); });
    var from = before.get(tok);
    var kids = pickIcons(n).map(makeToken);
    var focusFirst = document.activeElement === tok;
    kids.forEach(function (k) { row.insertBefore(k, tok); });
    row.removeChild(tok);
    if (focusFirst && kids[0]) kids[0].focus();
    if (still) return;

    all.forEach(function (t) {
      if (t === tok || !t.isConnected) return;
      var a = before.get(t), b = t.getBoundingClientRect();
      var dx = a.left - b.left, dy = a.top - b.top;
      if (dx || dy) t.animate([{ transform: "translate(" + dx + "px," + dy + "px)" }, { transform: "translate(0,0)" }], { duration: 380, easing: "cubic-bezier(.2,.8,.3,1)" });
    });
    kids.forEach(function (k, i) {
      var b = k.getBoundingClientRect();
      var dx = from.left - b.left, dy = from.top - b.top;
      var spin = (b.left >= from.left ? 1 : -1) * 360;
      k.animate([
        { transform: "translate(" + dx + "px," + dy + "px) scale(.3) rotate(0deg)", opacity: 0.4 },
        { transform: "translate(0,-18px) scale(1.08) rotate(" + spin * 0.9 + "deg)", opacity: 1, offset: 0.7 },
        { transform: "translate(0,0) scale(1) rotate(" + spin + "deg)" }
      ], { duration: 620, delay: i * 70, easing: "cubic-bezier(.3,.7,.4,1)", fill: "backwards" });
    });
  });
})();
