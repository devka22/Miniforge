#include "miniforge_native.h"

#include <cstdlib>
#include <cstring>
#include <string>

namespace {
const MiniForgeNativeHostV1* g_host = nullptr;

int32_t initialize(const MiniForgeNativeHostV1* host) {
    if (!host || host->abi_version != MINIFORGE_NATIVE_ABI_VERSION) return 10;
    g_host = host;
    if (g_host->log) g_host->log(1, "C++ example initialized");
    return 0;
}

void shutdown() { g_host = nullptr; }

int32_t invoke_json(const char* operation, const char* request, char** response) {
    if (!operation || !request || !response) return 20;
    const std::string json = std::string("{\"ok\":true,\"operation\":\"") + operation + "\"}";
    *response = static_cast<char*>(std::malloc(json.size() + 1));
    if (!*response) return 21;
    std::memcpy(*response, json.c_str(), json.size() + 1);
    return 0;
}

void free_string(char* value) { std::free(value); }

const MiniForgeNativePluginV1 plugin = {
    MINIFORGE_NATIVE_ABI_VERSION,
    sizeof(MiniForgeNativePluginV1),
    "MiniForge C++ Example",
    "1.0.0",
    "middleware",
    initialize,
    shutdown,
    invoke_json,
    free_string,
};
} // namespace

extern "C" MINIFORGE_NATIVE_EXPORT const MiniForgeNativePluginV1* miniforge_native_entry_v1() {
    return &plugin;
}

