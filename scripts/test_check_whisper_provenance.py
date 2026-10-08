import unittest

from scripts.check_whisper_build_policy import BuildPolicyError, validate_build_policy_text
from scripts.check_whisper_acceptance_workflow import WorkflowPolicyError, validate_shell_continuations
from scripts.check_whisper_provenance import (
    validate,
    validate_ffi_safety_text,
    validate_release_notice_policy_text,
)


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

fn emit_rerun_tree(root: &Path) {
    if let Ok(metadata) = std::fs::symlink_metadata(root) {
        if !metadata.is_dir() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        let _ = std::fs::read_dir(root);
    }
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
    let configure = Command::new("cmake").output();
    let Ok(configure) = configure else {
        return;
    };
    if !configure.status.success() {
        return;
    }
    let make = Command::new("make").status();
    let Ok(make) = make else {
        return;
    };
    if !make.success() {
        return;
    }
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
    def test_release_preparation_stages_pinned_whisper_license(self):
        validate_release_notice_policy_text(
            "python3 scripts/check_whisper_provenance.py\n"
            "cp $repo_root/third_party/whisper.cpp/LICENSE path/WHISPER_CPP_LICENSE\n"
        )

    def test_release_preparation_must_keep_whisper_license(self):
        with self.assertRaisesRegex(ValueError, "WHISPER_CPP_LICENSE"):
            validate_release_notice_policy_text(
                "python3 scripts/check_whisper_provenance.py\n"
                "cp $repo_root/third_party/whisper.cpp/LICENSE path/license\n"
            )

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

    def test_build_policy_requires_directory_watches_for_added_files(self):
        with self.assertRaisesRegex(BuildPolicyError, "added files"):
            validate_build_policy_text(
                VALID_BUILD_POLICY.replace(
                    '        println!("cargo:rerun-if-changed={}", path.display());\n',
                    "",
                )
            )

    def test_build_policy_requires_fail_closed_native_tool_handling(self):
        with self.assertRaisesRegex(BuildPolicyError, "native build spawn"):
            validate_build_policy_text(
                VALID_BUILD_POLICY.replace(
                    "    let Ok(make) = make else {\n        return;\n    };\n",
                    "    if let Ok(make) = make {\n        let _ = make.success();\n    }\n",
                )
            )

    def test_acceptance_shell_continuation_rejects_blank_line(self):
        broken = "run: |\n  env " + chr(92) + "\n\n  FOO=bar\n"
        with self.assertRaisesRegex(WorkflowPolicyError, "blank line after shell continuation"):
            validate_shell_continuations(broken)

    def test_acceptance_shell_continuation_accepts_adjacent_line(self):
        valid = "run: |\n  env " + chr(92) + "\n  FOO=bar\n"
        validate_shell_continuations(valid)

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
