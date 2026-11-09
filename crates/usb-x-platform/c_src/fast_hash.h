#ifndef USB_X_FAST_HASH_H
#define USB_X_FAST_HASH_H

#include <windows.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*CHashProgressCallback)(unsigned long long current_bytes, unsigned long long total_bytes);

// 1. In-memory hash computation
unsigned int c_fast_crc32(unsigned int initial_crc, const unsigned char* data, size_t length);
void c_fast_sha256(const unsigned char* data, size_t length, unsigned char output[32]);
void c_fast_md5(const unsigned char* data, size_t length, unsigned char output[16]);

// 2. High-speed single-pass whole-file hasher
int c_fast_hash_file(
    const wchar_t* file_path,
    unsigned char sha256_out[32],
    unsigned char md5_out[16],
    unsigned int* crc32_out,
    CHashProgressCallback progress_cb,
    const volatile int* cancel_flag
);

#ifdef __cplusplus
}
#endif

#endif // USB_X_FAST_HASH_H

