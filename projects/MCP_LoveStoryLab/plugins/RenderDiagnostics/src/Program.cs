using System.Text.Json;
using System.Text.Json.Nodes;

static int CountFiles(string root, string pattern)
{
    return Directory.Exists(root)
        ? Directory.EnumerateFiles(root, pattern, SearchOption.AllDirectories).Count()
        : 0;
}

var input = Console.In.ReadToEnd();
JsonNode? request = null;

try
{
    request = string.IsNullOrWhiteSpace(input) ? null : JsonNode.Parse(input);
}
catch (JsonException)
{
    request = null;
}

var projectRoot = request?["context"]?["project_root"]?.GetValue<string>() ?? Environment.CurrentDirectory;
var assetsRoot = Path.Combine(projectRoot, "assets");
var scenesRoot = Path.Combine(projectRoot, "saves", "scenes");
var scriptsRoot = Path.Combine(projectRoot, "scripts");
var spriteCount = CountFiles(assetsRoot, "*.png") + CountFiles(assetsRoot, "*.jpg") + CountFiles(assetsRoot, "*.webp");
var audioCount = CountFiles(assetsRoot, "*.wav") + CountFiles(assetsRoot, "*.ogg") + CountFiles(assetsRoot, "*.mp3");
var sceneCount = CountFiles(scenesRoot, "*.scene");
var scriptCount = CountFiles(scriptsRoot, "*.luau") + CountFiles(scriptsRoot, "*.mfgraph");
var health = 100;
if (!Directory.Exists(projectRoot))
{
    health = 0;
}
else
{
    if (sceneCount == 0) health -= 30;
    if (spriteCount == 0) health -= 20;
    if (scriptCount == 0) health -= 15;
    if (audioCount == 0) health -= 10;
}

var result = new JsonObject
{
    ["success"] = Directory.Exists(projectRoot),
    ["message"] = "RenderDiagnostics C# plugin bridge online",
    ["health"] = Math.Max(0, health),
    ["summary"] = new JsonObject
    {
        ["project_root"] = projectRoot,
        ["sprites"] = spriteCount,
        ["audio"] = audioCount,
        ["scenes"] = sceneCount,
        ["scripts"] = scriptCount
    },
    ["operations"] = new JsonArray
    {
        new JsonObject
        {
            ["operation"] = "log",
            ["value"] = $"RenderDiagnostics inspected {projectRoot} | health {Math.Max(0, health)}%"
        }
    },
    ["generated_files"] = new JsonArray()
};

Console.WriteLine(result.ToJsonString(new JsonSerializerOptions { WriteIndented = false }));
