import unittest

from scripts.check_whisper_build_policy import BuildPolicyError, validate_build_policy_text
from scripts.check_whisper_provenance import validate, validate_ffi_safety_text


VALID_BUILD_POLICY = """
const WHISPER_BUILD_JOBS: &str = "TALKING_MOOSE_WHISPER_BUILD_JOBS";

fn cpu_count() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
}

fn explicit_whisper_build_jobs() -> Option<usize> {
    None
}

fn whisper_build_jobs() -> usize {
    explicit_whisper_build_jobs().unwrap_or_else(cpu_count)
}

fn emit_whisper_native_rerun_paths(whisper_src: &Path) {
    for path in [
        whisper_src.join("CMakeLists.txt"),
        whisper_src.join("cmake"),
        whisper_src.join("include"),
        whisper_src.join("src"),
        whisper_src.join("ggml").join("CMakeLists.txt"),
        whisper_src.join("ggml").join("cmake"),
        whisper_src.join("ggml").join("include"),
        whisper_src.join("ggml").join("src"),
    ] {
        emit_rerun_tree(&path);
    }
}

fn build_whisper_from_source() {
    let threads = whisper_build_jobs().to_string();
}

fn main() {
    println!("cargo:rerun-if-env-changed={WHISPER_BUILD_JOBS}");
}
"""

VALID_FFI_SAFETY = """
struct WhisperAhead;
struct WhisperAheads;
struct WhisperVadParams;
struct WhisperGrammarElement;
struct WhisperGreedy;
struct WhisperBeamSearch;
struct WhisperFullParams;
struct WhisperContextParams;
pub(crate) struct WhisperModel;
// SAFETY: moving ownership is sound; shared concurrent access is deliberately not promised.
unsafe impl Send for WhisperModel {}
"""


class WhisperProvenanceTest(unittest.TestCase):
    def test_matching_revision_passes(self):
        sha = "1" * 40
        validate(sha, sha)

    def test_deliberately_wrong_revision_fails(self):
        with self.assertRaisesRegex(ValueError, "provenance mismatch"):
            validate("1" * 40, "2" * 40)

    def test_valid_build_policy_passes(self):
        validate_build_policy_text(VALID_BUILD_POLICY)

    def test_build_policy_rejects_proc_nproc_probe(self):
        with self.assertRaisesRegex(BuildPolicyError, "/proc/nproc"):
            validate_build_policy_text(VALID_BUILD_POLICY + '\nlet legacy = "/proc/nproc";\n')

    def test_build_policy_requires_operator_override(self):
        with self.assertRaisesRegex(BuildPolicyError, "parallelism override"):
            validate_build_policy_text(
                VALID_BUILD_POLICY.replace(
                    'const WHISPER_BUILD_JOBS: &str = "TALKING_MOOSE_WHISPER_BUILD_JOBS";\n',
                    "",
                )
            )

    def test_build_policy_requires_native_rerun_roots(self):
        with self.assertRaisesRegex(BuildPolicyError, "ggml.*include"):
            validate_build_policy_text(
                VALID_BUILD_POLICY.replace(
                    '        whisper_src.join("ggml").join("include"),\n',
                    "",
                )
            )

    def test_valid_ffi_safety_policy_passes(self):
        validate_ffi_safety_text(VALID_FFI_SAFETY)

    def test_ffi_safety_rejects_sync_model(self):
        with self.assertRaisesRegex(ValueError, "must not promise Sync"):
            validate_ffi_safety_text(
                VALID_FFI_SAFETY + "\nunsafe impl Sync for WhisperModel {}\n"
            )

    def test_ffi_safety_rejects_public_raw_c_type(self):
        with self.assertRaisesRegex(ValueError, "must remain private"):
            validate_ffi_safety_text(
                VALID_FFI_SAFETY.replace(
                    "struct WhisperFullParams;", "pub(crate) struct WhisperFullParams;"
                )
            )


if __name__ == "__main__":
    unittest.main()
