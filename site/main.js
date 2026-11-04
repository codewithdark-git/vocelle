// Vocelle website behaviour: the hold-to-talk demo, the pinned story, small
// interactive pieces and download links. No libraries; everything degrades
// to a readable static page without JavaScript or with reduced motion.

const REPO = "codewithdark-git/vocelle";
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => Array.from(root.querySelectorAll(selector));

// ---------- Cleanup, the same rules as the app (simplified) ----------

const FILLERS = new Set(["um", "uh", "er", "erm", "hmm", "mm", "ah"]);
const REPLACEMENTS = [[/\bdr\.? okafor\b/gi, "Dr. Okafor"]];

function clean(text, level) {
  let out = text;
  for (const [pattern, to] of REPLACEMENTS) out = out.replace(pattern, to);
  if (level === "off") return out;

  // Fillers, with the commas around them, like the app: "works, uh, for"
  // becomes "works for", and "today, um." becomes "today.".
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
    out = out.replace(/\b(\w+)\s+\1\b/gi, "$1"); // stutters: "the the"
    out = out.replace(/\bi\b/g, "I").replace(/\bi'(ll|m|ve|d)\b/g, "I'$1");
    out = out.replace(/([.?!]\s+)([a-z])/g, (_, gap, letter) => gap + letter.toUpperCase());
    if (!/[.?!]$/.test(out)) out += ".";
  }
  return out.replace(/\s{2,}/g, " ").trim().replace(/^./, (c) => c.toUpperCase());
}

// ---------- Hero: hold a key, talk, it's typed ----------

const PHRASES = [
  {
    app: "Mail",
    label: "To: Sarah · Subject: Onboarding review",
    heard: "um, I just finished the the review, uh, and the new flow looks really good",
  },
  {
    app: "Chat",
    label: "#launch · Message the team",
    heard: "uh, the windows build passed, so we can ship it, um, this afternoon",
  },
  {
    app: "Notes",
    label: "Tuesday · Ideas",
    heard: "remind me to call the the dentist tomorrow, uh, at ten",
  },
];

function initDemo() {
  const stage = $("#stage");
  const key = $("#key");
  if (!stage || !key) return;
  const pill = $(".pill", stage);
  const bars = $$(".pill-bars i", stage);
  const typed = $(".js-typed", stage);
  const heard = $(".js-heard", stage);
  const appName = $(".js-app-name", stage);
  const label = $(".field-label", stage);
  const hero = stage.closest(".hero");

  let index = 0;
  let state = "idle"; // idle | listening | finishing
  let wordTimer = 0;
  let barFrame = 0;
  let spokenWords = 0;
  let autoplay = !reduceMotion;
  let autoTimer = 0;

  const phrase = () => PHRASES[index % PHRASES.length];

  function showPhraseFrame() {
    const p = phrase();
    appName.textContent = p.app;
    label.textContent = p.label;
  }

  function animateBars(time) {
    bars.forEach((bar, i) => {
      const weight = [0.4, 0.65, 0.9, 1, 0.9, 0.65, 0.4][i];
      const level = 0.35 + 0.65 * Math.abs(Math.sin(time / 140 + i * 1.7) * Math.sin(time / 330 + i));
      bar.style.height = `${4 + Math.round(level * weight * 18)}px`;
    });
    barFrame = requestAnimationFrame(animateBars);
  }

  function start() {
    if (state !== "idle") return;
    state = "listening";
    key.classList.add("down");
    hero.classList.add("listening");
    pill.dataset.phase = "listening";
    if (!reduceMotion) barFrame = requestAnimationFrame(animateBars);
    showPhraseFrame();
    typed.textContent = "";
    heard.textContent = "";
    spokenWords = 0;
    const words = phrase().heard.split(" ");
    const speak = () => {
      if (state !== "listening" || spokenWords >= words.length) return;
      spokenWords += 1;
      heard.textContent = words.slice(0, spokenWords).join(" ");
      wordTimer = window.setTimeout(speak, 170 + Math.random() * 110);
    };
    speak();
  }

  function stop() {
    if (state !== "listening") return;
    state = "finishing";
    key.classList.remove("down");
    hero.classList.remove("listening");
    window.clearTimeout(wordTimer);
    cancelAnimationFrame(barFrame);
    const words = phrase().heard.split(" ");
    const said = words.slice(0, Math.max(spokenWords, 1)).join(" ");
    if (spokenWords < 3) {
      // A tap, like in the app: ignored.
      pill.dataset.phase = "idle";
      heard.textContent = "";
      state = "idle";
      return;
    }
    pill.dataset.phase = "transcribing";
    window.setTimeout(() => {
      typed.textContent = clean(said, "full");
      pill.dataset.phase = "done";
      window.setTimeout(() => {
        pill.dataset.phase = "idle";
        heard.textContent = "";
        state = "idle";
        index += 1;
        scheduleAutoplay();
      }, 900);
    }, 450);
  }

  // Autoplay: plays the gesture by itself until someone tries it.
  function scheduleAutoplay(delay = 2200) {
    window.clearTimeout(autoTimer);
    if (!autoplay) return;
    autoTimer = window.setTimeout(() => {
      if (!autoplay || state !== "idle" || !stageVisible) return;
      start();
      const length = phrase().heard.split(" ").length;
      window.setTimeout(() => autoplay && stop(), length * 230 + 400);
    }, delay);
  }

  function takeOver() {
    autoplay = false;
    window.clearTimeout(autoTimer);
  }

  let stageVisible = true;
  new IntersectionObserver(([entry]) => {
    stageVisible = entry.isIntersecting;
    if (stageVisible && state === "idle") scheduleAutoplay(1200);
  }).observe(stage);

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

  showPhraseFrame();
  if (reduceMotion) {
    typed.textContent = clean(phrase().heard, "full");
  } else {
    scheduleAutoplay(1600);
  }
}

