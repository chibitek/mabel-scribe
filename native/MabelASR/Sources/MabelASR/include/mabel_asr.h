#pragma once
/* C ABI mirror for the Rust FFI. Not compiled into the Swift package
 * (Package.swift excludes this header so the dylib stays a pure Swift
 * dynamic library). */

#ifdef __cplusplus
extern "C" {
#endif

/* Progress is percent complete in [0, 100]. May be called from a background queue. */
typedef void (*mabel_asr_progress_cb)(double percent, void *user);

/* 1 = ready, 0 = not on disk, -1 = error (see mabel_asr_last_error). */
int mabel_asr_parakeet_ready(const char *version);
int mabel_asr_parakeet_download(const char *version, mabel_asr_progress_cb cb, void *user);
int mabel_asr_parakeet_transcribe(
    const char *version,
    const char *wav_path,
    const char *language,
    char **out_text
);

int mabel_asr_whisperkit_ready(const char *cache_dir);
int mabel_asr_whisperkit_download(const char *cache_dir, mabel_asr_progress_cb cb, void *user);
int mabel_asr_whisperkit_transcribe(
    const char *cache_dir,
    const char *wav_path,
    const char *language,
    char **out_text
);

void mabel_asr_string_free(char *s);
const char *mabel_asr_last_error(void);

#ifdef __cplusplus
}
#endif
