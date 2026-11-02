// Styled dropdowns. Native <select> popups are drawn by the OS: long, plain
// lists that don't match the app. Each select is kept (hidden) as the source
// of truth, so existing code that reads or sets `.value` keeps working, and a
// button plus listbox is drawn in its place.
import { el, type TalkKey } from "./api";

const pickers = new WeakMap<HTMLSelectElement, () => void>();
let openClose: (() => void) | null = null;

/** Replaces `select`'s visible control with a styled picker. */
export function enhanceSelect(select: HTMLSelectElement): void {
  if (pickers.has(select)) return;

  const label = el("span", { className: "picker-label" });
  const button = el(
    "button",
    { type: "button", className: "picker-button" },
    label,
    chevron(),
  );
  button.setAttribute("aria-haspopup", "listbox");
  button.setAttribute("aria-expanded", "false");
  const labelledBy = select.id ? document.querySelector(`label[for="${select.id}"]`) : null;
  button.setAttribute("aria-label", labelledBy?.textContent ?? select.getAttribute("aria-label") ?? "");

  const list = el("div", { className: "picker-list" });
  list.setAttribute("role", "listbox");
  list.hidden = true;

  const wrap = el("div", { className: "picker" }, button, list);
  select.after(wrap);
  select.hidden = true;

  let active = -1;
  const items = () => Array.from(list.querySelectorAll<HTMLElement>("[role=option]"));

  const sync = () => {
    const option = select.selectedOptions[0];
    label.textContent = option?.textContent ?? "";
    button.disabled = select.disabled;
    wrap.classList.toggle("disabled", select.disabled);
  };

  // Setting `.value` from code updates the button too.
  const proto = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!;
  Object.defineProperty(select, "value", {
    get: () => proto.get!.call(select),
    set: (v: string) => {
      proto.set!.call(select, v);
      sync();
    },
  });
  new MutationObserver(sync).observe(select, { childList: true, subtree: true, attributes: true });

  const render = () => {
    const children: HTMLElement[] = [];
    const addOption = (option: HTMLOptionElement, container: HTMLElement[]) => {
      const item = el("div", { className: "picker-option", textContent: option.textContent ?? "" });
      item.setAttribute("role", "option");
      item.dataset.value = option.value;
      item.setAttribute("aria-selected", String(option.value === select.value));
      item.onclick = () => choose(option.value);
      item.onmousemove = () => highlight(items().indexOf(item));
      container.push(item);
    };
    for (const child of Array.from(select.children)) {
      if (child instanceof HTMLOptGroupElement) {
        const group: HTMLElement[] = [];
        child.querySelectorAll("option").forEach((o) => addOption(o, group));
        const columns = Number(child.dataset.grid);
        const body = el("div", { className: columns ? "picker-grid" : "picker-group" }, ...group);
        if (columns) body.style.gridTemplateColumns = `repeat(${columns}, 1fr)`;
        children.push(el("div", { className: "picker-heading", textContent: child.label }), body);
      } else if (child instanceof HTMLOptionElement) {
        addOption(child, children);
      }
    }
    list.replaceChildren(...children);
  };

  const highlight = (index: number) => {
    const all = items();
    if (!all.length) return;
    active = (index + all.length) % all.length;
    all.forEach((item, i) => item.classList.toggle("active", i === active));
    // Scroll only the list; scrollIntoView would also scroll the page or dialog.
    const item = all[active];
    if (item.offsetTop < list.scrollTop) list.scrollTop = item.offsetTop - 4;
    else if (item.offsetTop + item.offsetHeight > list.scrollTop + list.clientHeight)
      list.scrollTop = item.offsetTop + item.offsetHeight - list.clientHeight + 4;
  };

  const close = () => {
    list.hidden = true;
    button.setAttribute("aria-expanded", "false");
    document.removeEventListener("pointerdown", onOutside, true);
    if (openClose === close) openClose = null;
  };

  const open = () => {
    if (select.disabled) return;
    openClose?.();
    openClose = close;
    render();
    list.hidden = false;
    button.setAttribute("aria-expanded", "true");
    // Open upwards when there isn't room below.
    const room = window.innerHeight - button.getBoundingClientRect().bottom;
    wrap.classList.toggle("up", room < Math.min(280, list.scrollHeight + 16));
    highlight(Math.max(0, items().findIndex((i) => i.dataset.value === select.value)));
    document.addEventListener("pointerdown", onOutside, true);
  };

  const onOutside = (e: Event) => {
    if (!wrap.contains(e.target as Node)) close();
  };

  const choose = (value: string) => {
    close();
    button.focus();
    if (value === select.value) return;
    select.value = value;
    select.dispatchEvent(new Event("change"));
  };

  button.onclick = () => (list.hidden ? open() : close());
  button.onkeydown = (e) => {
    const isOpen = !list.hidden;
    switch (e.key) {
      case "ArrowDown":
      case "ArrowUp":
        e.preventDefault();
        if (!isOpen) open();
        else highlight(active + (e.key === "ArrowDown" ? 1 : -1));
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        if (isOpen && active >= 0) choose(items()[active].dataset.value!);
        else open();
        break;
      case "Escape":
        if (isOpen) {
          e.preventDefault();
          close();
        }
        break;
      case "Tab":
        close();
        break;
      default:
        // Type to jump to the next option starting with that letter.
        if (isOpen && e.key.length === 1) {
          const all = items();
          const start = active + 1;
          for (let i = 0; i < all.length; i++) {
            const item = all[(start + i) % all.length];
            if (item.textContent?.toLowerCase().startsWith(e.key.toLowerCase())) {
              highlight((start + i) % all.length);
              break;
            }
          }
        }
    }
  };

  pickers.set(select, sync);
  sync();
}

function chevron(): SVGSVGElement {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  svg.classList.add("picker-chevron");
  const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
  path.setAttribute("d", "M6 9l6 6 6-6");
  svg.append(path);
  return svg;
}

/**
 * Fills a talk key select in groups shown as grids of key chips, so all keys
 * fit in the popup without scrolling: the keys most people use first, then
 * the rest, then the F-keys.
 */
export function fillTalkKeys(select: HTMLSelectElement, keys: TalkKey[], value: string): void {
  const option = (k: TalkKey) => el("option", { value: k.code, textContent: k.label });
  const fkeys = keys.filter((k) => /^F\d+$/.test(k.code));
  const recommended = keys.filter((k) => ["ControlRight", "ShiftRight", "CapsLock", "ControlLeft"].includes(k.code));
  const others = keys.filter((k) => !fkeys.includes(k) && !recommended.includes(k));
  const group = (label: string, list: TalkKey[], columns: number) => {
    const g = el("optgroup", { label }, ...list.map(option));
    g.dataset.grid = String(columns);
    return g;
  };
  select.replaceChildren(
    group("Recommended", recommended, 2),
    ...(others.length ? [group("Other keys", others, 4)] : []),
    ...(fkeys.length ? [group("Function keys", fkeys, 6)] : []),
  );
  select.value = value;
}
