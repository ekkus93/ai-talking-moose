import React from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { AudioDiagnosticsPanel } from "./AudioDiagnosticsPanel";

export const DiagnosticsTab: React.FC = () => {
  const { triggerCanned } = useMooseStore();

  return (
    <div className="space-y-6">
      <section className="space-y-3">
        <h3 className="font-bold text-sm border-b border-black pb-1">
          Audio Diagnostics
        </h3>
        <AudioDiagnosticsPanel />
      </section>

      <section className="space-y-3">
        <h3 className="font-bold text-sm border-b border-black pb-1">
          Offline Canned Utterance Tests
        </h3>
        <div className="grid grid-cols-2 gap-2">
          <button
            onClick={() => triggerCanned("greeting")}
            className="p-2 bg-white border border-black rounded font-bold hover:bg-gray-100"
          >
            Test Greeting
          </button>
          <button
            onClick={() => triggerCanned("click")}
            className="p-2 bg-white border border-black rounded font-bold hover:bg-gray-100"
          >
            Test Click Remark
          </button>
          <button
            onClick={() => triggerCanned("dismiss")}
            className="p-2 bg-white border border-black rounded font-bold hover:bg-gray-100"
          >
            Test Dismiss Remark
          </button>
          <button
            onClick={() => triggerCanned("error")}
            className="p-2 bg-white border border-black rounded font-bold hover:bg-gray-100"
          >
            Test Error Remark
          </button>
        </div>
      </section>
    </div>
  );
};
