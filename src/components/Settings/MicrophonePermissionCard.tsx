import React, { useCallback, useEffect, useRef, useState } from "react";
import { AlertCircle, CheckCircle, RefreshCw } from "lucide-react";
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
    useState<MicrophonePermissionState | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(true);
  const [isRequesting, setIsRequesting] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const refreshRequestId = useRef(0);
  const requestInFlight = useRef(false);

  const refresh = useCallback(async () => {
    if (requestInFlight.current) return;
    const requestId = ++refreshRequestId.current;
    setIsRefreshing(true);
    try {
      const nextPermission = await tauriBridge.getMicrophonePermission();
      if (requestId !== refreshRequestId.current) return;
      setPermission(nextPermission);
      setErrorMessage(null);
    } catch (error) {
      if (requestId !== refreshRequestId.current) return;
      setErrorMessage(String(error));
    } finally {
      if (requestId === refreshRequestId.current) {
        setIsRefreshing(false);
      }
    }
  }, []);

  useEffect(() => {
    void refresh();

    const refreshWhenFocused = () => {
      void refresh();
    };
    const refreshWhenVisible = () => {
      if (document.visibilityState === "visible") {
        void refresh();
      }
    };

    window.addEventListener("focus", refreshWhenFocused);
    document.addEventListener("visibilitychange", refreshWhenVisible);

    return () => {
      window.removeEventListener("focus", refreshWhenFocused);
      document.removeEventListener("visibilitychange", refreshWhenVisible);
      refreshRequestId.current += 1;
    };
  }, [refresh]);

  const requestAccess = async () => {
    requestInFlight.current = true;
    refreshRequestId.current += 1;
    setIsRefreshing(false);
    setIsRequesting(true);
    setErrorMessage(null);
    try {
      const nextPermission = await tauriBridge.requestMicrophoneAccess();
      setPermission(nextPermission);
    } catch (error) {
      setErrorMessage(String(error));
    } finally {
      requestInFlight.current = false;
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
            macOS permission is checked directly; this is not a saved
            preference.
          </p>
        </div>
        <div
          className="flex items-center gap-1.5 font-bold"
          role="status"
          aria-live="polite"
        >
          {permission === null ? (
            <>
              <RefreshCw className="w-4 h-4" aria-hidden="true" />
              <span>Checking…</span>
            </>
          ) : (
            <>
              {granted ? (
                <CheckCircle
                  className="w-4 h-4 text-green-700"
                  aria-hidden="true"
                />
              ) : (
                <AlertCircle
                  className="w-4 h-4 text-amber-700"
                  aria-hidden="true"
                />
              )}
              <span>{permissionLabel[permission]}</span>
            </>
          )}
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
          Access was denied or restricted. Open macOS System Settings → Privacy
          & Security → Microphone, enable Talking Moose, then return here; the
          status updates automatically when this window regains focus.
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
        disabled={isRefreshing}
        className="px-2 py-1 bg-white border border-black rounded font-bold hover:bg-gray-100 disabled:opacity-50"
      >
        {isRefreshing ? "Refreshing..." : "Refresh Status"}
      </button>
    </section>
  );
};
