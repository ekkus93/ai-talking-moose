import React, { useCallback, useEffect, useState } from "react";
import { tauriBridge } from "../../lib/tauriBridge";
import type { ToolAuditRecord } from "../../types/moose";

const readable = (value: string) => value.replace(/_/g, " ");

export const ToolAuditDiagnosticsPanel: React.FC = () => {
  const [records, setRecords] = useState<ToolAuditRecord[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      setRecords(await tauriBridge.getToolAudit());
      setErrorMessage(null);
    } catch (error) {
      setErrorMessage(String(error));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const visibleRecords = records.slice(-20).reverse();

  return (
    <section className="border border-black rounded bg-[#fbf9f5] p-3 space-y-3">
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <p className="font-bold">Tool Activity</p>
          <p className="text-[11px] text-gray-600">
            Session-local audit metadata only. Raw tool arguments and results
            are never included.
          </p>
        </div>
        <button
          type="button"
          onClick={() => void refresh()}
          disabled={isLoading}
          className="px-2 py-1 bg-white border border-black rounded font-bold hover:bg-gray-100 disabled:opacity-50"
        >
          {isLoading ? "Refreshing..." : "Refresh"}
        </button>
      </div>

      {errorMessage && (
        <p role="alert" className="text-[11px] text-red-700">
          {errorMessage}
        </p>
      )}

      {isLoading && records.length === 0 ? (
        <p role="status" className="text-[11px] text-gray-600">
          Loading tool activity…
        </p>
      ) : visibleRecords.length === 0 ? (
        <p className="text-[11px] text-gray-600">
          No tool calls recorded this session.
        </p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-[11px] border-collapse">
            <thead>
              <tr className="text-left border-b border-black">
                <th className="py-1 pr-2">Time</th>
                <th className="py-1 pr-2">Tool</th>
                <th className="py-1 pr-2">Permission</th>
                <th className="py-1 pr-2">Result</th>
                <th className="py-1 text-right">Duration</th>
              </tr>
            </thead>
            <tbody>
              {visibleRecords.map((record, index) => (
                <tr
                  key={`${record.timestamp}-${record.tool_name}-${index}`}
                  className="border-b border-gray-300 last:border-b-0"
                >
                  <td className="py-1 pr-2 whitespace-nowrap">
                    <time dateTime={record.timestamp}>{record.timestamp}</time>
                  </td>
                  <td className="py-1 pr-2 font-bold">{record.tool_name}</td>
                  <td className="py-1 pr-2">
                    {readable(record.permission_outcome)}
                  </td>
                  <td className="py-1 pr-2">
                    {readable(record.result_category)}
                  </td>
                  <td className="py-1 text-right whitespace-nowrap">
                    {record.duration_ms} ms
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
};
