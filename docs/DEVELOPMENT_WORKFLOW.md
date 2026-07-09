# Flujo de desarrollo de MiniForge

Este flujo convierte las comprobaciones del motor en comandos repetibles. La regla es simple: feedback corto mientras se trabaja y una verificación completa antes de integrar o empaquetar.

## Inicio rápido

```bash
cargo dev -- doctor
cargo dev -- quick
cargo dev -- project projects/DefaultProject
cargo dev -- verify
```

- `doctor`: comprueba workspace, Rust, Cargo, rustfmt y Clippy. También detecta `cargo-nextest` y `cargo-deny` como herramientas opcionales.
- `quick`: formato, compilación de todos los targets y tests de biblioteca. Es el bucle recomendado durante una edición.
- `project [ruta]`: ejecuta `ProjectValidator` y `SystemReadinessReport`, mostrando errores, warnings, readiness y las siguientes acciones.
- `verify`: reproduce las puertas de calidad: formato, check, Clippy sin warnings, todos los tests y documentación.
- `ship`: ejecuta `verify` y compila `miniforge_runtime` con el perfil `ship`.

Los comandos aceptan `--json`; los workflows aceptan `--keep-going` para recoger todos los fallos en una sola pasada.

## Comandos directos

```bash
cargo dev-check
cargo dev-lint
cargo dev-test
cargo ship-runtime
```

Todos los pasos que resuelven dependencias usan `--locked`. El archivo `rust-toolchain.toml` garantiza que rustfmt y Clippy estén disponibles en la toolchain estable. CI añade documentación con `RUSTDOCFLAGS=-D warnings`, cancela runs obsoletos de una misma rama y limita el tiempo total.

## Perfiles de compilación

`release` sigue siendo el perfil práctico para iterar sobre builds optimizados. `ship` está reservado para distribución: Thin LTO, una unidad de codegen, símbolos eliminados y panic abort. Cargo documenta que Thin LTO ofrece buena parte del beneficio de LTO completo con menor coste de enlace; un perfil separado evita pagar ese coste durante cada iteración.

## Tecnologías evaluadas

### Adoptadas ahora

- Cargo aliases y un runner Rust dentro del propio motor para que local y CI hablen el mismo idioma.
- Toolchain declarada con rustfmt/Clippy.
- Dependabot semanal para Cargo y mensual para GitHub Actions, agrupando actualizaciones para evitar ruido.
- Perfil `ship` para runtimes compactos.

### Siguiente lote recomendado

1. **`cargo-nextest`** para paralelizar la suite cuando el tiempo de CI empiece a doler. Soporta perfiles, detección de flaky tests, retries explícitos y JUnit. No debe sustituir doctests; se mantiene `cargo test --doc` o `cargo doc` como puerta aparte.
2. **`cargo-deny`** para advisories, licencias, fuentes y duplicados. Antes de hacerlo bloqueante hay que acordar la política SPDX del proyecto y revisar excepciones reales, no copiar una allowlist genérica.
3. **`tracing`** como capa de telemetría estructurada. Los spans encajan especialmente bien en asset import, scheduler, scripting, render y packaging porque conservan causalidad y duración, cosa que el logger de texto actual no expresa.
4. **Tracy** como profiler de captura opcional para CPU/GPU, memoria y locks. Debe ir detrás de un feature flag (`profiling-tracy`) y complementar el profiler embebido, no reemplazarlo.
5. **KTX 2.0/Basis Universal** en el pipeline de texturas cuando se consolide el backend GPU. Reduce tamaño de distribución y memoria GPU mediante transcodificación al formato nativo de cada plataforma.
6. **`wgpu`** como backend futuro, no como reescritura inmediata. La abstracción `render::backend` permite desarrollar un backend experimental detrás de feature flag y conservar Macroquad hasta igualar 2D, editor y packaging.

## Criterios para introducir una tecnología

Una dependencia nueva entra al núcleo solo si tiene propietario, caso de uso medible, feature flag cuando sea costosa, fallback y una prueba o métrica que demuestre la mejora. Las migraciones de render o ECS deben convivir con la ruta estable hasta alcanzar paridad; no conviene convertir una investigación en bloqueo del editor.

## Fuentes consultadas

- [Cargo configuration y aliases](https://doc.rust-lang.org/cargo/reference/config.html)
- [Cargo profiles, Thin LTO y strip](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [GitHub Actions concurrency](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency)
- [Dependabot options](https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuration-options-for-the-dependabot.yml-file)
- [cargo-nextest en CI](https://nexte.st/docs/installation/pre-built-binaries/)
- [cargo-nextest retries y flaky tests](https://nexte.st/docs/features/retries/)
- [cargo-deny checks](https://embarkstudios.github.io/cargo-deny/checks/)
- [`tracing`: spans y eventos estructurados](https://docs.rs/tracing/latest/tracing/)
- [wgpu: backends y portabilidad](https://wgpu.rs/)
- [Tracy profiler](https://github.com/wolfpld/tracy)
- [KTX 2.0 y Basis Universal](https://www.khronos.org/ktx/)
