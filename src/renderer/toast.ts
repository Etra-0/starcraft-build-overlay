/**
 * src/renderer/toast.ts
 * Tiny non-blocking toast notifications. Pushes a `<div>` into the
 * `#toasts` stack with an animation, auto-removes after `durationMs`.
 * Variants: ok / warn / error / info.
 */
import { dom } from "./dom.js";

export function toast(message: string, kind = "info", durationMs = 3500) {
  const el = document.createElement("div");
  el.className = `toast ${kind}`;
  el.textContent = String(message);
  dom.toasts.appendChild(el);
  setTimeout(() => {
    el.style.opacity = "0";
    el.style.transform = "translateY(8px)";
    el.style.transition = "opacity 200ms, transform 200ms";
    setTimeout(() => el.remove(), 250);
  }, durationMs);
  return el;
}

export const toastOk = (m: string, d?: number) => toast(m, "ok", d);
export const toastWarn = (m: string, d?: number) => toast(m, "warn", d);
export const toastError = (m: string, d?: number) => toast(m, "error", d);
