#include "fast_hash.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// -------------------------------------------------------------
// 1. FAST CRC-32 (IEEE 802.3, poly 0xEDB88320) with 256-entry table
// -------------------------------------------------------------
static unsigned int s_crc_table[256];
static int s_crc_table_initialized = 0;

static void init_crc_table(void) {
    if (s_crc_table_initialized) return;
    for (unsigned int i = 0; i < 256; i++) {
        unsigned int c = i;
        for (int j = 0; j < 8; j++) {
            c = (c & 1) ? (0xEDB88320U ^ (c >> 1)) : (c >> 1);
        }
        s_crc_table[i] = c;
    }
    s_crc_table_initialized = 1;
}

unsigned int c_fast_crc32(unsigned int initial_crc, const unsigned char* data, size_t length) {
    if (!s_crc_table_initialized) init_crc_table();
    unsigned int crc = ~initial_crc;

    // Process in unrolled 4-byte steps when possible
    while (length >= 4) {
        crc = s_crc_table[(crc ^ data[0]) & 0xFF] ^ (crc >> 8);
        crc = s_crc_table[(crc ^ data[1]) & 0xFF] ^ (crc >> 8);
        crc = s_crc_table[(crc ^ data[2]) & 0xFF] ^ (crc >> 8);
        crc = s_crc_table[(crc ^ data[3]) & 0xFF] ^ (crc >> 8);
        data += 4;
        length -= 4;
    }
    while (length > 0) {
        crc = s_crc_table[(crc ^ *data) & 0xFF] ^ (crc >> 8);
        data++;
        length--;
    }
    return ~crc;
}

// -------------------------------------------------------------
// 2. FAST SHA-256 (FIPS 180-4)
// -------------------------------------------------------------
typedef struct {
    unsigned int state[8];
    unsigned long long count;
    unsigned char buffer[64];
} CSha256Context;

static const unsigned int K256[64] = {
    0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
    0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
    0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
    0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
    0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
    0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
    0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
    0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
};

#define ROR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
#define CH(x, y, z) (((x) & (y)) ^ (~(x) & (z)))
#define MAJ(x, y, z) (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))
#define EP0(x) (ROR(x, 2) ^ ROR(x, 13) ^ ROR(x, 22))
#define EP1(x) (ROR(x, 6) ^ ROR(x, 11) ^ ROR(x, 25))
#define SIG0(x) (ROR(x, 7) ^ ROR(x, 18) ^ ((x) >> 3))
#define SIG1(x) (ROR(x, 17) ^ ROR(x, 19) ^ ((x) >> 10))

static void sha256_transform(CSha256Context* ctx, const unsigned char data[64]) {
    unsigned int a, b, c, d, e, f, g, h, t1, t2, m[64];
    for (int i = 0, j = 0; i < 16; ++i, j += 4) {
        m[i] = ((unsigned int)data[j] << 24) | ((unsigned int)data[j + 1] << 16) |
               ((unsigned int)data[j + 2] << 8) | ((unsigned int)data[j + 3]);
    }
    for (int i = 16; i < 64; ++i) {
        m[i] = SIG1(m[i - 2]) + m[i - 7] + SIG0(m[i - 15]) + m[i - 16];
    }
    a = ctx->state[0]; b = ctx->state[1]; c = ctx->state[2]; d = ctx->state[3];
    e = ctx->state[4]; f = ctx->state[5]; g = ctx->state[6]; h = ctx->state[7];

    for (int i = 0; i < 64; ++i) {
        t1 = h + EP1(e) + CH(e, f, g) + K256[i] + m[i];
        t2 = EP0(a) + MAJ(a, b, c);
        h = g; g = f; f = e; e = d + t1;
        d = c; c = b; b = a; a = t1 + t2;
    }

    ctx->state[0] += a; ctx->state[1] += b; ctx->state[2] += c; ctx->state[3] += d;
    ctx->state[4] += e; ctx->state[5] += f; ctx->state[6] += g; ctx->state[7] += h;
}

static void sha256_init(CSha256Context* ctx) {
    ctx->state[0] = 0x6a09e667; ctx->state[1] = 0xbb67ae85;
    ctx->state[2] = 0x3c6ef372; ctx->state[3] = 0xa54ff53a;
    ctx->state[4] = 0x510e527f; ctx->state[5] = 0x9b05688c;
    ctx->state[6] = 0x1f83d9ab; ctx->state[7] = 0x5be0cd19;
    ctx->count = 0;
}

