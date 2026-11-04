// Vocelle website motion. GSAP (ScrollTrigger, SplitText, Flip, MotionPath,
// DrawSVG) and Lenis, all self-hosted. Every scene explains the product:
// hold a key, talk, let go, it's typed. Without JavaScript, or with reduced
// motion, the page shows each scene's end state.

const REPO = "codewithdark-git/vocelle";
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => Array.from(root.querySelectorAll(selector));
const hasGsap = typeof window.gsap !== "undefined" && typeof window.ScrollTrigger !== "undefined";

// ---------- Cleanup, the same rules as the app ----------

const FILLERS = new Set(["um", "uh", "er", "erm", "hmm", "mm", "ah"]);
const REPLACEMENTS = [[/\bdr\.? okafor\b/gi, "Dr. Okafor"]];

function clean(text, level) {
  let out = text;
  for (const [pattern, to] of REPLACEMENTS) out = out.replace(pattern, to);
  if (level === "off") return out;

  // Fillers, with the commas around them: "works, uh, for" -> "works for".
  const words = [];
  for (const token of out.split(/\s+/)) {
    if (!FILLERS.has(token.toLowerCase().replace(/[,.?!]/g, ""))) {
      words.push(token);
      continue;
    }
    if (!words.length) continue;
    const end = token.match(/[.?!]+$/);
    const last = words.length - 1;
    if (end) words[last] = words[last].replace(/[,;:]$/, "") + end[0];
    else if (token.endsWith(",")) words[last] = words[last].replace(/,$/, "");
  }
  out = words.join(" ");

  if (level === "full") {
    out = out.replace(/\b(\w+)\s+\1\b/gi, "$1");
    out = out.replace(/\bi\b/g, "I").replace(/\bi'(ll|m|ve|d)\b/g, "I'$1");
    out = out.replace(/([.?!]\s+)([a-z])/g, (_, gap, letter) => gap + letter.toUpperCase());
    if (!/[.?!]$/.test(out)) out += ".";
  }
  return out.replace(/\s{2,}/g, " ").trim().replace(/^./, (c) => c.toUpperCase());
}

// ---------- Downloads: latest release, right file for this system ----------

async function initDownloads() {
  const linux = /Linux/i.test(navigator.userAgent) && !/Android/i.test(navigator.userAgent);
  if (linux) $$(".js-download-label").forEach((el) => (el.textContent = "Download for Linux"));
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return;
    const release = await res.json();
    const asset = (suffix) => release.assets.find((a) => a.name.endsWith(suffix));
    const exe = asset("x64-setup.exe");
    const deb = asset("amd64.deb");
    const appimage = asset(".AppImage");
    $$(".js-version").forEach((el) => (el.textContent = `Version ${release.tag_name.replace(/^v/, "")}`));
    if (exe) $$(".js-size").forEach((el) => (el.textContent = `${(exe.size / 1048576).toFixed(1)} MB`));
    const primary = linux ? deb : exe;
    if (primary) $$(".js-download").forEach((el) => (el.href = primary.browser_download_url));
    if (exe) $$(".js-exe").forEach((el) => (el.href = exe.browser_download_url));
    if (deb) $$(".js-deb").forEach((el) => (el.href = deb.browser_download_url));
    if (appimage) $$(".js-appimage").forEach((el) => (el.href = appimage.browser_download_url));
  } catch {
    // Offline or rate-limited: the links still go to the releases page.
  }
}

// ---------- Cleanup demo: removed words collapse out (Flip) ----------

