import React from "react";
import { useMooseStore } from "../../stores/mooseStore";

export const SpeechBubble: React.FC = () => {
  const { speechBubbleText, speechBubbleVisible, hideSpeechBubble } =
    useMooseStore();

  if (!speechBubbleVisible || !speechBubbleText) {
    return null;
  }

  return (
    <button
      type="button"
      data-testid="speech-bubble"
      aria-label="Dismiss Moose speech"
      aria-live="polite"
      onClick={hideSpeechBubble}
      className="relative w-auto text-left z-20 mx-3 my-2 p-3 bg-white border-2 border-black rounded-lg shadow-[3px_3px_0px_0px_rgba(0,0,0,1)] cursor-pointer select-text transition-all duration-150"
    >
      {/* Speech bubble text */}
      <p className="text-xs font-mono font-semibold text-black leading-relaxed tracking-tight">
        {speechBubbleText}
      </p>

      {/* Retro bubble pointer tail pointing down-left toward moose */}
      <div className="absolute -bottom-2.5 left-8 w-0 h-0 border-l-[6px] border-l-transparent border-r-[6px] border-r-transparent border-t-[8px] border-t-black" />
      <div className="absolute -bottom-2 left-8 w-0 h-0 border-l-[4px] border-l-transparent border-r-[4px] border-r-transparent border-t-[6px] border-t-white" />
    </button>
  );
};