static void sha256_update(CSha256Context* ctx, const unsigned char* data, size_t len) {
    size_t left = (size_t)(ctx->count & 0x3F);
    ctx->count += len;

    if (left > 0) {
        size_t fill = 64 - left;
        if (len >= fill) {
            memcpy(ctx->buffer + left, data, fill);
            sha256_transform(ctx, ctx->buffer);
            data += fill;
            len -= fill;
        } else {
            memcpy(ctx->buffer + left, data, len);
            return;
        }
    }

    while (len >= 64) {
        sha256_transform(ctx, data);
        data += 64;
        len -= 64;
    }

    if (len > 0) {
        memcpy(ctx->buffer, data, len);
    }
}

static void sha256_final(CSha256Context* ctx, unsigned char hash[32]) {
    unsigned char pad[64];
    pad[0] = 0x80;
    memset(pad + 1, 0, 63);

    unsigned long long bits = ctx->count * 8;
    unsigned char bits_be[8];
    for (int i = 0; i < 8; ++i) {
        bits_be[i] = (unsigned char)((bits >> ((7 - i) * 8)) & 0xFF);
    }

    size_t left = (size_t)(ctx->count & 0x3F);
    size_t padlen = (left < 56) ? (56 - left) : (120 - left);
    sha256_update(ctx, pad, padlen);
    sha256_update(ctx, bits_be, 8);

    for (int i = 0; i < 8; ++i) {
        hash[i * 4]     = (unsigned char)((ctx->state[i] >> 24) & 0xFF);
        hash[i * 4 + 1] = (unsigned char)((ctx->state[i] >> 16) & 0xFF);
        hash[i * 4 + 2] = (unsigned char)((ctx->state[i] >> 8) & 0xFF);
        hash[i * 4 + 3] = (unsigned char)(ctx->state[i] & 0xFF);
    }
}

void c_fast_sha256(const unsigned char* data, size_t length, unsigned char output[32]) {
    CSha256Context ctx;
    sha256_init(&ctx);
    sha256_update(&ctx, data, length);
    sha256_final(&ctx, output);
}

// -------------------------------------------------------------
// 3. FAST MD5 (RFC 1321)
// -------------------------------------------------------------
typedef struct {
    unsigned int state[4];
    unsigned long long count;
    unsigned char buffer[64];
} CMd5Context;

#define F_MD5(x, y, z) (((x) & (y)) | ((~x) & (z)))
#define G_MD5(x, y, z) (((x) & (z)) | ((y) & (~z)))
#define H_MD5(x, y, z) ((x) ^ (y) ^ (z))
#define I_MD5(x, y, z) ((y) ^ ((x) | (~z)))

#define ROTL(x, n) (((x) << (n)) | ((x) >> (32 - (n))))
#define FF(a, b, c, d, x, s, ac) { (a) += F_MD5((b), (c), (d)) + (x) + (unsigned int)(ac); (a) = ROTL((a), (s)); (a) += (b); }
#define GG(a, b, c, d, x, s, ac) { (a) += G_MD5((b), (c), (d)) + (x) + (unsigned int)(ac); (a) = ROTL((a), (s)); (a) += (b); }
#define HH(a, b, c, d, x, s, ac) { (a) += H_MD5((b), (c), (d)) + (x) + (unsigned int)(ac); (a) = ROTL((a), (s)); (a) += (b); }
#define II(a, b, c, d, x, s, ac) { (a) += I_MD5((b), (c), (d)) + (x) + (unsigned int)(ac); (a) = ROTL((a), (s)); (a) += (b); }