function initCleanup() {
  const said = $(".said");
  const out = $(".js-cleaned");
  const thumb = $(".segmented-thumb");
  if (!said || !out) return;
  const raw = said.lastChild.textContent.trim();

  const render = (level, animate) => {
    const words = clean(raw, level).split(" ");
    const state = animate && hasGsap && window.Flip ? Flip.getState(out.children) : null;
    // Reuse a word element when the same word (case-insensitively) is still
    // there, so Flip can move it instead of replacing it.
    const pool = Array.from(out.children);
    const next = words.map((word, i) => {
      const key = word.toLowerCase().replace(/[^a-z']/g, "");
      const reuse = pool.findIndex((el) => el.dataset.key === key);
      const el = reuse >= 0 ? pool.splice(reuse, 1)[0] : document.createElement("span");
      el.className = "w";
      el.dataset.key = key;
      el.dataset.flipId = `${key}-${i}`;
      el.textContent = word;
      return el;
    });
    const removed = pool;
    out.replaceChildren(...next.flatMap((el, i) => (i ? [document.createTextNode(" "), el] : [el])));
    if (state) {
      removed.forEach((el) => {
        el.style.position = "absolute";
        out.appendChild(el);
      });
      Flip.from(state, {
        duration: 0.7,
        ease: "power3.inOut",
        absolute: true,
        onEnter: (els) => gsap.fromTo(els, { opacity: 0, y: 12 }, { opacity: 1, y: 0, duration: 0.5 }),
      });
      gsap.to(removed, {
        opacity: 0,
        y: -18,
        rotation: () => gsap.utils.random(-12, 12),
        duration: 0.45,
        onComplete: () => removed.forEach((el) => el.remove()),
      });
    }
  };

  $$("input[name=level]").forEach((input, i) =>
    input.addEventListener("change", () => {
      thumb.style.transform = `translateX(${i * 100}%)`;
      render(input.value, !reduceMotion);
    }),
  );
  thumb.style.transform = "translateX(200%)";
  render("full", false);
}

// ---------- Hero demo: hold a key, talk, it's typed ----------

const PHRASES = [
  { app: "Mail", label: "To: Sarah · Subject: Onboarding review", heard: "um, I just finished the the review, uh, and the new flow looks really good" },
  { app: "Chat", label: "#launch · Message the team", heard: "uh, the windows build passed, so we can ship it, um, this afternoon" },
  { app: "Notes", label: "Tuesday · Ideas", heard: "remind me to call the the dentist tomorrow, uh, at ten" },
];

const demo = { listening: false, level: 0 };

function buildKeyboard() {
  const kb = $(".keyboard");
  if (!kb) return;
  const rows = [14, 14, 13, 12];
  const html = rows
    .map((n, r) => {
      const keys = Array.from({ length: n }, (_, i) => `<i class="kb-key${(i === 0 || i === n - 1) && r > 0 ? " wide" : ""}"></i>`);
      return `<div class="kb-row">${keys.join("")}</div>`;
    })
    .join("");
  // Bottom row: modifiers, space, then the talk key (Right Ctrl) in ember.
  const bottom = `<div class="kb-row"><i class="kb-key wide"></i><i class="kb-key"></i><i class="kb-key"></i><i class="kb-key space"></i><i class="kb-key"></i><i class="kb-key talk"></i><i class="kb-key"></i><i class="kb-key"></i></div>`;
  kb.innerHTML = html + bottom;
}

function initDemo() {
  const stage = $("#stage");
  const key = $("#key");
  if (!stage || !key) return;
  const pill = $(".pill", stage);
  const bars = $$(".pill-bars i", stage);
  const typed = $(".js-typed", stage);
  const heard = $(".js-heard", stage);
  const appName = $(".js-app-name", stage);
  const label = $(".js-field-label", stage);
  const talkKey = () => $(".kb-key.talk", stage);

  let index = 0;
  let state = "idle";
  let wordTimer = 0;
  let spoken = 0;
  let autoplay = !reduceMotion;
  let autoTimer = 0;
  let visible = true;
  const phrase = () => PHRASES[index % PHRASES.length];

  const showFrame = () => {
    appName.textContent = phrase().app;
    label.textContent = phrase().label;
  };

  const tickBars = (time) => {
    if (!demo.listening) return;
    bars.forEach((bar, i) => {
      const weight = [0.4, 0.65, 0.9, 1, 0.9, 0.65, 0.4][i];
      const level = 0.35 + 0.65 * Math.abs(Math.sin(time / 140 + i * 1.7) * Math.sin(time / 330 + i));
      bar.style.height = `${4 + Math.round(level * weight * 18)}px`;
    });
    requestAnimationFrame(tickBars);
  };

  function start() {
    if (state !== "idle") return;
    state = "listening";
    demo.listening = true;
    key.classList.add("down");
    talkKey()?.classList.add("down");
    pill.dataset.phase = "listening";
    requestAnimationFrame(tickBars);
    showFrame();
    typed.textContent = "";
    heard.textContent = "";
    spoken = 0;
    const words = phrase().heard.split(" ");
    const speak = () => {
      if (state !== "listening" || spoken >= words.length) return;
      spoken += 1;
      heard.textContent = words.slice(0, spoken).join(" ");
      wordTimer = window.setTimeout(speak, 170 + Math.random() * 110);
    };
    speak();
  }

  function stop() {
    if (state !== "listening") return;
    state = "finishing";
    demo.listening = false;
    key.classList.remove("down");
    talkKey()?.classList.remove("down");
    window.clearTimeout(wordTimer);
    if (spoken < 3) {
      pill.dataset.phase = "idle";
      heard.textContent = "";
      state = "idle";
      return;
    }
    const said = phrase().heard.split(" ").slice(0, spoken).join(" ");
    pill.dataset.phase = "transcribing";
    window.setTimeout(() => {
      typed.textContent = clean(said, "full");
      pill.dataset.phase = "done";
      window.setTimeout(() => {
        pill.dataset.phase = "idle";
        heard.textContent = "";
        state = "idle";
        index += 1;
        schedule();
      }, 900);
    }, 450);
  }

  function schedule(delay = 2400) {
    window.clearTimeout(autoTimer);
    if (!autoplay) return;
    autoTimer = window.setTimeout(() => {
      if (!autoplay || state !== "idle" || !visible) return schedule(1500);
      start();
      window.setTimeout(() => autoplay && stop(), phrase().heard.split(" ").length * 230 + 400);
    }, delay);
  }

  const takeOver = () => {
    autoplay = false;
    window.clearTimeout(autoTimer);
  };

  new IntersectionObserver(([entry]) => (visible = entry.isIntersecting)).observe(stage);

  key.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    key.setPointerCapture(event.pointerId);
    takeOver();
    start();
  });
  key.addEventListener("pointerup", stop);
  key.addEventListener("pointercancel", stop);
  key.addEventListener("keydown", (event) => {
    if ((event.key === " " || event.key === "Enter") && !event.repeat) {
      event.preventDefault();
      takeOver();
      start();
    }
  });
  key.addEventListener("keyup", (event) => {
    if (event.key === " " || event.key === "Enter") stop();
  });
  window.addEventListener("keydown", (event) => {
    if (event.code === "ControlRight" && !event.repeat) {
      takeOver();
      start();
    }
  });
  window.addEventListener("keyup", (event) => {
    if (event.code === "ControlRight") stop();
  });

  showFrame();
  if (reduceMotion) typed.textContent = clean(phrase().heard, "full");
  else schedule(2600);
}

