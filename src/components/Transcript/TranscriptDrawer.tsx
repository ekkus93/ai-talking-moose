import React, { useState, useRef, useEffect } from "react";
import { useMooseStore } from "../../stores/mooseStore";
import { Terminal, X, Send, Trash2, CornerDownLeft } from "lucide-react";

export const TranscriptDrawer: React.FC = () => {
  const { isTranscriptOpen, toggleTranscript, transcripts, sendTextMessage, forgetEverything } =
    useMooseStore();

  const [inputMessage, setInputMessage] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isTranscriptOpen) {
      inputRef.current?.focus();
      if (scrollRef.current) {
        scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
      }
    }
  }, [isTranscriptOpen, transcripts]);

  if (!isTranscriptOpen) {
    return null;
  }

  const handleSend = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const msg = inputMessage.trim();
    if (!msg || isSubmitting) return;

    setIsSubmitting(true);
    setInputMessage("");
    try {
      await sendTextMessage(msg);
    } catch (err) {
      console.error("Failed to send message:", err);
    } finally {
      setIsSubmitting(false);
      inputRef.current?.focus();
    }
  };

  const handleChipClick = (chipText: string) => {
    setInputMessage(chipText);
    inputRef.current?.focus();
  };

  return (
    <div
      data-testid="transcript-drawer"
      className="absolute inset-0 z-30 bg-[#ece7de] flex flex-col border-2 border-black animate-in slide-in-from-bottom-5 duration-150 select-none font-mono text-xs"
    >
      {/* Terminal Title Bar */}
      <div className="flex items-center justify-between px-3 py-2 bg-black text-white text-xs font-bold">
        <div className="flex items-center gap-1.5">
          <Terminal className="w-4 h-4 text-green-400" />
          <span>MOOSE DEBUG TERMINAL</span>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => forgetEverything()}
            className="text-gray-400 hover:text-red-400 p-0.5"
            title="Clear All Logs"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => toggleTranscript(false)}
            className="hover:bg-gray-800 p-0.5 rounded"
            title="Close Terminal"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Suggested Fast Prompts */}
      <div className="px-2.5 py-1.5 bg-[#ded9cf] border-b border-black/30 flex gap-1.5 overflow-x-auto text-[10px] whitespace-nowrap">
        {["Who are you?", "Tell me a joke", "What time is it?", "Why a moose?"].map((chip) => (
          <button
            key={chip}
            onClick={() => handleChipClick(chip)}
            className="px-2 py-0.5 bg-white border border-black rounded shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] hover:bg-gray-100 active:translate-y-0.5"
          >
            {chip}
          </button>
        ))}
      </div>

      {/* Transcript Log Output Area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-3 space-y-2.5 select-text">
        {transcripts.length === 0 ? (
          <div className="text-center text-gray-500 italic py-8 space-y-1">
            <p className="font-bold">Interactive Debug Terminal</p>
            <p className="text-[11px]">
              Type a prompt below to bypass microphone input and test Gemini responses & speech!
            </p>
          </div>
        ) : (
          transcripts.map((entry) => (
            <div
              key={entry.id}
              className={`p-2.5 rounded border-2 border-black shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] ${
                entry.role === "moose" ? "bg-[#fff9e6] ml-2" : "bg-white mr-2"
              }`}
            >
              <div className="flex items-center justify-between text-[10px] font-bold text-gray-700 mb-1 border-b border-gray-200 pb-0.5">
                <span className={entry.role === "moose" ? "text-amber-900" : "text-blue-900"}>
                  {entry.role === "moose" ? "🫎 MOOSE" : "👤 YOU"}
                </span>
                <span>{entry.created_at}</span>
              </div>
              <p className="text-gray-900 leading-snug font-mono text-xs">{entry.text}</p>
            </div>
          ))
        )}
      </div>

      {/* Interactive Input Prompt Bar */}
      <form
        onSubmit={handleSend}
        className="p-2.5 bg-[#dcd6cd] border-t-2 border-black flex gap-1.5 items-center"
      >
        <span className="text-green-700 font-bold text-sm select-none">&gt;</span>
        <input
          ref={inputRef}
          type="text"
          placeholder="Type a message to Moose..."
          value={inputMessage}
          onChange={(e) => setInputMessage(e.target.value)}
          disabled={isSubmitting}
          className="flex-1 px-2 py-1.5 border-2 border-black rounded bg-white font-mono text-xs select-text focus:outline-none focus:ring-1 focus:ring-black"
        />
        <button
          type="submit"
          disabled={!inputMessage.trim() || isSubmitting}
          className="px-3 py-1.5 bg-black text-white font-bold rounded border-2 border-black shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] hover:bg-gray-800 disabled:opacity-40 flex items-center gap-1 active:translate-y-0.5"
          title="Send Message"
        >
          {isSubmitting ? (
            <span className="animate-spin text-xs">...</span>
          ) : (
            <>
              <Send className="w-3 h-3" />
              <CornerDownLeft className="w-3 h-3" />
            </>
          )}
        </button>
      </form>
    </div>
  );
};
