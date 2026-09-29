import { backOut, quintOut } from "svelte/easing";

// Bottom sheet: slides up from off-screen on the ease-out curve.
export function sheetSlide(_node: Element, { duration = 500 } = {}) {
  return { duration, easing: quintOut, css: (t: number) => `transform: translateY(${(1 - t) * 100}%)` };
}

// Welcome card: springy scale + lift.
export function pop(_node: Element, { duration = 550 } = {}) {
  return {
    duration,
    easing: backOut,
    css: (t: number, u: number) => `transform: scale(${0.94 + 0.06 * t}) translateY(${16 * u}px)`,
  };
}