// ---------- Hero waveform: an ambient voice line that swells while listening ----------

function initWave() {
  const canvas = $(".hero-wave");
  if (!canvas || reduceMotion) return;
  const ctx = canvas.getContext("2d");
  let w = 0;
  let h = 0;
  let visible = true;
  const resize = () => {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    w = canvas.clientWidth;
    h = canvas.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  };
  resize();
  window.addEventListener("resize", resize);
  new IntersectionObserver(([e]) => (visible = e.isIntersecting)).observe(canvas);

  const lines = [
    { amp: 46, freq: 0.0055, speed: 0.0009, alpha: 0.5, width: 1.6 },
    { amp: 30, freq: 0.008, speed: -0.0013, alpha: 0.3, width: 1.2 },
    { amp: 62, freq: 0.0035, speed: 0.0006, alpha: 0.18, width: 1 },
  ];
  const draw = (time) => {
    requestAnimationFrame(draw);
    if (!visible) return;
    demo.level += ((demo.listening ? 1 : 0) - demo.level) * 0.05;
    ctx.clearRect(0, 0, w, h);
    const mid = h * 0.62;
    for (const line of lines) {
      ctx.beginPath();
      for (let x = 0; x <= w; x += 6) {
        const envelope = Math.sin((x / w) * Math.PI);
        const speech = 1 + demo.level * (1.6 + Math.sin(time * 0.01 + x * 0.02));
        const y =
          mid +
          Math.sin(x * line.freq + time * line.speed) * line.amp * envelope * speech +
          Math.sin(x * line.freq * 2.3 - time * line.speed * 1.7) * line.amp * 0.3 * envelope;
        if (x === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.strokeStyle = `rgba(255, 90, 31, ${line.alpha + demo.level * 0.3})`;
      ctx.lineWidth = line.width;
      ctx.stroke();
    }
  };
  requestAnimationFrame(draw);
}

// ---------- Motion (GSAP) ----------

function initMotion() {
  gsap.registerPlugin(ScrollTrigger, SplitText, Flip, MotionPathPlugin, DrawSVGPlugin);
  document.documentElement.classList.add("motion");

  // Smooth scrolling, driven by GSAP's ticker so scroll scenes stay in sync.
  const lenis = new Lenis({ lerp: 0.1, wheelMultiplier: 1 });
  lenis.on("scroll", ScrollTrigger.update);
  gsap.ticker.add((time) => lenis.raf(time * 1000));
  gsap.ticker.lagSmoothing(0);
  $$('a[href^="#"]').forEach((a) =>
    a.addEventListener("click", (event) => {
      const target = $(a.getAttribute("href"));
      if (!target) return;
      event.preventDefault();
      lenis.scrollTo(target, { offset: a.getAttribute("href") === "#top" ? 0 : -20, duration: 1.4 });
    }),
  );

  initNav();
  initSpeak();
  initReveals();
  initSheets();
  initHero();

  const mm = gsap.matchMedia();
  mm.add("(min-width: 700px)", () => {
    initHow();
    initSpeed();
    initAnywhere();
    initPrivate();
    initScreens();
  });
  mm.add("(max-width: 699px)", () => {
    initHowSimple();
    initSpeedSimple();
    initAnywhereSimple();
    initPrivateSimple();
  });

  initCounters();
  initFaq();
  initCtaDots();
  initWordmark();
  initMagnetic();
  window.addEventListener("load", () => ScrollTrigger.refresh());
}

function initNav() {
  const nav = $("#nav");
  let last = 0;
  ScrollTrigger.create({
    start: 0,
    end: "max",
    onUpdate: (self) => {
      const y = self.scroll();
      nav.classList.toggle("scrolled", y > 20);
      nav.classList.toggle("hidden", y > 400 && y > last);
      last = y;
    },
  });
}

// Words are "spoken" onto the page: each starts condensed and faint and
// widens to full voice, one after another.
function initSpeak() {
  $$("[data-speak]").forEach((el) => {
    const split = new SplitText(el, { type: "words", wordsClass: "word" });
    const tween = {
      fontVariationSettings: "'wdth' 112",
      opacity: 1,
      y: 0,
      duration: 1.1,
      ease: "expo.out",
      stagger: 0.09,
    };
    gsap.set(split.words, { fontVariationSettings: "'wdth' 50", opacity: 0, y: 24 });
    if (el.closest(".hero")) gsap.to(split.words, { ...tween, delay: 0.15 });
    else gsap.to(split.words, { ...tween, scrollTrigger: { trigger: el, start: "top 80%" } });
  });
}

// Headings rise a line at a time from a mask; leads and facts follow softly.
function initReveals() {
  $$("[data-reveal]").forEach((el) => {
    const isHeading = /^H[1-3]$/.test(el.tagName);
    if (isHeading) {
      const split = new SplitText(el, { type: "lines", linesClass: "line", mask: "lines" });
      gsap.from(split.lines, {
        yPercent: 110,
        duration: 1.1,
        ease: "expo.out",
        stagger: 0.1,
        scrollTrigger: { trigger: el, start: "top 85%" },
      });
    } else {
      gsap.from(el, {
        opacity: 0,
        y: 30,
        duration: 1,
        ease: "power3.out",
        scrollTrigger: { trigger: el, start: "top 88%" },
      });
    }
  });
}

// Each section rises over the last like a sheet: it starts slightly smaller
// with rounder corners and settles as it reaches the top.
function initSheets() {
  $$(".sheet").forEach((sheet) => {
    gsap.fromTo(
      sheet,
      { scale: 0.94, borderRadius: "80px 80px 0 0" },
      {
        scale: 1,
        borderRadius: "40px 40px 0 0",
        ease: "none",
        scrollTrigger: { trigger: sheet, start: "top bottom", end: "top 25%", scrub: true },
      },
    );
  });
}

function initHero() {
  const lid = $(".lid");
  const laptop = $(".laptop");
  // The lid opens on load, then all the way, and the laptop comes forward, as you scroll.
  gsap.fromTo(lid, { "--lid": "-82deg" }, { "--lid": "-24deg", duration: 1.8, ease: "expo.out", delay: 0.5 });
  gsap.fromTo(laptop, { y: 80, opacity: 0 }, { y: 0, opacity: 1, duration: 1.4, ease: "expo.out", delay: 0.35 });
  gsap.fromTo(".hero-lead, .hero-actions, .hero-meta", { opacity: 0, y: 24 }, { opacity: 1, y: 0, duration: 1, ease: "power3.out", stagger: 0.08, delay: 0.55 });
  gsap.timeline({
    scrollTrigger: { trigger: ".laptop-scene", start: "top 85%", end: "top 15%", scrub: 0.6 },
  })
    .to(lid, { "--lid": "0deg", ease: "none" }, 0)
    .fromTo(laptop, { scale: 0.9 }, { scale: 1.04, ease: "none" }, 0);
}

// How it works: one pinned scene played by scroll.
function initHow() {
  const section = $(".how");
  const stage = $(".how-stage", section);
  const steps = $$(".how-step", section);
  const num = $(".js-step-num", section);
  const chips = $$(".chip", section);
  const fillers = chips.filter((c) => c.classList.contains("filler"));
  const kept = chips.filter((c) => !c.classList.contains("filler"));
  const words = $$(".how-sentence span[data-word]", section);
  const pill = $(".how-pill", section);
  const bars = $$(".pill-bars i", pill);
  const bigkey = $(".bigkey", section);

  // Positions relative to the stage, ignoring transforms.
  const offset = (el) => {
    let x = 0;
    let y = 0;
    for (let node = el; node && node !== stage; node = node.offsetParent) {
      x += node.offsetLeft;
      y += node.offsetTop;
    }
    return { x: x + el.offsetWidth / 2, y: y + el.offsetHeight / 2 };
  };
  const target = (chip) => words.find((w) => w.dataset.word === chip.dataset.chip);

  let barsOn = false;
  const animateBars = (time) => {
    if (!barsOn) return;
    bars.forEach((bar, i) => {
      bar.style.height = `${4 + Math.round((0.3 + 0.7 * Math.abs(Math.sin(time / 150 + i * 1.9))) * 18)}px`;
    });
    requestAnimationFrame(animateBars);
  };
  const setStep = (i) => {
    steps.forEach((s, j) => s.classList.toggle("is-active", i === j));
    num.textContent = String(i + 1);
  };
  const setPhase = (phase) => {
    if (pill.dataset.phase === phase) return;
    pill.dataset.phase = phase;
    barsOn = phase === "listening";
    if (barsOn) requestAnimationFrame(animateBars);
  };

  const tl = gsap.timeline({
    defaults: { ease: "power2.inOut" },
    scrollTrigger: {
      trigger: section,
      start: "top top",
      end: "+=260%",
      pin: true,
      scrub: 0.8,
      invalidateOnRefresh: true,
      onUpdate: (self) => {
        const p = self.progress;
        setStep(p < 0.22 ? 0 : p < 0.6 ? 1 : 2);
        bigkey.classList.toggle("down", p > 0.06 && p < 0.6);
        setPhase(p < 0.06 ? "idle" : p < 0.6 ? "listening" : p < 0.9 ? "transcribing" : "done");
        fillers.forEach((f) => f.classList.toggle("flagged", p > 0.6));
      },
    },
  });

  // Words rise one by one while you talk.
  tl.fromTo(chips, { opacity: 0, y: 60, scale: 0.8 }, { opacity: 1, y: 0, scale: 1, stagger: 0.06, duration: 0.25, ease: "back.out(1.6)" }, 0.12);
  // Fillers drop away.
  tl.to(fillers, { y: () => stage.offsetHeight * 0.55, rotation: (i) => [-24, 18, -12][i % 3], opacity: 0, stagger: 0.04, duration: 0.3, ease: "power2.in" }, 0.64);
  // The rest travel into the sentence, then become it.
  tl.to(kept, {
    x: (i, chip) => offset(target(chip)).x - offset(chip).x,
    y: (i, chip) => offset(target(chip)).y - offset(chip).y,
    scale: 0.92,
    stagger: 0.03,
    duration: 0.28,
  }, 0.74);
  tl.to(kept, { opacity: 0, duration: 0.06 }, 0.97);
  tl.to(words, { opacity: 1, duration: 0.06 }, 0.95);
}

function initHowSimple() {
  $$(".how-sentence span[data-word]").forEach((w) => (w.style.opacity = 1));
  $$(".how-step").forEach((s) => s.classList.add("is-active"));
  $(".how-chips").style.display = "none";
  gsap.set(".how-step", { position: "static", marginTop: 28 });
  $(".how-steps").style.minHeight = "0";
}

// Speed: the same passage, typed and spoken.
const PASSAGE =
  "Thanks for the update. I'll review the draft this afternoon and send comments by Friday. If the numbers hold, we can share it with the whole team next week and start the rollout in March.";

function initSpeed() {
  const typeEl = $(".js-lane-type");
  const voiceEl = $(".js-lane-voice");
  const typeCount = $(".js-wpm-type");
  const voiceCount = $(".js-wpm-voice");
  const words = PASSAGE.split(" ");
  const render = (p) => {
    const voice = Math.round(p * words.length);
    const type = Math.round(p * words.length * (40 / 150));
    voiceEl.innerHTML = `${words.slice(0, voice).join(" ")}<span class="typed-caret"></span>`;
    typeEl.innerHTML = `${words.slice(0, type).join(" ")}<span class="typed-caret"></span>`;
    voiceCount.textContent = voice;
    typeCount.textContent = type;
  };
  render(0);
  ScrollTrigger.create({
    trigger: ".speed",
    start: "top top",
    end: "+=140%",
    pin: true,
    scrub: true,
    onUpdate: (self) => render(Math.min(1, self.progress * 1.15)),
  });
}

function initSpeedSimple() {
  const words = PASSAGE.split(" ");
  ScrollTrigger.create({
    trigger: ".race",
    start: "top 75%",
    onEnter: () => {
      const proxy = { p: 0 };
      gsap.to(proxy, {
        p: 1,
        duration: 4,
        ease: "none",
        onUpdate: () => {
          const voice = Math.round(proxy.p * words.length);
          const type = Math.round(proxy.p * words.length * (40 / 150));
          $(".js-lane-voice").textContent = words.slice(0, voice).join(" ");
          $(".js-lane-type").textContent = words.slice(0, type).join(" ");
          $(".js-wpm-voice").textContent = voice;
          $(".js-wpm-type").textContent = type;
        },
      });
    },
    once: true,
  });
}

// Anywhere: app windows travel along a curve; the centre one receives the dictation.
function initAnywhere() {
  const cards = $$(".flow .card");
  const n = cards.length;
  const place = (p) => {
    const centre = p * (n - 1);
    cards.forEach((card, i) => {
      const d = i - centre;
      const abs = Math.abs(d);
      // Depth by light, not transparency: cards further from the centre
      // darken instead of turning grey over the night background.
      gsap.set(card, {
        x: d * Math.min(window.innerWidth * 0.36, 470),
        z: -abs * 240,
        rotationY: d * -26,
        y: abs * 30,
        opacity: abs > 2.6 ? 0 : 1,
        filter: `brightness(${Math.max(0.28, 1 - abs * 0.36)})`,
        zIndex: 100 - Math.round(abs * 10),
      });
      card.classList.toggle("is-center", abs < 0.5);
    });
  };
  place(0);
  ScrollTrigger.create({
    trigger: ".anywhere",
    start: "top top",
    end: "+=220%",
    pin: true,
    scrub: 0.6,
    onUpdate: (self) => place(self.progress),
  });
}

function initAnywhereSimple() {
  const flow = $(".flow");
  flow.style.height = "auto";
  flow.style.display = "grid";
  flow.style.gap = "16px";
  $$(".flow .card").forEach((card) => {
    card.style.position = "relative";
    card.style.left = "auto";
    card.style.margin = "0 auto";
    ScrollTrigger.create({ trigger: card, start: "top 70%", onEnter: () => card.classList.add("is-center"), once: true });
  });
}

// Privacy: the voice reaches this computer; the line to the cloud is cut.
function initPrivate() {
  const toggle = $(".js-airplane");
  gsap.to(".airgap .ring", {
    scale: 2.4,
    opacity: 0,
    duration: 2.4,
    stagger: 0.8,
    repeat: -1,
    ease: "power1.out",
    transformOrigin: "50% 50%",
    startAt: { scale: 1, opacity: 0.8 },
  });
  gsap.to(".airgap .ag-wave", { strokeDashoffset: -60, duration: 1.6, repeat: -1, ease: "none", strokeDasharray: "30 30" });

  const tl = gsap.timeline({
    scrollTrigger: {
      trigger: ".private",
      start: "top top",
      end: "+=130%",
      pin: true,
      scrub: 0.6,
      onUpdate: (self) => {
        if (!toggle.dataset.touched) toggle.checked = self.progress > 0.55;
      },
    },
  });
  tl.from(".airgap .ag-link-in", { drawSVG: "0%", duration: 0.3 })
    .from(".airgap .ag-pc", { opacity: 0.2, scale: 0.92, transformOrigin: "50% 50%", duration: 0.2 }, "<0.1")
    .from(".airgap .ag-link-out", { drawSVG: "0%", duration: 0.2 })
    .to(".airgap .ag-link-out", { opacity: 0, y: 30, duration: 0.2 }, "+=0.1")
    .from(".airgap .ag-cross", { drawSVG: "0%", duration: 0.2 }, "<")
    .to(".airgap .ag-cloud", { opacity: 0.35, duration: 0.2 }, "<");
  toggle.addEventListener("change", () => (toggle.dataset.touched = "1"));
}

function initPrivateSimple() {
  gsap.to(".airgap .ring", { scale: 2.4, opacity: 0, duration: 2.4, stagger: 0.8, repeat: -1, transformOrigin: "50% 50%", startAt: { scale: 1, opacity: 0.8 } });
  $(".js-airplane").checked = true;
}

// App screens stack up as you scroll: each slides over the last.
function initScreens() {
  const cards = $$(".screen-card");
  gsap.set(cards.slice(1), { yPercent: 110 });
  const tl = gsap.timeline({
    scrollTrigger: { trigger: ".screens", start: "top 12%", end: `+=${cards.length * 80}%`, pin: ".app", scrub: 0.7 },
  });
  cards.slice(1).forEach((card, i) => {
    tl.to(card, { yPercent: 0, ease: "power2.out", duration: 1 }, i)
      .to(cards[i], { scale: 0.92, yPercent: -4, filter: "brightness(0.6)", duration: 1 }, i);
  });
}

function initCounters() {
  $$("[data-count]").forEach((el) => {
    const end = parseFloat(el.dataset.count);
    const decimals = parseInt(el.dataset.decimals || "0", 10);
    const proxy = { v: end === 0 ? 1000 : 0 };
    el.textContent = proxy.v.toFixed(decimals);
    ScrollTrigger.create({
      trigger: el,
      start: "top 85%",
      once: true,
      onEnter: () =>
        gsap.to(proxy, {
          v: end,
          duration: 1.6,
          ease: "power3.out",
          onUpdate: () => (el.textContent = (end === 0 ? Math.round(proxy.v) : proxy.v).toFixed(decimals).replace(/^(\d+)\.0$/, "$1")),
        }),
    });
  });
}

// FAQ: answers slide open instead of jumping.
function initFaq() {
  $$(".faq details").forEach((details) => {
    const summary = $("summary", details);
    const answer = $(".faq-a", details);
    summary.addEventListener("click", (event) => {
      event.preventDefault();
      if (details.open) {
        gsap.to(answer, { height: 0, duration: 0.45, ease: "power3.inOut", onComplete: () => (details.open = false) });
      } else {
        details.open = true;
        gsap.fromTo(answer, { height: 0 }, { height: "auto", duration: 0.55, ease: "power3.out" });
      }
    });
  });
}

// Closing moment: sparks stream along a voice line behind the call to action.
function initCtaDots() {
  const box = $(".cta-dots");
  if (!box) return;
  const count = window.innerWidth < 700 ? 24 : 60;
  for (let i = 0; i < count; i++) {
    const dot = document.createElement("i");
    dot.className = "cta-dot";
    box.appendChild(dot);
    gsap.to(dot, {
      duration: 14,
      repeat: -1,
      ease: "none",
      motionPath: { path: "#cta-path", align: "#cta-path", alignOrigin: [0.5, 0.5] },
      opacity: gsap.utils.random(0.4, 1),
    }).progress(i / count);
  }
}

// The footer wordmark widens as you arrive, like a word said out loud.
function initWordmark() {
  gsap.fromTo(
    ".wordmark",
    { fontVariationSettings: "'wdth' 50" },
    { fontVariationSettings: "'wdth' 100", ease: "none", scrollTrigger: { trigger: ".footer", start: "top bottom", end: "bottom bottom", scrub: true } },
  );
}

// Buttons lean towards the pointer.
function initMagnetic() {
  if (!window.matchMedia("(hover: hover)").matches) return;
  $$(".btn, .try-key, .dl").forEach((el) => {
    const strength = el.classList.contains("dl") ? 0.08 : 0.25;
    el.addEventListener("pointermove", (event) => {
      const r = el.getBoundingClientRect();
      gsap.to(el, {
        x: (event.clientX - r.left - r.width / 2) * strength,
        y: (event.clientY - r.top - r.height / 2) * strength,
        duration: 0.4,
        ease: "power3.out",
      });
    });
    el.addEventListener("pointerleave", () => gsap.to(el, { x: 0, y: 0, duration: 0.7, ease: "elastic.out(1, 0.4)" }));
  });
}

// ---------- Start ----------

buildKeyboard();
initDemo();
initWave();
initCleanup();
initDownloads();

if (hasGsap && !reduceMotion) {
  initMotion();
} else {
  // Static page: show every scene's end state.
  document.documentElement.classList.remove("js");
  $$(".how-step").forEach((s) => s.classList.add("is-active"));
  const lid = $(".lid");
  if (lid) lid.style.setProperty("--lid", "0deg");
  const words = PASSAGE.split(" ");
  const voice = $(".js-lane-voice");
  if (voice) {
    voice.textContent = PASSAGE;
    $(".js-lane-type").textContent = words.slice(0, Math.round(words.length * 0.27)).join(" ");
    $(".js-wpm-voice").textContent = words.length;
    $(".js-wpm-type").textContent = Math.round(words.length * 0.27);
  }
  $$(".flow .card").forEach((card) => card.classList.add("is-center"));
  const airplane = $(".js-airplane");
  if (airplane) airplane.checked = true;
}
