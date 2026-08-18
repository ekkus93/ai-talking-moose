import React, { useEffect } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { MooseSprite } from "./MooseSprite";

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

  // Natural idle blinking scheduler
  useEffect(() => {
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
  }, [setBlinking]);

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
        mouth={mouthShape}
        isBlinking={isBlinking}
        className="w-full h-full max-w-[85vw] max-h-[55vh] aspect-square drop-shadow-md"
        onClick={handleClick}
      />
    </div>
  );
};
