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
  document.querySelectorAll(".js-version").forEach((el) => (el.textContent = release.tag_name));
  document.querySelectorAll(".js-size").forEach((el) => (el.textContent = size));
}

document.querySelectorAll(".js-year").forEach((el) => (el.textContent = new Date().getFullYear()));

// Respect reduced-motion: don't autoplay the demo.
const video = document.querySelector(".demo video");
if (video && matchMedia("(prefers-reduced-motion: reduce)").matches) {
  video.removeAttribute("autoplay");
  video.pause();
}

useLatestRelease().catch(() => {});
