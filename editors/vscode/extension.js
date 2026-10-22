// Khách LSP tối thiểu: chạy `gritc lsp` (stdio). KHUNG SƠ KHAI — chưa thử trong VS Code thật (Seed 1.0).
const { workspace } = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const gritc = workspace.getConfiguration("grit").get("gritcPath") || "gritc";
  const serverOptions = { command: gritc, args: ["lsp"] };
  const clientOptions = { documentSelector: [{ scheme: "file", language: "grit" }] };
  client = new LanguageClient("grit", "Grit", serverOptions, clientOptions);
  client.start();
  context.subscriptions.push({ dispose: () => client && client.stop() });
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
