# KataBox Arch

KataBox Arch es una app de escritorio para Windows y Linux para practicar diseño de sistemas al estilo LeetCode. Colocas componentes reales (balanceadores de carga, cachés, colas, bases de datos) en un lienzo libre, los conectas y ejecutas una simulación de tráfico determinista que anima cada petición y califica el diseño según los SLO del problema. Funciona en local, sin cuenta y sin telemetría, y forma parte de la familia KataBox. El código tiene licencia MIT OR Apache-2.0.

> **Estado:** versión previa (0.1.0, camino de la beta 0.9.0). Todo lo de abajo funciona ya, salvo la revisión con IA, que llega en M4.

## Qué incluye

- **30 problemas** en dos series, del acortador de URLs a una bolsa con casamiento de órdenes: cada uno con un
  enunciado, objetivos, pruebas visibles y ocultas, tres pistas y una solución comentada. Envía un diseño y el juez lo
  puntúa por corrección, coste y margen de latencia; el mismo diseño obtiene siempre el mismo resultado.
- **Un lienzo libre** con 25 tipos de componente y sus variantes de productos reales, regiones, zonas de
  disponibilidad, grupos de autoescalado y notas; reglas de conexión que se explican solas; deshacer, ajuste a la
  cuadrícula y archivos `.kbarch`.
- **Ejecuciones en vivo:** cada petición animada, tarjetas que muestran su carga, fallos inyectados, una línea de tiempo
  que puedes recorrer, métricas en vivo y un buscador del punto de ruptura.
- **Aprendizaje:** la Guía de componentes (25 entradas, tarjetas de concepto, cuestionarios), 10 patrones, guías de
  decisión, un glosario, una chuleta de cifras, ejemplos guiados, ejercicios de estimación, diez ejercicios de
  «Encuentra el fallo», cinco rutas de aprendizaje y repaso espaciado de las preguntas falladas.
- **Compartir:** paquetes de problemas (`.kbpack`, comprobados antes de instalarse), exportación a Mermaid e imágenes
  SVG y PNG.
- **Inglés y español** en todo; alto contraste y movimiento reducido; funciona sin conexión.

Consulta [docs/user-guide.es.md](docs/user-guide.es.md) para un recorrido, [docs/pack-authoring.md](docs/pack-authoring.md)
para compartir problemas y [docs/known-issues.md](docs/known-issues.md).
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
