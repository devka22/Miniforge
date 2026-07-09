declare global {
  const miniforge: MiniForgePluginHost;
}

export type MfEntityId = bigint;
export type MfAssetId = string;
export type MfCommandId = string;
export type MfPermission =
  | "scene.read"
  | "scene.write"
  | "assets.read"
  | "assets.write"
  | "commands.execute"
  | "ui.panels"
  | "notifications";

export interface MiniForgePluginManifest {
  name: string;
  version: string;
  apiVersion: "1";
  main?: string;
  permissions: MfPermission[];
  activationEvents?: string[];
}

export interface MiniForgePluginHost {
  readonly apiVersion: "1";
  readonly editor: MiniForgeEditorApi;
  readonly scene: MiniForgeSceneApi;
  readonly assets: MiniForgeAssetsApi;
  readonly notifications: MiniForgeNotificationApi;
}

export interface MiniForgeEditorApi {
  registerCommand(command: MiniForgeCommandContribution): Disposable;
  registerMenuItem(item: MiniForgeMenuItemContribution): Disposable;
  registerPanel(panel: MiniForgePanelContribution): Disposable;
  registerImporter(importer: MiniForgeImporterContribution): Disposable;
  getSelection(): Promise<MfEntityId[]>;
  executeCommand(commandId: MfCommandId, payload?: unknown): Promise<void>;
}

export interface MiniForgeSceneApi {
  query(query: MiniForgeSceneQuery): Promise<MiniForgeEntitySummary[]>;
}

export interface MiniForgeAssetsApi {
  find(query: MiniForgeAssetQuery): Promise<MiniForgeAssetSummary[]>;
}

export interface MiniForgeNotificationApi {
  show(notification: MiniForgeNotification): void;
}

export interface MiniForgeCommandContribution {
  id: MfCommandId;
  label: string;
  category?: string;
  shortcut?: string;
  run(payload?: unknown): void | Promise<void>;
}

export interface MiniForgeMenuItemContribution {
  menu: "File" | "Edit" | "View" | "Assets" | "Tools" | "Help";
  command: MfCommandId;
  group?: string;
  order?: number;
}

export interface MiniForgePanelContribution {
  id: string;
  title: string;
  preferredDockArea?: "left" | "right" | "bottom" | "top" | "floating";
  qmlEntry?: string;
}

export interface MiniForgeImporterContribution {
  id: string;
  label: string;
  extensions: string[];
  import(path: string): Promise<void>;
}

export interface MiniForgeSceneQuery {
  nameContains?: string;
  tag?: string;
  layer?: string;
  selectedOnly?: boolean;
}

export interface MiniForgeEntitySummary {
  id: MfEntityId;
  name: string;
  entityType: string;
  tag: string;
  layer: string;
}

export interface MiniForgeAssetQuery {
  nameContains?: string;
  type?: string;
  label?: string;
}

export interface MiniForgeAssetSummary {
  guid: MfAssetId;
  name: string;
  relativePath: string;
  assetType: string;
}

export interface MiniForgeNotification {
  title: string;
  message?: string;
  severity?: "info" | "warning" | "error";
}

export interface Disposable {
  dispose(): void;
}
