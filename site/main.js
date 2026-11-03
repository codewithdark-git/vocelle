// Vocelle website: download links, reveal-on-scroll, scroll-driven scenes
// and the try-it demo. No libraries; every scene degrades to its end state
// with prefers-reduced-motion.
const REPO = "codewithdark-git/vocelle";
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
const clamp = (v, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, v));

// ---------- Downloads: point buttons at the latest installer ----------

async function useLatestRelease() {
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { Accept: "application/vnd.github+json" },
  });
  if (!res.ok) return;
  const release = await res.json();
  const installer = release.assets.find((a) => /x64-setup\.exe$/i.test(a.name));
  if (!installer) return;
  const size = `${(installer.size / 1024 / 1024).toFixed(1)} MB`;
  document.querySelectorAll(".js-download").forEach((a) => (a.href = installer.browser_download_url));
  document.querySelectorAll(".js-version").forEach((el) => (el.textContent = release.tag_name.replace(/^v/, "")));
  document.querySelectorAll(".js-size").forEach((el) => (el.textContent = size));
}

document.querySelectorAll(".js-year").forEach((el) => (el.textContent = new Date().getFullYear()));

// ---------- Reveal on scroll ----------

const revealer = new IntersectionObserver(
  (entries) => {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        entry.target.classList.add("is-in");
        revealer.unobserve(entry.target);
      }
    }
  },
  { rootMargin: "0px 0px -8% 0px" },
);
document.querySelectorAll(".reveal").forEach((el) => revealer.observe(el));

// ---------- Speed race: bars and counters run once when in view ----------

const race = document.querySelector(".js-race");
if (race) {
  const fills = race.querySelectorAll(".lane-fill");
  const counts = race.querySelectorAll(".js-count");
  const finish = () => {
    fills.forEach((f) => f.style.setProperty("--p", 1));
    counts.forEach((c) => (c.textContent = c.dataset.to));
  };
  if (reduceMotion) {
    finish();
  } else {
    new IntersectionObserver(
      (entries, observer) => {
        if (!entries[0].isIntersecting) return;
        observer.disconnect();
        const start = performance.now();
        const step = (now) => {
          const t = clamp((now - start) / 1800);
          const eased = 1 - Math.pow(1 - t, 3);
          fills.forEach((f) => f.style.setProperty("--p", eased));
          counts.forEach((c) => (c.textContent = Math.round(Number(c.dataset.to) * eased)));
          if (t < 1) requestAnimationFrame(step);
        };
        requestAnimationFrame(step);
      },
      { threshold: 0.4 },
    ).observe(race);
  }
}

// ---------- Scroll scenes ----------

const MAIL = "Hey Sarah, the proposal looks solid. Can we simplify step two before Friday?";

const story = document.querySelector("#how");
const storySteps = story ? [...story.querySelectorAll(".story-steps li")] : [];
const stageKey = document.querySelector(".js-stage-key");
const stagePill = document.querySelector(".js-stage-pill");
const spoken = [...document.querySelectorAll(".js-spoken span")];
const mailText = document.querySelector(".js-mail-text");
const privacy = document.querySelector("#privacy");
const cleanup = document.querySelector(".cleanup");
const caps = cleanup ? [...cleanup.querySelectorAll(".cap")] : [];

/** Progress through a pinned section: 0 when it reaches the top, 1 when it ends. */
function progress(section) {
  const rect = section.getBoundingClientRect();
  const travel = rect.height - innerHeight;
  return travel > 0 ? clamp(-rect.top / travel) : rect.top < innerHeight / 2 ? 1 : 0;
}

function renderStory(p) {
  // Three steps share the scroll: hold the key, talk, let go.
  const stepped = p * 3;
  const step = Math.min(2, Math.floor(stepped));
  const local = stepped - step;
  storySteps.forEach((li, i) => li.classList.toggle("is-active", i === step));

  const holding = (step === 0 && local > 0.25) || step === 1;
  stageKey.classList.toggle("is-down", holding);

  let phase = "idle";
  if (holding) phase = "listening";
  if (step === 2) phase = local < 0.3 ? "thinking" : "done";
  stagePill.dataset.phase = phase;

  // Talking: words drift in one by one, then the fillers get struck out.
  const shown = step === 0 ? 0 : step === 1 ? Math.ceil(local * 1.25 * spoken.length) : spoken.length;
  spoken.forEach((s, i) => {
    s.classList.toggle("is-in", i < shown);
    s.classList.toggle("is-out", s.classList.contains("filler") && (step === 2 || (step === 1 && local > 0.85)));
  });

  // Letting go: the clean sentence types itself into the email.
  const typed = step < 2 ? 0 : Math.round(clamp((local - 0.3) / 0.45) * MAIL.length);
  mailText.textContent = MAIL.slice(0, typed);
}

