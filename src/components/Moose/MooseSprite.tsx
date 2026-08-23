import React, { useEffect, useMemo, useRef, useState } from "react";
import { CharacterState, MouthShape } from "../../types/moose";
import { getMooseSprite } from "../../lib/sprites";

interface MooseSpriteProps {
  state: CharacterState;
  mouth: MouthShape;
  isBlinking: boolean;
  className?: string;
  onClick?: () => void;
}

const PIXEL_GRID = 32;

export const MooseSprite: React.FC<MooseSpriteProps> = ({
  state,
  mouth,
  isBlinking,
  className = "",
  onClick,
}) => {
  const frameRef = useRef<HTMLDivElement>(null);
  const [renderSize, setRenderSize] = useState<number | null>(null);
  const svgContent = useMemo(() => {
    return getMooseSprite(state, mouth, isBlinking);
  }, [state, mouth, isBlinking]);

  useEffect(() => {
    const frame = frameRef.current;
    if (!frame || typeof ResizeObserver === "undefined") return;

    const updateSize = (width: number, height: number) => {
      const available = Math.min(width, height);
      if (available < PIXEL_GRID) {
        setRenderSize(null);
        return;
      }
      const integerScaled = Math.floor(available / PIXEL_GRID) * PIXEL_GRID;
      setRenderSize(integerScaled);
    };

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) updateSize(entry.contentRect.width, entry.contentRect.height);
    });
    observer.observe(frame);

    const rect = frame.getBoundingClientRect();
    updateSize(rect.width, rect.height);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      ref={frameRef}
      className={`relative select-none flex items-center justify-center p-2 ${className}`}
    >
      <button
        type="button"
        data-testid="moose-sprite"
        aria-label="Talk to Moose"
        className="pixelated-sprite cursor-pointer flex items-center justify-center p-0 bg-transparent border-0"
        onClick={onClick}
        style={
          renderSize
            ? {
                width: `${renderSize}px`,
                height: `${renderSize}px`,
                imageRendering: "pixelated",
              }
            : {
                width: "100%",
                height: "100%",
                imageRendering: "pixelated",
              }
        }
        dangerouslySetInnerHTML={{ __html: svgContent }}
      />
    </div>
  );
};
