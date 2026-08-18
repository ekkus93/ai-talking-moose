# Talking Moose AI — Privacy & Security Architecture

## 1. Principles

1. **Local-First & Data Minimization:** Raw desktop contents, keystrokes, clipboard, and browser history are strictly never accessed or collected.
2. **Opt-In Permissions:** Desktop awareness (such as application switching patterns) and memory persistence are explicitly granular and can be disabled at any time.
3. **Zero Secret Exposure:** Google API credentials are stored only in native Rust memory / secure settings and are permanently redacted from all application logs and frontend storage.
4. **Complete Erasure ("Forget Everything"):** Users can inspect every recorded fact and instantly purge all local SQLite memory, conversation history, and observations.
5. **No Continuous Hidden Mic Capture:** The microphone is only opened during active user-initiated voice conversations.
