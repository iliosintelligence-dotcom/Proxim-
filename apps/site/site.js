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