// ---------- Pinned story: hold, talk, release ----------

function initStory() {
  const story = $(".story");
  if (!story) return;
  const steps = $$(".story-step", story);
  const pill = $(".story-pill", story);
  const heard = $(".story-heard", story);
  const bars = $$(".pill-bars i", story);

  // Mark the fillers and the stutter so they can be struck through as the
  // text is cleaned. The text is the page's own, not user input.
  heard.innerHTML = heard.textContent
    .split(" ")
    .map((word) => {
      const filler = FILLERS.has(word.toLowerCase().replace(/[,.]/g, ""));
      return filler ? `<span class="filler">${word}</span>` : word;
    })
    .join(" ")
    .replace("the the", 'the <span class="filler">the</span>');

  let frame = 0;
  const animate = (time) => {
    bars.forEach((bar, i) => {
      const level = 0.3 + 0.7 * Math.abs(Math.sin(time / 150 + i * 1.9));
      bar.style.height = `${4 + Math.round(level * 18)}px`;
    });
    frame = requestAnimationFrame(animate);
  };

  const setStep = (step, sub) => {
    story.dataset.step = String(step);
    story.toggleAttribute("data-strike", step === 1 && sub > 0.55);
    steps.forEach((el, i) => el.classList.toggle("active", i === step));
    const phase = step === 2 ? (sub < 0.25 ? "transcribing" : "done") : "listening";
    if (pill.dataset.phase !== phase) {
      pill.dataset.phase = phase;
      cancelAnimationFrame(frame);
      if (phase === "listening" && !reduceMotion) frame = requestAnimationFrame(animate);
    }
  };

  const pinned = () => getComputedStyle($(".story-sticky", story)).position === "sticky";
  const update = () => {
    if (!pinned()) {
      story.dataset.step = "2";
      steps.forEach((el) => el.classList.add("active"));
      return;
    }
    const rect = story.getBoundingClientRect();
    const total = rect.height - window.innerHeight;
    const progress = Math.min(1, Math.max(0, -rect.top / total));
    const scaled = progress * 3;
    const step = Math.min(2, Math.floor(scaled));
    setStep(step, scaled - step);
  };

  window.addEventListener("scroll", update, { passive: true });
  window.addEventListener("resize", update);
  update();
}

// ---------- Anywhere you type ----------

const APPS = [
  {
    name: "Mail",
    html: '<div class="meta">To: Sarah · Re: Onboarding review</div><p>Hi Sarah,</p><p><span class="landing">I just finished the review, and the new flow looks really good. Can we ship it on Thursday?</span></p>',
  },
  {
    name: "Chat",
    html: '<div class="meta">#launch</div><p class="bubble">Is the Windows build ready?</p><p style="clear:both"></p><p class="bubble me"><span class="landing">It passed all checks, so we can ship it this afternoon.</span></p>',
  },
  {
    name: "Document",
    html: '<div class="meta">Q3 plan.docx</div><h3 style="margin:0 0 10px;font:800 22px/1.2 var(--display)">Goals</h3><p><span class="landing">This quarter we focus on onboarding: fewer steps, a clearer first dictation, and a faster model download.</span></p>',
  },
  {
    name: "Code editor",
    html: '<div class="meta">dictation.rs</div><code><span class="comment"><span class="landing">// Transcribe finished phrases while the key is still held,\n// so only the last few seconds are left on release.</span></span>\nfn transcribe_ahead(app: &amp;AppHandle) {</code>',
  },
];