function renderCleanup(p) {
  cleanup.style.setProperty("--strike", clamp(p * 3 - 0.2).toFixed(3));
  cleanup.style.setProperty("--gone", clamp(p * 3 - 1.2).toFixed(3));
  const fixed = p > 0.62;
  caps.forEach((c) => {
    c.textContent = fixed ? c.dataset.to : c.dataset.from;
    c.classList.toggle("is-fixed", fixed);
  });
}

let ticking = false;
function onScroll() {
  if (ticking) return;
  ticking = true;
  requestAnimationFrame(() => {
    ticking = false;
    if (story) renderStory(progress(story));
    if (privacy) privacy.style.setProperty("--p", progress(privacy).toFixed(3));
    if (cleanup) renderCleanup(progress(cleanup));
  });
}

if (reduceMotion) {
  // Show each scene's end state.
  if (story) {
    stageKey.classList.remove("is-down");
    stagePill.dataset.phase = "done";
    spoken.forEach((s) => s.classList.add("is-in", ...(s.classList.contains("filler") ? ["is-out"] : [])));
    mailText.textContent = MAIL;
  }
  if (privacy) privacy.style.setProperty("--p", 1);
  if (cleanup) renderCleanup(1);
} else {
  addEventListener("scroll", onScroll, { passive: true });
  addEventListener("resize", onScroll);
  onScroll();
}

// ---------- Try it: hold the keycap (or the real Right Ctrl) ----------

const SENTENCES = [
  "Can we move the design review to Thursday at three? I'd like the new mockups first.",
  "Thanks for the quick turnaround. The draft looks great, just two small notes below.",
  "Remind the team that the release notes need to be final by Friday morning.",
  "I'm running about ten minutes late, start without me and I'll catch up.",
];

const key = document.querySelector(".js-key");
const field = document.querySelector(".try-field");
const text = document.querySelector(".js-try-text");
const status = document.querySelector(".js-status");
const pill = document.querySelector(".js-try-pill");

let holding = false;
let heldSince = 0;
let sentence = 0;
let typing = 0;
let settle = 0;

function press() {
  if (holding) return;
  holding = true;
  heldSince = performance.now();
  clearInterval(typing);
  clearTimeout(settle);
  key.classList.add("is-down");
  pill.dataset.phase = "listening";
  status.textContent = "Listening. Let go when you're done.";
}

function release() {
  if (!holding) return;
  holding = false;
  key.classList.remove("is-down");

  if (performance.now() - heldSince < 250) {
    pill.dataset.phase = "idle";
    status.textContent = "Hold the key a little longer, then let go.";
    return;
  }

  const words = SENTENCES[sentence++ % SENTENCES.length];
  field.classList.add("has-text");
  pill.dataset.phase = "thinking";
  status.textContent = "Typed. Hold it again for another.";
  const type = () => {
    pill.dataset.phase = "done";
    if (reduceMotion) {
      text.textContent = words;
      return;
    }
    let shown = 0;
    text.textContent = "";
    typing = setInterval(() => {
      shown += 2;
      text.textContent = words.slice(0, shown);
      if (shown >= words.length) clearInterval(typing);
    }, 16);
    settle = setTimeout(() => (pill.dataset.phase = "idle"), 2200);
  };
  setTimeout(type, reduceMotion ? 0 : 450);
}

if (key) {
  key.addEventListener("pointerdown", (e) => {
    key.setPointerCapture(e.pointerId);
    press();
  });
  key.addEventListener("pointerup", release);
  key.addEventListener("pointercancel", release);
  key.addEventListener("keydown", (e) => {
    if ((e.key === " " || e.key === "Enter") && !e.repeat) {
      e.preventDefault();
      press();
    }
  });
  key.addEventListener("keyup", (e) => {
    if (e.key === " " || e.key === "Enter") release();
  });
  // The real talk key works anywhere on the page.
  addEventListener("keydown", (e) => {
    if (e.code === "ControlRight" && !e.repeat) press();
  });
  addEventListener("keyup", (e) => {
    if (e.code === "ControlRight") release();
  });
  addEventListener("blur", release);
}

useLatestRelease().catch(() => {});
