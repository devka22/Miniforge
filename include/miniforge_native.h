#ifndef MINIFORGE_NATIVE_H
#define MINIFORGE_NATIVE_H

#include <stdint.h>

#if defined(_WIN32)
#  define MINIFORGE_NATIVE_EXPORT __declspec(dllexport)
#else
#  define MINIFORGE_NATIVE_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define MINIFORGE_NATIVE_ABI_VERSION 1u

typedef struct MiniForgeNativeHostV1 {
    uint32_t abi_version;
    uint32_t struct_size;
    void* user_data;
    void (*log)(uint32_t level, const char* message);
} MiniForgeNativeHostV1;

typedef struct MiniForgeNativePluginV1 {
    uint32_t abi_version;
    uint32_t struct_size;
    const char* name;
    const char* version;
    const char* category;
    int32_t (*initialize)(const MiniForgeNativeHostV1* host);
    void (*shutdown)(void);
    int32_t (*invoke_json)(const char* operation, const char* request_json, char** response_json);
    void (*free_string)(char* value);
} MiniForgeNativePluginV1;

MINIFORGE_NATIVE_EXPORT const MiniForgeNativePluginV1* miniforge_native_entry_v1(void);

#ifdef __cplusplus
}
#endif

#endif

