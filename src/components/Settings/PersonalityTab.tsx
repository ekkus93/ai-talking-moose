import React from "react";
import { useMooseStore } from "../../stores/mooseStore";

const SLIDERS = [
  { key: "dry", label: "Dry Wit & Deadpan" },
  { key: "sarcastic", label: "Sarcasm & Snark" },
  { key: "friendly", label: "Warmth & Friendliness" },
  { key: "absurd", label: "Absurdity & Goofiness" },
  { key: "helpful", label: "Helpfulness (Keep low for classic Moose!)" },
  { key: "verbosity", label: "Verbosity (Response length)" },
] as const;

export const PersonalityTab: React.FC = () => {
  const { settings, updateSettings } = useMooseStore();
  if (!settings) return null;

  return (
    <div className="space-y-3">
      <h3 className="font-bold text-sm border-b border-black pb-1">
        Moose Character Persona
      </h3>

      {SLIDERS.map((slider) => (
        <div key={slider.key} className="space-y-1">
          <div className="flex justify-between text-[11px]">
            <span>{slider.label}</span>
            <span className="font-bold">
              {(
                (settings[slider.key as keyof typeof settings] as number) * 100
              ).toFixed(0)}
              %
            </span>
          </div>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={settings[slider.key as keyof typeof settings] as number}
            aria-label={slider.label}
            onChange={(e) =>
              updateSettings({
                ...settings,
                [slider.key]: parseFloat(e.target.value),
              })
            }
            className="w-full accent-black"
          />
        </div>
      ))}
    </div>
  );
};
