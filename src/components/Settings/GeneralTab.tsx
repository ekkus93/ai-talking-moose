import React from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { Keyboard } from "lucide-react";

export const GeneralTab: React.FC = () => {
  const { settings, updateSettings } = useMooseStore();
  if (!settings) return null;

  return (
    <div className="space-y-4">
      <h3 className="font-bold text-sm border-b border-black pb-1">
        Application Preferences
      </h3>
      <label className="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          checked={settings.launch_at_login}
          onChange={(e) =>
            updateSettings({
              ...settings,
              launch_at_login: e.target.checked,
            })
          }
          className="accent-black"
        />
        <span>Launch Talking Moose at system login</span>
      </label>
      <label className="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          checked={settings.show_in_menu_bar}
          onChange={(e) =>
            updateSettings({
              ...settings,
              show_in_menu_bar: e.target.checked,
            })
          }
          className="accent-black"
        />
        <span>Show Talking Moose in the system tray / menu bar</span>
      </label>
      <label className="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          checked={settings.always_on_top}
          onChange={(e) =>
            updateSettings({
              ...settings,
              always_on_top: e.target.checked,
            })
          }
          className="accent-black"
        />
        <span>Keep Moose window always on top</span>
      </label>
      <label className="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          checked={settings.restore_position}
          onChange={(e) =>
            updateSettings({
              ...settings,
              restore_position: e.target.checked,
            })
          }
          className="accent-black"
        />
        <span>Restore desktop window position across restarts</span>
      </label>

      <section
        className="border border-black rounded bg-[#fbf9f5] p-3 space-y-2"
        aria-labelledby="keyboard-shortcuts-heading"
      >
        <div
          className="flex items-center gap-1.5 font-bold"
          id="keyboard-shortcuts-heading"
        >
          <Keyboard className="w-3.5 h-3.5" aria-hidden="true" />
          Keyboard Shortcuts
        </div>
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-[11px]">
          <dt className="font-bold">Ctrl/Cmd + Enter</dt>
          <dd>Start or stop a conversation</dd>
          <dt className="font-bold">Ctrl/Cmd + Shift + M</dt>
          <dd>Mute or unmute Moose</dd>
          <dt className="font-bold">Ctrl/Cmd + ,</dt>
          <dd>Open Settings</dd>
          <dt className="font-bold">Escape</dt>
          <dd>Close the active panel</dd>
        </dl>
        <p className="text-[10px] text-gray-700">
          These shortcuts work only while the Moose window is focused. No global
          keyboard capture is registered. Show/hide remains available from the
          tray so a hidden window can be restored safely.
        </p>
      </section>
    </div>
  );
};
