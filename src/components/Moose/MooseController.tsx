import React, { useEffect, useState } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { MooseSprite } from "./MooseSprite";

const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

const reducedMotionPreference = () =>
  typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia(REDUCED_MOTION_QUERY).matches
    : false;

export const MooseController: React.FC = () => {
  const {
    characterState,
    mouthShape,
    isBlinking,
    setBlinking,
    isConversationActive,
    startConversation,
    triggerCanned,
    bargeIn,
  } = useMooseStore();
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(
    reducedMotionPreference,
  );

  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const media = window.matchMedia(REDUCED_MOTION_QUERY);
    const updatePreference = (event: MediaQueryListEvent) =>
      setPrefersReducedMotion(event.matches);
    setPrefersReducedMotion(media.matches);
    media.addEventListener("change", updatePreference);
    return () => media.removeEventListener("change", updatePreference);
  }, []);

  // Natural idle blinking scheduler. Reduced-motion users get a stable open-eye frame.
  useEffect(() => {
    if (prefersReducedMotion) {
      setBlinking(false);
      return;
    }

    let blinkTimeout: NodeJS.Timeout;
    let nextBlinkTimeout: NodeJS.Timeout;

    const scheduleNextBlink = () => {
      const delay = Math.random() * 4000 + 2500; // 2.5s - 6.5s
      nextBlinkTimeout = setTimeout(() => {
        setBlinking(true);
        blinkTimeout = setTimeout(() => {
          setBlinking(false);
          scheduleNextBlink();
        }, 160);
      }, delay);
    };

    scheduleNextBlink();

    return () => {
      clearTimeout(blinkTimeout);
      clearTimeout(nextBlinkTimeout);
    };
  }, [prefersReducedMotion, setBlinking]);

  if (characterState === "hidden" || characterState === "dismissed") {
    return null;
  }

  const handleClick = async () => {
    if (characterState === "talking") {
      // Barge in / interrupt
      await bargeIn();
    } else if (!isConversationActive) {
      // If idle, start conversation or trigger canned offline reaction
      await startConversation();
    } else {
      await triggerCanned("click");
    }
  };

  return (
    <div className="w-full h-full flex items-center justify-center overflow-hidden">
      <MooseSprite
        state={characterState}
        mouth={prefersReducedMotion ? "closed" : mouthShape}
        isBlinking={prefersReducedMotion ? false : isBlinking}
        className="w-full h-full max-w-[85vw] max-h-[55vh] aspect-square drop-shadow-md"
        onClick={handleClick}
      />
    </div>
  );
};
