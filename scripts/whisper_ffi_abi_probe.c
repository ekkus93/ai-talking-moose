/* Compile-only ABI check against pinned whisper.h on Linux/macOS.
 * Constants mirror repr(C) checks in src-tauri/src/asr/whisper/ffi.rs.
 */
#include <stddef.h>
#include "whisper.h"
_Static_assert(sizeof(struct whisper_full_params) == 304, "Whisper full params ABI");
_Static_assert(_Alignof(struct whisper_full_params) == 8, "Whisper full params alignment");
_Static_assert(sizeof(struct whisper_context_params) == 48, "Whisper context params ABI");
_Static_assert(sizeof(struct whisper_aheads) == 16, "Whisper aheads ABI");
_Static_assert(sizeof(struct whisper_ahead) == 8, "Whisper ahead ABI");
_Static_assert(sizeof(struct whisper_vad_params) == 24, "Whisper VAD params ABI");
_Static_assert(sizeof(struct whisper_grammar_element) == 8, "Whisper grammar ABI");
_Static_assert(offsetof(struct whisper_full_params, language) == 104, "Whisper language offset");
_Static_assert(offsetof(struct whisper_full_params, vad_params) == 280, "Whisper VAD offset");
_Static_assert(offsetof(struct whisper_context_params, dtw_aheads) == 24, "Whisper DTW offset");
