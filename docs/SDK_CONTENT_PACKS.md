# SDK y Content Packs opcionales

MiniForge separa el código del motor de los toolchains y contenidos pesados.
GitHub contiene únicamente el motor, editor, pruebas y documentación. Los juegos
de usuario no forman parte del catálogo ni de ningún paquete.

## Perfiles

El catálogo versionado del motor define tres perfiles:

- **Lean Development**: compilador y SDK `wgpu`, aproximadamente 2.44 GiB
  instalados.
- **Creator 2D**: añade materiales, superficies, partículas, audio, fuentes y
  plantillas multigénero; aproximadamente 6.05 GiB instalados.
- **Studio Heavy**: suma símbolos nativos, ejemplos y SDKs de exportación;
  aproximadamente 9.03 GiB instalados.

Los tamaños son presupuestos verificables del perfil, no archivos de relleno.
`SdkPackCatalog::validate()` rechaza dependencias inexistentes, ciclos, IDs
duplicados, tamaños inválidos y perfiles que no cumplan su rango.

## Flujo de instalación

`SdkPackCatalog::install_plan()`:

1. resuelve dependencias en orden;
2. omite paquetes instalados con la misma versión y verificación válida;
3. marca versiones antiguas o no verificadas para reparación;
4. calcula descarga, crecimiento de disco y tamaño final;
5. comprueba el objetivo de tamaño del perfil.

Cada manifest exige un SHA-256 suministrado por el manifest de release. La ruta
del archivo es independiente de plataforma y versión:

```text
packs/0.9.3.4/<pack-id>-<platform>.tar.zst
```

La descarga e instalación atómica se conectará al canal de releases; los
archives pesados no deben almacenarse en el historial Git.

## API

- Rust: `SdkPackCatalog`, `SdkPackRegistry` y `SdkPackInstallPlan`.
- C ABI: `mf_editor_sdk_pack_catalog_json` y
  `mf_editor_sdk_pack_plan_json`.
- Qt/QML: `MfBridge::sdkPackCatalogJson()` y
  `MfBridge::sdkPackPlanJson()`.
- Editor: pestaña **SDK Packs** y acción **Project → SDK & Content Packs**.
