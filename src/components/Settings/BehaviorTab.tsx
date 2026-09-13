import React, { useEffect, useMemo, useState } from "react";
import { frontendDefaultSettings } from "../../lib/backendContract";
import { useMooseStore } from "../../stores/mooseStore";
import { formatLocalHour } from "./formatLocalHour";

const MAX_SEED_TOPICS = 64;
const MAX_SEED_CHARS = 120;
const MAX_TOTAL_SEED_CHARS = 4096;

const formatIdleBanterInterval = (minutes: number): string => {
  if (!Number.isFinite(minutes) || minutes < 1) return "";
  if (minutes % 60 === 0) {
    const hours = minutes / 60;
    return `${hours} ${hours === 1 ? "hour" : "hours"}`;
  }
  if (minutes >= 60) {
    const hours = Math.floor(minutes / 60);
    const remainder = minutes % 60;
    return `${hours}h ${remainder}m`;
  }
  return `${minutes} minutes`;
};

const normalizeSeedTopics = (topics: string[]): string[] | null => {
  if (topics.length < 1 || topics.length > MAX_SEED_TOPICS) return null;
  const normalized = topics.map((topic) => topic.trim());
  if (
    normalized.some(
      (topic) => topic.length === 0 || [...topic].length > MAX_SEED_CHARS,
    )
  )
    return null;
  if (
    normalized.reduce((total, topic) => total + [...topic].length, 0) >
    MAX_TOTAL_SEED_CHARS
  )
    return null;
  const keys = normalized.map((topic) => topic.toLowerCase());
  if (new Set(keys).size !== keys.length) return null;
  return normalized;
};

