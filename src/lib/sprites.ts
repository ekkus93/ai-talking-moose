import { CharacterState, MouthShape } from "../types/moose";

// Original Retro 1-bit / 2-bit Macintosh Style Pixel Moose Artwork
// Rendered on a 32x32 crisp pixel grid with classic dither and silhouette

export interface SpriteFrame {
  name: string;
  svg: string;
}

export const getMooseSprite = (
  state: CharacterState,
  mouth: MouthShape,
  isBlinking: boolean,
): string => {
  // Determine effective eye state
  const eyesClosed = isBlinking;
  const isListening = state === "listening";
  const isThinking = state === "thinking";
  const isError = state === "error";

  // Determine mouth shape
  const effectiveMouth = state === "talking" ? mouth : "closed";

  return renderMooseSvg({
    eyesClosed,
    isListening,
    isThinking,
    isError,
    mouth: effectiveMouth,
  });
};

interface RenderParams {
  eyesClosed: boolean;
  isListening: boolean;
  isThinking: boolean;
  isError: boolean;
  mouth: MouthShape;
}

const renderMooseSvg = (p: RenderParams): string => {
  // Antlers SVG paths
  const leftAntler = p.isListening
    ? `<path d="M 4,4 H 7 V 7 H 9 V 10 H 7 V 13 H 5 V 10 H 3 V 7 H 4 Z M 1,2 H 4 V 4 H 1 Z" fill="#1a1a1a"/>`
    : `<path d="M 5,6 H 8 V 9 H 10 V 12 H 8 V 14 H 6 V 11 H 4 V 9 H 5 Z M 2,4 H 5 V 6 H 2 Z" fill="#1a1a1a"/>`;

  const rightAntler = p.isListening
    ? `<path d="M 25,4 H 28 V 7 H 29 V 10 H 27 V 13 H 25 V 10 H 23 V 7 H 25 Z M 28,2 H 31 V 4 H 28 Z" fill="#1a1a1a"/>`
    : `<path d="M 24,6 H 27 V 9 H 28 V 12 H 26 V 14 H 24 V 11 H 22 V 9 H 24 Z M 27,4 H 30 V 6 H 27 Z" fill="#1a1a1a"/>`;

  // Ears
  const leftEar = p.isListening
    ? `<rect x="6" y="11" width="3" height="4" fill="#3a3a3a"/>`
    : `<rect x="7" y="13" width="3" height="3" fill="#3a3a3a"/>`;

  const rightEar = p.isListening
    ? `<rect x="23" y="11" width="3" height="4" fill="#3a3a3a"/>`
    : `<rect x="22" y="13" width="3" height="3" fill="#3a3a3a"/>`;

  // Head Base & Neck
  const headBase = `
    <!-- Neck / Collar -->
    <rect x="13" y="24" width="6" height="7" fill="#2b2b2b"/>
    <rect x="11" y="29" width="10" height="3" fill="#1a1a1a"/>
    <!-- Head Outline -->
    <rect x="10" y="12" width="12" height="12" fill="#524a42"/>
    <rect x="9" y="14" width="14" height="8" fill="#524a42"/>
    <rect x="8" y="18" width="16" height="8" fill="#6e6255"/>
    <rect x="9" y="24" width="14" height="4" fill="#6e6255"/>
  `;

  // Eyes
  let eyesSvg = "";
  if (p.isError) {
    // X eyes
    eyesSvg = `
      <!-- Left X Eye -->
      <path d="M 11,15 L 14,18 M 14,15 L 11,18" stroke="#111" stroke-width="1.5"/>
      <!-- Right X Eye -->
      <path d="M 18,15 L 21,18 M 21,15 L 18,18" stroke="#111" stroke-width="1.5"/>
    `;
  } else if (p.eyesClosed) {
    // Closed line eyes
    eyesSvg = `
      <rect x="11" y="17" width="4" height="1" fill="#1a1a1a"/>
      <rect x="17" y="17" width="4" height="1" fill="#1a1a1a"/>
    `;
  } else if (p.isThinking) {
    // Looking up / right
    eyesSvg = `
      <rect x="11" y="14" width="4" height="4" fill="#ffffff"/>
      <rect x="13" y="14" width="2" height="2" fill="#1a1a1a"/>
      <rect x="17" y="14" width="4" height="4" fill="#ffffff"/>
      <rect x="19" y="14" width="2" height="2" fill="#1a1a1a"/>
      <!-- Raised Eyebrow -->
      <rect x="17" y="12" width="4" height="1" fill="#1a1a1a"/>
    `;
  } else {
    // Normal / Alert Eyes
    eyesSvg = `
      <rect x="11" y="15" width="4" height="4" fill="#ffffff"/>
      <rect x="12" y="16" width="2" height="2" fill="#1a1a1a"/>
      <rect x="17" y="15" width="4" height="4" fill="#ffffff"/>
      <rect x="18" y="16" width="2" height="2" fill="#1a1a1a"/>
    `;
  }

  // Snout & Nostrils
  const snout = `
    <!-- Big Cartoon Snout -->
    <rect x="10" y="20" width="12" height="6" fill="#8c7d6b"/>
    <rect x="11" y="21" width="3" height="3" fill="#1a1a1a"/>
    <rect x="18" y="21" width="3" height="3" fill="#1a1a1a"/>
  `;

  // Mouth Shapes
  let mouthSvg = "";
  switch (p.mouth) {
    case "small":
      mouthSvg = `
        <rect x="14" y="26" width="4" height="2" fill="#1a1a1a"/>
        <rect x="15" y="26" width="2" height="1" fill="#ff6b6b"/>
      `;
      break;
    case "medium":
      mouthSvg = `
        <rect x="13" y="25" width="6" height="3" fill="#1a1a1a"/>
        <rect x="14" y="26" width="4" height="2" fill="#e04040"/>
        <rect x="14" y="25" width="4" height="1" fill="#ffffff"/>
      `;
      break;
    case "wide":
      mouthSvg = `
        <rect x="12" y="25" width="8" height="4" fill="#1a1a1a"/>
        <rect x="13" y="26" width="6" height="3" fill="#d32f2f"/>
        <rect x="13" y="25" width="6" height="1" fill="#ffffff"/>
      `;
      break;
    case "closed":
    default:
      mouthSvg = `
        <rect x="13" y="26" width="6" height="1" fill="#1a1a1a"/>
      `;
      break;
  }

  return `
    <svg viewBox="0 0 32 32" width="100%" height="100%" xmlns="http://www.w3.org/2000/svg" shape-rendering="crispEdges">
      <g>
        ${leftAntler}
        ${rightAntler}
        ${leftEar}
        ${rightEar}
        ${headBase}
        ${eyesSvg}
        ${snout}
        ${mouthSvg}
      </g>
    </svg>
  `;
};
