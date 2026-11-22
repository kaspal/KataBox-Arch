# KataBox Arch

KataBox Arch es una app de escritorio para Windows y Linux para practicar diseño de sistemas al estilo LeetCode. Colocas componentes reales (balanceadores de carga, cachés, colas, bases de datos) en un lienzo libre, los conectas y ejecutas una simulación de tráfico determinista que anima cada petición y califica el diseño según los SLO del problema. Funciona en local, sin cuenta y sin telemetría, y forma parte de la familia KataBox. El código tiene licencia MIT OR Apache-2.0.

> **Estado:** en desarrollo (M0: el editor del lienzo, abrir y guardar). La simulación, los problemas y la Guía de componentes llegan en hitos posteriores.

## Desarrollo

Necesitas Rust 1.99+, Node 22+ y pnpm 10, además de WebView2 (Windows) o WebKitGTK 4.1 (Linux).

```sh
pnpm install
pnpm dev        # ejecuta la app (las compilaciones de depuración guardan sus datos en carpetas -dev aparte)
cargo test      # crates
pnpm test       # interfaz
```

Consulta `CLAUDE.md` para todas las comprobaciones y las reglas del proyecto, y `docs/adr/` para las decisiones.

[Read in English](README.md)
