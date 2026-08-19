import React, { useCallback, useEffect, useState } from "react";
import { AlertCircle, CheckCircle } from "lucide-react";
import { tauriBridge } from "../../lib/tauriBridge";
import type { MicrophonePermissionState } from "../../types/moose";

const permissionLabel: Record<MicrophonePermissionState, string> = {
  not_requested: "Not requested",
  granted: "Granted",
  denied: "Denied",
  unavailable: "Unavailable",
};

export const MicrophonePermissionCard: React.FC = () => {
  const [permission, setPermission] =
    useState<MicrophonePermissionState>("unavailable");
  const [isRequesting, setIsRequesting] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setPermission(await tauriBridge.getMicrophonePermission());
      setErrorMessage(null);
    } catch (error) {
      setErrorMessage(String(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const requestAccess = async () => {
    setIsRequesting(true);
    setErrorMessage(null);
    try {
      setPermission(await tauriBridge.requestMicrophoneAccess());
    } catch (error) {
      setErrorMessage(String(error));
    } finally {
      setIsRequesting(false);
    }
  };

  const granted = permission === "granted";

  return (
    <section className="border border-black rounded p-3 bg-[#fbf9f5] space-y-2">
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="font-bold">Microphone Access</p>
          <p className="text-[11px] text-gray-600">
            macOS permission is checked directly; this is not a saved preference.
          </p>
        </div>
        <div className="flex items-center gap-1.5 font-bold">
          {granted ? (
            <CheckCircle className="w-4 h-4 text-green-700" />
          ) : (
            <AlertCircle className="w-4 h-4 text-amber-700" />
          )}
          <span>{permissionLabel[permission]}</span>
        </div>
      </div>

      {permission === "not_requested" && (
        <button
          type="button"
          onClick={requestAccess}
          disabled={isRequesting}
          className="px-3 py-1.5 bg-black text-white rounded font-bold disabled:opacity-50"
        >
          {isRequesting ? "Requesting..." : "Request Microphone Access"}
        </button>
      )}

      {permission === "denied" && (
        <p className="text-[11px] text-amber-900">
          Access was denied or restricted. Open macOS System Settings → Privacy &
          Security → Microphone, enable Talking Moose, then return here and press
          Refresh Status.
        </p>
      )}

      {permission === "unavailable" && (
        <p className="text-[11px] text-gray-600">
          Native microphone permission status is unavailable on this platform or
          runtime.
        </p>
      )}

      {errorMessage && (
        <p role="alert" className="text-[11px] text-red-700">
          {errorMessage}
        </p>
      )}

      <button
        type="button"
        onClick={() => void refresh()}
        className="px-2 py-1 bg-white border border-black rounded font-bold hover:bg-gray-100"
      >
        Refresh Status
      </button>
    </section>
  );
};
