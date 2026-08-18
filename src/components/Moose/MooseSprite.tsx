import React, { useMemo } from "react";
import { CharacterState, MouthShape } from "../../types/moose";
import { getMooseSprite } from "../../lib/sprites";

interface MooseSpriteProps {
  state: CharacterState;
  mouth: MouthShape;
  isBlinking: boolean;
  className?: string;
  onClick?: () => void;
}

export const MooseSprite: React.FC<MooseSpriteProps> = ({
  state,
  mouth,
  isBlinking,
  className = "",
  onClick,
}) => {
  const svgContent = useMemo(() => {
    return getMooseSprite(state, mouth, isBlinking);
  }, [state, mouth, isBlinking]);

  return (
    <div
      data-testid="moose-sprite"
      className={`relative select-none cursor-pointer flex items-center justify-center p-2 transition-transform duration-75 active:scale-95 ${className}`}
      onClick={onClick}
      style={{
        imageRendering: "pixelated",
      }}
      dangerouslySetInnerHTML={{ __html: svgContent }}
    />
  );
};
