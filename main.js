// Point every download button straight at the installer from the latest
// release, so a click starts the download instead of opening GitHub.
// The links in the HTML are a working fallback if this request fails.
const REPO = "codewithdark-git/vocelle";

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

const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

const video = document.querySelector(".film video");
if (video && reduceMotion) {
  video.removeAttribute("autoplay");
  video.pause();
}

// The try-it panel: hold the keycap (or the real Right Ctrl) and the
// waveform moves; let go and a sentence types itself into the field.
const SENTENCES = [
  "Can we move the design review to Thursday at 3? I'd like the new mockups first.",
  "Thanks for the quick turnaround. The draft looks great, just two small notes below.",
  "Remind the team that the release notes need to be final by Friday morning.",
  "I'm running about ten minutes late, start without me and I'll catch up.",
];

const key = document.querySelector(".js-key");
const field = document.querySelector(".try-field");
const text = document.querySelector(".js-try-text");
const status = document.querySelector(".js-status");
const bars = [...document.querySelectorAll(".js-wave i")];

let holding = false;
let heldSince = 0;
let sentence = 0;
let typing = 0;
let frame = 0;

function animateWave(t) {
  bars.forEach((bar, i) => {
    const level = holding
      ? 0.25 + 0.75 * Math.abs(Math.sin(t / 170 + i * 0.7) * Math.sin(t / 410 + i * 0.31))
      : 0.08;
    bar.style.transform = `scaleY(${level.toFixed(3)})`;
  });
  if (holding) frame = requestAnimationFrame(animateWave);
}

function press() {
  if (holding) return;
  holding = true;
  heldSince = performance.now();
  clearInterval(typing);
  key.classList.add("is-down");
  status.textContent = "Listening. Let go when you're done.";
  if (!reduceMotion) frame = requestAnimationFrame(animateWave);
}

function release() {
  if (!holding) return;
  holding = false;
  cancelAnimationFrame(frame);
  animateWave(0);
  key.classList.remove("is-down");

  if (performance.now() - heldSince < 250) {
    status.textContent = "Hold the key a little longer, then let go.";
    return;
  }

  const words = SENTENCES[sentence++ % SENTENCES.length];
  field.classList.add("has-text");
  status.textContent = "Typed. Hold it again for another.";
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
  }, 18);
}

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

useLatestRelease().catch(() => {});
