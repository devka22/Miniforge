miniforge.editor.registerCommand({
  id: "hello-plugin.say_hello",
  label: "Say Hello",
  category: "Plugins",
  async run() {
    miniforge.notifications.show({
      title: "Hello from MiniForge",
      message: "The TypeScript plugin API contract is wired for future QuickJS hosting.",
      severity: "info"
    });
  }
});
