import React from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { Trash2 } from "lucide-react";

export const DataTab: React.FC = () => {
  const { memories, deleteMemory, forgetEverything } = useMooseStore();

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center border-b border-black pb-1">
        <h3 className="font-bold text-sm">
          Stored Memories ({memories.length})
        </h3>
        <button
          onClick={forgetEverything}
          className="px-2 py-1 bg-red-600 text-white rounded font-bold text-[11px] hover:bg-red-700 flex items-center gap-1"
        >
          <Trash2 className="w-3 h-3" />
          <span>Forget Everything</span>
        </button>
      </div>

      <div className="max-h-60 overflow-y-auto space-y-1.5">
        {memories.length === 0 ? (
          <div className="text-gray-500 italic text-center py-6">
            No memories saved yet.
          </div>
        ) : (
          memories.map((m) => (
            <div
              key={m.id}
              className="p-2 border border-black rounded flex justify-between items-center bg-[#fbf9f5]"
            >
              <div>
                <p className="font-bold">{m.fact}</p>
                <p className="text-[10px] text-gray-500">{m.created_at}</p>
              </div>
              <button
                onClick={() => deleteMemory(m.id)}
                title={`Forget: ${m.fact}`}
                aria-label={`Forget memory: ${m.fact}`}
                className="hover:bg-red-100 p-1 rounded text-red-600"
              >
                <Trash2 className="w-3.5 h-3.5" aria-hidden="true" />
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
};