export const BehaviorTab: React.FC = () => {
  const { settings, updateSettingsPatch, updateSettingsContinuousPatch } =
    useMooseStore();
  const [seedDraft, setSeedDraft] = useState<string[]>([]);
  const [newSeed, setNewSeed] = useState("");
  const [seedError, setSeedError] = useState<string | null>(null);

  const persistedSeeds = settings?.idle_banter_seed_topics;
  useEffect(() => {
    if (persistedSeeds) setSeedDraft([...persistedSeeds]);
  }, [persistedSeeds]);

  const defaultSeeds = useMemo(
    () => frontendDefaultSettings().idle_banter_seed_topics,
    [],
  );

  if (!settings) return null;

  const saveSeedDraft = async (topics = seedDraft) => {
    const normalized = normalizeSeedTopics(topics);
    if (!normalized) {
      setSeedError(
        "Seed topics must be unique, non-empty, 120 characters or fewer, and stay within the list limits.",
      );
      return;
    }
    setSeedError(null);
    setSeedDraft(normalized);
    await updateSettingsPatch({ idle_banter_seed_topics: normalized });
  };

  const addSeed = () => {
    const candidate = newSeed.trim();
    const next = [...seedDraft, candidate];
    if (!normalizeSeedTopics(next)) {
      setSeedError(
        "That topic is empty, duplicated, too long, or would exceed the seed-topic limits.",
      );
      return;
    }
    setSeedError(null);
    setSeedDraft(next);
    setNewSeed("");
  };

  const disabled = !settings.idle_banter_enabled;

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
            updateSettingsPatch({
              unsolicited_comments: e.target.checked,
            })
          }
          className="accent-black"
        />
        <span className="font-bold">Enable unsolicited ambient remarks</span>
      </label>

      <div className="border border-black p-3 rounded bg-[#fbf9f5] space-y-3">
        <label className="flex items-center gap-2 font-bold cursor-pointer">
          <input
            type="checkbox"
            checked={settings.idle_banter_enabled}
            onChange={(e) =>
              updateSettingsPatch({ idle_banter_enabled: e.target.checked })
            }
            className="accent-black"
          />
          <span>Idle Banter</span>
        </label>
        {!settings.unsolicited_comments && (
          <p className="text-[11px] text-gray-700" role="status">
            Idle Banter is paused while unsolicited ambient remarks are
            disabled.
          </p>
        )}
        <p className="text-[11px] text-gray-700">
          After you ignore Moose for a while, he can occasionally make a short
          snarky remark. Direct interaction with Moose restarts the timer.
        </p>

        <div className="grid grid-cols-2 gap-2">
          <label className="space-y-1">
            <span className="block font-bold text-[11px]">
              First remark after
            </span>
            <div className="flex items-center gap-1">
              <input
                aria-label="Idle Banter initial delay in minutes"
                type="number"
                min={5}
                max={1440}
                value={settings.idle_banter_initial_delay_minutes}
                disabled={disabled}
                onChange={(e) =>
                  updateSettingsPatch({
                    idle_banter_initial_delay_minutes: Number(e.target.value),
                  })
                }
                className="w-full p-1.5 border border-black rounded bg-white disabled:opacity-60"
              />
              <span className="text-[11px]">min</span>
            </div>
            <span className="block text-[10px] text-gray-600">
              {formatIdleBanterInterval(
                settings.idle_banter_initial_delay_minutes,
              )}
            </span>
          </label>
          <label className="space-y-1">
            <span className="block font-bold text-[11px]">About every</span>
            <div className="flex items-center gap-1">
              <input
                aria-label="Idle Banter repeat interval in minutes"
                type="number"
                min={5}
                max={1440}
                value={settings.idle_banter_repeat_interval_minutes}
                disabled={disabled}
                onChange={(e) =>
                  updateSettingsPatch({
                    idle_banter_repeat_interval_minutes: Number(e.target.value),
                  })
                }
                className="w-full p-1.5 border border-black rounded bg-white disabled:opacity-60"
              />
              <span className="text-[11px]">min</span>
            </div>
            <span className="block text-[10px] text-gray-600">
              {formatIdleBanterInterval(
                settings.idle_banter_repeat_interval_minutes,
              )}
            </span>
          </label>
        </div>

        <div className={`space-y-2 ${disabled ? "opacity-60" : ""}`}>
          <div className="flex justify-between items-center gap-2">
            <span className="font-bold text-[11px]">Creative seed topics</span>
            <button
              type="button"
              disabled={disabled}
              onClick={() => {
                const restored = [...defaultSeeds];
                setSeedDraft(restored);
                setSeedError(null);
                void updateSettingsPatch({ idle_banter_seed_topics: restored });
              }}
              className="px-2 py-1 border border-black rounded bg-white text-[11px] disabled:opacity-60"
            >
              Restore Defaults
            </button>
          </div>
          {seedDraft.map((topic, index) => (
            <div key={index} className="flex gap-2 items-center">
              <input
                aria-label={`Idle Banter seed topic ${index + 1}`}
                value={topic}
                disabled={disabled}
                maxLength={MAX_SEED_CHARS}
                onChange={(e) => {
                  const next = [...seedDraft];
                  next[index] = e.target.value;
                  setSeedDraft(next);
                  setSeedError(null);
                }}
                className="flex-1 p-1.5 border border-black rounded bg-white disabled:opacity-60"
              />
              <button
                type="button"
                aria-label={`Delete Idle Banter seed topic ${index + 1}`}
                disabled={disabled || seedDraft.length === 1}
                onClick={() => {
                  setSeedDraft(
                    seedDraft.filter((_, candidate) => candidate !== index),
                  );
                  setSeedError(null);
                }}
                className="px-2 py-1 border border-black rounded bg-white text-[11px] disabled:opacity-40"
              >
                Delete
              </button>
            </div>
          ))}
          <div className="flex gap-2">
            <input
              aria-label="New Idle Banter seed topic"
              value={newSeed}
              disabled={disabled}
              maxLength={MAX_SEED_CHARS}
              onChange={(e) => setNewSeed(e.target.value)}
              className="flex-1 p-1.5 border border-black rounded bg-white disabled:opacity-60"
              placeholder="Add a creative direction"
            />
            <button
              type="button"
              disabled={disabled || seedDraft.length >= MAX_SEED_TOPICS}
              onClick={addSeed}
              className="px-2 py-1 border border-black rounded bg-white text-[11px] disabled:opacity-40"
            >
              Add
            </button>
            <button
              type="button"
              disabled={disabled}
              onClick={() => void saveSeedDraft()}
              className="px-2 py-1 border border-black rounded bg-white text-[11px] disabled:opacity-40"
            >
              Save Topics
            </button>
          </div>
          {seedError && (
            <p role="alert" className="text-[11px] font-bold">
              {seedError}
            </p>
          )}
        </div>
      </div>

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
            updateSettingsContinuousPatch({
              talkativeness: parseFloat(e.target.value),
            })
          }
          className="w-full accent-black"
        />
        <p className="text-[11px] text-gray-700">
          Lower values make Moose require more important events before speaking;
          higher values make him more willing to comment. The hourly cap and
          quiet hours still apply. Due Idle Banter uses its configured schedule
          instead of this importance threshold.
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
            updateSettingsContinuousPatch({
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
              updateSettingsPatch({
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
                updateSettingsPatch({
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
                updateSettingsPatch({
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
