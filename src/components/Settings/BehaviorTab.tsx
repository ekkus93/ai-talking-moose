import React from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { formatLocalHour } from "./formatLocalHour";

export const BehaviorTab: React.FC = () => {
  const { settings, updateSettings } = useMooseStore();
  if (!settings) return null;

  return (
    <div className="space-y-4">
      <h3 className="font-bold text-sm border-b border-black pb-1">
        Ambient Behavior & Timing
      </h3>
      <label className="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          checked={settings.unsolicited_comments}
          onChange={(e) =>
            updateSettings({
              ...settings,
              unsolicited_comments: e.target.checked,
            })
          }
          className="accent-black"
        />
        <span className="font-bold">Enable unsolicited ambient remarks</span>
      </label>

      <div className="space-y-1">
        <div className="flex justify-between">
          <span>Talkativeness</span>
          <span className="font-bold">
            {(settings.talkativeness * 100).toFixed(0)}%
          </span>
        </div>
        <input
          type="range"
          min="0"
          max="1"
          step="0.05"
          value={settings.talkativeness}
          aria-label="Talkativeness"
          onChange={(e) =>
            updateSettings({
              ...settings,
              talkativeness: parseFloat(e.target.value),
            })
          }
          className="w-full accent-black"
        />
        <p className="text-[11px] text-gray-700">
          Lower values make Moose require more important events before speaking;
          higher values make him more willing to comment. The hourly cap and
          quiet hours still apply.
        </p>
      </div>

      <div className="space-y-1">
        <div className="flex justify-between">
          <span>Maximum Ambient Comments Per Hour</span>
          <span className="font-bold">{settings.max_comments_per_hour}</span>
        </div>
        <input
          type="range"
          min="1"
          max="12"
          step="1"
          value={settings.max_comments_per_hour}
          aria-label="Maximum ambient comments per hour"
          onChange={(e) =>
            updateSettings({
              ...settings,
              max_comments_per_hour: parseInt(e.target.value),
            })
          }
          className="w-full accent-black"
        />
      </div>

      <div className="border border-black p-3 rounded bg-[#fbf9f5] space-y-2">
        <label className="flex items-center gap-2 font-bold">
          <input
            type="checkbox"
            checked={settings.quiet_hours_enabled}
            onChange={(e) =>
              updateSettings({
                ...settings,
                quiet_hours_enabled: e.target.checked,
              })
            }
            className="accent-black"
          />
          <span>Quiet Hours</span>
        </label>
        <div className="grid grid-cols-2 gap-2">
          <label className="space-y-1">
            <span className="block font-bold text-[11px]">
              Start (local time)
            </span>
            <select
              aria-label="Quiet hours start"
              value={settings.quiet_hours_start}
              onChange={(e) =>
                updateSettings({
                  ...settings,
                  quiet_hours_start: Number(e.target.value),
                })
              }
              disabled={!settings.quiet_hours_enabled}
              className="w-full p-1.5 border border-black rounded bg-white disabled:opacity-60"
            >
              {Array.from({ length: 24 }, (_, hour) => (
                <option key={hour} value={hour}>
                  {formatLocalHour(hour)}
                </option>
              ))}
            </select>
          </label>
          <label className="space-y-1">
            <span className="block font-bold text-[11px]">
              End (local time)
            </span>
            <select
              aria-label="Quiet hours end"
              value={settings.quiet_hours_end}
              onChange={(e) =>
                updateSettings({
                  ...settings,
                  quiet_hours_end: Number(e.target.value),
                })
              }
              disabled={!settings.quiet_hours_enabled}
              className="w-full p-1.5 border border-black rounded bg-white disabled:opacity-60"
            >
              {Array.from({ length: 24 }, (_, hour) => (
                <option key={hour} value={hour}>
                  {formatLocalHour(hour)}
                </option>
              ))}
            </select>
          </label>
        </div>
        <p className="text-gray-700 text-[11px]">
          {settings.quiet_hours_start === settings.quiet_hours_end
            ? "Start and end are the same, so there is no quiet interval."
            : settings.quiet_hours_start > settings.quiet_hours_end
              ? `Overnight: quiet from ${formatLocalHour(settings.quiet_hours_start)} through midnight until ${formatLocalHour(settings.quiet_hours_end)}.`
              : `Same-day: quiet from ${formatLocalHour(settings.quiet_hours_start)} until ${formatLocalHour(settings.quiet_hours_end)}.`}
        </p>
      </div>
    </div>
  );
};