static void md5_transform(unsigned int state[4], const unsigned char block[64]) {
    unsigned int a = state[0], b = state[1], c = state[2], d = state[3], x[16];
    for (int i = 0, j = 0; i < 16; i++, j += 4) {
        x[i] = (unsigned int)block[j] | ((unsigned int)block[j+1] << 8) |
               ((unsigned int)block[j+2] << 16) | ((unsigned int)block[j+3] << 24);
    }
    // Round 1
    FF(a, b, c, d, x[ 0],  7, 0xd76aa478); FF(d, a, b, c, x[ 1], 12, 0xe8c7b756);
    FF(c, d, a, b, x[ 2], 17, 0x242070db); FF(b, c, d, a, x[ 3], 22, 0xc1bdceee);
    FF(a, b, c, d, x[ 4],  7, 0xf57c0faf); FF(d, a, b, c, x[ 5], 12, 0x4787c62a);
    FF(c, d, a, b, x[ 6], 17, 0xa8304613); FF(b, c, d, a, x[ 7], 22, 0xfd469501);
    FF(a, b, c, d, x[ 8],  7, 0x698098d8); FF(d, a, b, c, x[ 9], 12, 0x8b44f7af);
    FF(c, d, a, b, x[10], 17, 0xffff5bb1); FF(b, c, d, a, x[11], 22, 0x895cd7be);
    FF(a, b, c, d, x[12],  7, 0x6b901122); FF(d, a, b, c, x[13], 12, 0xfd987193);
    FF(c, d, a, b, x[14], 17, 0xa679438e); FF(b, c, d, a, x[15], 22, 0x49b40821);
    // Round 2
    GG(a, b, c, d, x[ 1],  5, 0xf61e2562); GG(d, a, b, c, x[ 6],  9, 0xc040b340);
    GG(c, d, a, b, x[11], 14, 0x265e5a51); GG(b, c, d, a, x[ 0], 20, 0xe9b6c7aa);
    GG(a, b, c, d, x[ 5],  5, 0xd62f105d); GG(d, a, b, c, x[10],  9, 0x02441453);
    GG(c, d, a, b, x[15], 14, 0xd8a1e681); GG(b, c, d, a, x[ 4], 20, 0xe7d3fbc8);
    GG(a, b, c, d, x[ 9],  5, 0x21e1cde6); GG(d, a, b, c, x[14],  9, 0xc33707d6);
    GG(c, d, a, b, x[ 3], 14, 0xf4d50d87); GG(b, c, d, a, x[ 8], 20, 0x455a14ed);
    GG(a, b, c, d, x[13],  5, 0xa9e3e905); GG(d, a, b, c, x[ 2],  9, 0xfcefa3f8);
    GG(c, d, a, b, x[ 7], 14, 0x676f02d9); GG(b, c, d, a, x[12], 20, 0x8d2a4c8a);
    // Round 3
    HH(a, b, c, d, x[ 5],  4, 0xfffa3942); HH(d, a, b, c, x[ 8], 11, 0x8771f681);
    HH(c, d, a, b, x[11], 16, 0x6d9d6122); HH(b, c, d, a, x[14], 23, 0xfde5380c);
    HH(a, b, c, d, x[ 1],  4, 0xa4beea44); HH(d, a, b, c, x[ 4], 11, 0x4bdecfa9);
    HH(c, d, a, b, x[ 7], 16, 0xf6bb4b60); HH(b, c, d, a, x[10], 23, 0xbebfbc70);
    HH(a, b, c, d, x[13],  4, 0x289b7ec6); HH(d, a, b, c, x[ 0], 11, 0xeaa127fa);
    HH(c, d, a, b, x[ 3], 16, 0xd4ef3085); HH(b, c, d, a, x[ 6], 23, 0x04881d05);
    HH(a, b, c, d, x[ 9],  4, 0xd9d4d039); HH(d, a, b, c, x[12], 11, 0xe6db99e5);
    HH(c, d, a, b, x[15], 16, 0x1fa27cf8); HH(b, c, d, a, x[ 2], 23, 0xc4ac5665);
    // Round 4
    II(a, b, c, d, x[ 0],  6, 0xf4292244); II(d, a, b, c, x[ 7], 10, 0x432aff97);
    II(c, d, a, b, x[14], 15, 0xab9423a7); II(b, c, d, a, x[ 5], 21, 0xfc93a039);
    II(a, b, c, d, x[12],  6, 0x655b59c3); II(d, a, b, c, x[ 3], 10, 0x8f0ccc92);
    II(c, d, a, b, x[10], 15, 0xffeff47d); II(b, c, d, a, x[ 1], 21, 0x85845dd1);
    II(a, b, c, d, x[ 8],  6, 0x6fa87e4f); II(d, a, b, c, x[15], 10, 0xfe2ce6e0);
    II(c, d, a, b, x[ 6], 15, 0xa3014314); II(b, c, d, a, x[13], 21, 0x4e0811a1);
    II(a, b, c, d, x[ 4],  6, 0xf7537e82); II(d, a, b, c, x[11], 10, 0xbd3af235);
    II(c, d, a, b, x[ 2], 15, 0x2ad7d2bb); II(b, c, d, a, x[ 9], 21, 0xeb86d391);

    state[0] += a; state[1] += b; state[2] += c; state[3] += d;
}

static void md5_init(CMd5Context* ctx) {
    ctx->state[0] = 0x67452301; ctx->state[1] = 0xefcdab89;
    ctx->state[2] = 0x98badcfe; ctx->state[3] = 0x10325476;
    ctx->count = 0;
}