function initAnywhere() {
  const tabs = $$(".app-tabs button");
  const body = $(".js-app-body");
  const chrome = $(".js-chrome");
  if (!body) return;
  const show = (i) => {
    tabs.forEach((tab, j) => tab.setAttribute("aria-selected", String(i === j)));
    chrome.textContent = APPS[i].name;
    body.innerHTML = APPS[i].html;
  };
  tabs.forEach((tab, i) => tab.addEventListener("click", () => show(i)));
  show(0);
}

// ---------- Cleanup levels ----------

function initCleanup() {
  const said = $(".compare-said");
  const out = $(".js-cleaned");
  if (!said || !out) return;
  const raw = said.lastChild.textContent.trim();
  const render = () => {
    const level = $("input[name=level]:checked").value;
    out.style.opacity = "0";
    window.setTimeout(() => {
      out.textContent = clean(raw, level);
      out.style.opacity = "1";
    }, reduceMotion ? 0 : 120);
  };
  $$("input[name=level]").forEach((input) => input.addEventListener("change", render));
  out.textContent = clean(raw, "full");
}

// ---------- App screens ----------

const SHOTS = {
  home: "Vocelle's Home page: words dictated, time saved and recent dictations grouped by day",
  welcome: "The first-run welcome: a short guided setup with five steps",
  text: "The Text page: cleanup levels with a live preview, and the personal dictionary",
  models: "The Models page: six speech models that run on your computer",
};

function initShots() {
  const img = $(".js-shot");
  const figure = $(".shot");
  if (!img) return;
  $$(".shots-tabs button").forEach((tab, _, all) =>
    tab.addEventListener("click", () => {
      all.forEach((t) => t.setAttribute("aria-selected", String(t === tab)));
      const name = tab.dataset.shot;
      img.style.opacity = "0";
      window.setTimeout(() => {
        img.src = `assets/product/${name}.png`;
        img.alt = SHOTS[name];
        img.onload = () => (img.style.opacity = "1");
      }, reduceMotion ? 0 : 160);
    }),
  );
  new IntersectionObserver(
    (entries, observer) => {
      if (entries[0].isIntersecting) {
        figure.classList.add("in-view");
        observer.disconnect();
      }
    },
    { threshold: 0.2 },
  ).observe(figure);
}

// ---------- Top bar ----------

function initBar() {
  const bar = $("#bar");
  const dark = $$(".hero, .story, .private, .app-showcase, .download, .footer");
  const update = () => {
    bar.classList.toggle("scrolled", window.scrollY > 8);
    const y = bar.offsetHeight / 2;
    const onDark = dark.some((el) => {
      const r = el.getBoundingClientRect();
      return r.top <= y && r.bottom >= y;
    });
    bar.classList.toggle("on-dark", onDark);
  };
  window.addEventListener("scroll", update, { passive: true });
  update();
}

// ---------- Downloads: latest release, right file for this system ----------

async function initDownloads() {
  const linux = /Linux/i.test(navigator.userAgent) && !/Android/i.test(navigator.userAgent);
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
    const version = release.tag_name.replace(/^v/, "");
    $$(".js-version").forEach((el) => (el.textContent = `Version ${version}`));
    if (exe) {
      $$(".js-size").forEach((el) => (el.textContent = `${(exe.size / 1048576).toFixed(1)} MB`));
    }
    const primary = linux ? deb : exe;
    $$(".js-download").forEach((el) => primary && (el.href = primary.browser_download_url));
    if (exe) $$(".dl-main").forEach((el) => (el.href = exe.browser_download_url));
    if (deb) $$(".js-deb").forEach((el) => (el.href = deb.browser_download_url));
    if (appimage) $$(".js-appimage").forEach((el) => (el.href = appimage.browser_download_url));
  } catch {
    // Offline or rate-limited: the links still go to the releases page.
  }
  if (linux) {
    const hero = $(".hero-actions .js-download");
    if (hero) hero.textContent = "Download for Linux";
  }
}

// ---------- Start ----------

// A timeout, not requestAnimationFrame: background tabs pause frames, which
// would leave a page opened in the background without its content.
window.setTimeout(() => document.documentElement.classList.add("loaded"), 30);
initDemo();
initStory();
initAnywhere();
initCleanup();
initShots();
initBar();
initDownloads();