static void md5_update(CMd5Context* ctx, const unsigned char* data, size_t len) {
    size_t left = (size_t)(ctx->count & 0x3F);
    ctx->count += len;

    if (left > 0) {
        size_t fill = 64 - left;
        if (len >= fill) {
            memcpy(ctx->buffer + left, data, fill);
            md5_transform(ctx->state, ctx->buffer);
            data += fill;
            len -= fill;
        } else {
            memcpy(ctx->buffer + left, data, len);
            return;
        }
    }

    while (len >= 64) {
        md5_transform(ctx->state, data);
        data += 64;
        len -= 64;
    }

    if (len > 0) {
        memcpy(ctx->buffer, data, len);
    }
}

static void md5_final(CMd5Context* ctx, unsigned char hash[16]) {
    unsigned char pad[64];
    pad[0] = 0x80;
    memset(pad + 1, 0, 63);

    unsigned long long bits = ctx->count * 8;
    unsigned char bits_le[8];
    for (int i = 0; i < 8; ++i) {
        bits_le[i] = (unsigned char)((bits >> (i * 8)) & 0xFF);
    }

    size_t left = (size_t)(ctx->count & 0x3F);
    size_t padlen = (left < 56) ? (56 - left) : (120 - left);
    md5_update(ctx, pad, padlen);
    md5_update(ctx, bits_le, 8);

    for (int i = 0; i < 4; ++i) {
        hash[i * 4]     = (unsigned char)(ctx->state[i] & 0xFF);
        hash[i * 4 + 1] = (unsigned char)((ctx->state[i] >> 8) & 0xFF);
        hash[i * 4 + 2] = (unsigned char)((ctx->state[i] >> 16) & 0xFF);
        hash[i * 4 + 3] = (unsigned char)((ctx->state[i] >> 24) & 0xFF);
    }
}

void c_fast_md5(const unsigned char* data, size_t length, unsigned char output[16]) {
    CMd5Context ctx;
    md5_init(&ctx);
    md5_update(&ctx, data, length);
    md5_final(&ctx, output);
}

// -------------------------------------------------------------
// 4. HIGH-SPEED WHOLE-FILE STREAMING HASHER (SHA-256 + MD5 + CRC32)
// -------------------------------------------------------------
int c_fast_hash_file(
    const wchar_t* file_path,
    unsigned char sha256_out[32],
    unsigned char md5_out[16],
    unsigned int* crc32_out,
    CHashProgressCallback progress_cb,
    const volatile int* cancel_flag
) {
    if (!file_path || !sha256_out || !md5_out || !crc32_out) return -1;

    HANDLE hFile = CreateFileW(
        file_path,
        GENERIC_READ,
        FILE_SHARE_READ,
        NULL,
        OPEN_EXISTING,
        FILE_FLAG_SEQUENTIAL_SCAN,
        NULL
    );

    if (hFile == INVALID_HANDLE_VALUE) {
        return (int)GetLastError();
    }

    LARGE_INTEGER size_li;
    if (!GetFileSizeEx(hFile, &size_li)) {
        CloseHandle(hFile);
        return (int)GetLastError();
    }
    unsigned long long total_bytes = (unsigned long long)size_li.QuadPart;

    // Use 4 MiB buffer for high-throughput sequential reading
    const DWORD CHUNK_SIZE = 4 * 1024 * 1024;
    unsigned char* buffer = (unsigned char*)malloc(CHUNK_SIZE);
    if (!buffer) {
        CloseHandle(hFile);
        return -2;
    }

    CSha256Context sha_ctx;
    sha256_init(&sha_ctx);

    CMd5Context md5_ctx;
    md5_init(&md5_ctx);

    init_crc_table();
    unsigned int running_crc = 0;

    unsigned long long processed_bytes = 0;
    DWORD bytes_read = 0;
    int cancelled = 0;

    while (ReadFile(hFile, buffer, CHUNK_SIZE, &bytes_read, NULL) && bytes_read > 0) {
        if (cancel_flag && *cancel_flag != 0) {
            cancelled = 1;
            break;
        }

        // Concurrently update all three digests
        sha256_update(&sha_ctx, buffer, bytes_read);
        md5_update(&md5_ctx, buffer, bytes_read);
        running_crc = c_fast_crc32(running_crc, buffer, bytes_read);

        processed_bytes += bytes_read;
        if (progress_cb) {
            progress_cb(processed_bytes, total_bytes);
        }
    }

    free(buffer);
    CloseHandle(hFile);

    if (cancelled) {
        return -3; // Cancelled
    }

    sha256_final(&sha_ctx, sha256_out);
    md5_final(&md5_ctx, md5_out);
    *crc32_out = running_crc;

    return 0;
}
