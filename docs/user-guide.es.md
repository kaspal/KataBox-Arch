# Guía de usuario de KataBox Arch

KataBox Arch es una aplicación de escritorio para practicar diseño de sistemas. Colocas componentes de productos reales (PostgreSQL, Redis, Kafka, NGINX y muchos más) en un lienzo libre, los conectas y ejecutas una simulación de tráfico que anima cada petición. Cuando trabajas en un problema, la app califica tu diseño frente a los objetivos del problema.

Todo funciona en tu ordenador, sin conexión. Esta guía usa los nombres de botones y pestañas de la app en español; la versión en inglés es `user-guide.md`.

## Primeros pasos

1. **Abre la app.** Llegas a **Inicio**. La primera vez, Inicio te ofrece un recorrido: **Empezar el recorrido** reproduce «Hola, tráfico», un ejemplo breve que muestra el lienzo, una ejecución en vivo y las métricas en un minuto y medio, más o menos. **Ahora no** oculta la oferta; el recorrido sigue disponible en **Ejemplos**.
2. **Mira un ejemplo.** Abre **Ejemplos** en la barra de la izquierda y elige uno, por ejemplo «Acortador de URLs a escala». Avanza con el botón **Siguiente**. Cada paso carga un diseño, le aplica tráfico y muestra qué ha cambiado. En el último paso, **Pruébalo tú** copia el diseño al Sandbox para que puedas modificarlo.
3. **Resuelve tu primer problema.** Abre **Problemas** y elige **#1 Acortador de URLs** (Fácil). El lienzo se abre con la pestaña **Enunciado** a la derecha. Lee los objetivos y monta un primer diseño: arrastra un **Cliente**, un **Balanceador de carga**, un **Servidor de aplicaciones** y una **Base de datos SQL** desde la **Biblioteca** al lienzo y conéctalos en ese orden.
4. **Ejecútalo.** Elige una prueba en el selector de pruebas y pulsa **Ejecutar**. Observa cómo fluyen las peticiones y luego abre la pestaña **Métricas**.
5. **Envíalo.** Pulsa **Enviar**. Los **Resultados** muestran un veredicto para cada prueba, incluidas las ocultas, y una puntuación sobre 100. Si una prueba falla, **Repetir desde el fallo** te lleva al momento en que se torció. Corrige el diseño (las pistas ayudan) y vuelve a enviarlo.

## Cómo moverte por la app

La barra de la izquierda tiene las páginas principales: **Inicio**, **Problemas**, **Sandbox**, **Guía**, **Ejemplos**, **Progreso** y, abajo, **Ajustes**.

Algunas páginas no están en la barra. Llegas a ellas desde otras páginas o desde la paleta de comandos:

- **Patrones** y **Glosario**: enlaces en la parte superior de la Guía.
- **¿Cuál necesito?** (guías de decisión): en la página de la Guía.
- **Repaso**, **Itinerarios**, **Chuleta de números**, **Ejercicios de estimación** y **Encuentra el fallo**: en la paleta de comandos.

**La paleta de comandos.** Pulsa **Ctrl+K** en cualquier sitio. Escribe parte de un nombre para ir a una página, crear, abrir o guardar un diseño, abrir un archivo reciente, abrir una entrada de la Guía o un patrón, añadir un componente al lienzo, o cambiar el tema o el idioma. La búsqueda ignora tildes y signos de puntuación.

## Inicio

Inicio es tu mesa de trabajo. De arriba abajo:

- **Nuevo diseño** y **Abrir…** empiezan un diseño en blanco o abren un archivo `.kbarch`.
- **Práctica** muestra hasta tres tarjetas:
  - **Para repasar**: cuántas preguntas de cuestionario falladas tocan hoy (ver Repaso).
  - **Continuar**: el último problema en el que trabajaste.
  - **Siguiente recomendado**: si sigues un itinerario, su siguiente paso pendiente, con tu avance («2 de 9 hechos»). Si no, el siguiente problema que aún no has resuelto.
- Tu racha («Racha de 3 días») aparece debajo de las tarjetas.
- **Mira cómo funciona**: una galería de ejemplos, con **Todos los ejemplos** para ver la lista completa.
- **Diseños recientes**: los archivos que has abierto o guardado. **Abrir** vuelve a abrir uno; **Olvidar** lo quita de la lista (el archivo no se toca). Un archivo que se ha movido o borrado muestra «Archivo no encontrado».

## Problemas

**Problemas** lista 30 problemas incluidos, en dos series: del #1 al #15 (serie 1, de Acortador de URLs a Juez en línea) y del #16 al #30 (serie 2, de Miniaturas de imágenes a Bolsa con casamiento de órdenes). Cada fila muestra el número, el título, lo que enseña, la dificultad, tu estado (**Resuelto**, **Intentado** o **Sin empezar**), tu mejor puntuación y cuántos ejercicios de estimación has hecho para ese problema.

Para acotar la lista:

- **Buscar problemas** busca en los títulos, en lo que enseñan y en sus temas, en español y en inglés.
- Los botones de dificultad: **Todos**, **Fácil**, **Media**, **Difícil**.
- Las etiquetas de tema (por ejemplo `#caching`): pulsa una para filtrar y vuelve a pulsarla para quitar el filtro.
- **Origen** (**Todos**, **Incluidos**, **Paquetes**) aparece cuando has instalado un paquete de problemas. Los problemas de un paquete llevan una marca: «Del paquete <nombre>».

En la parte superior de la página:

- **Continuar: <título>** vuelve a abrir el último problema en el que trabajaste.
- **Problema al azar** abre un problema que aún no has resuelto.
- **Importar paquete…** añade un paquete de problemas (ver Paquetes de problemas).

## Resolver un problema

Un problema se abre con la misma disposición que el Sandbox, con una pestaña **Enunciado** en el panel lateral. Tu diseño se guarda solo como borrador mientras trabajas, así que puedes irte y volver. No necesitas guardar un archivo salvo que quieras uno.

### El enunciado

La pestaña **Enunciado** reúne todo lo que te dice el problema:

- **El planteamiento**: qué construyes y qué importa.
- **Objetivos**: **Tráfico pico**, **Latencia p95**, **Latencia p99** (solo en los problemas que la fijan), **Disponibilidad** y **Presupuesto** al mes. Al lado está el enlace a la **Chuleta de números**.
- **Tráfico**: los tipos de petición y su parte de la carga.
- **Proveedores externos**, en los problemas que llaman a uno (por ejemplo, un proveedor de pagos): su latencia mediana y p99 y su límite de peticiones.
- **Pruebas**: cada prueba con su nombre y su duración. Algunas están marcadas como **oculta**: ves su nombre, pero no su tráfico ni sus fallos, hasta que resuelves el problema.
- **Estima primero**: los ejercicios de estimación del problema, plegados hasta que los abres, con un recuento como «1 de 2 estimadas». Nunca cuentan para tu puntuación.
- **Relacionado en KataBox**: temas de Learn y problemas de KataBox que practican los algoritmos que hay detrás. KataBox es una app aparte, así que son títulos para buscar allí, no enlaces.
- **Pistas**: tres por problema, que se desbloquean de una en una con **Mostrar la pista 1**, **Mostrar la pista 2** y **Mostrar la pista 3**. Cada una revela un poco más.
- **Solución comentada**: la solución explicada. Aparece cuando has resuelto el problema.

### Ejecutar y Enviar

**Ejecutar** reproduce una prueba en vivo sobre el lienzo. Elígela en el selector de pruebas que hay junto a Ejecutar. El selector muestra las pruebas visibles; las ocultas solo aparecen ahí cuando las repites desde los Resultados.

**Enviar** juzga el diseño en todas las pruebas, también las ocultas. Mientras trabaja, el botón dice **Juzgando…**. Después se abren los **Resultados**:

- Un resumen, como «3 de 4 pruebas superadas; el diseño cuesta 1.840 US$ al mes».
- **Puntuación**, dividida en **Corrección**, **Coste** (el tuyo frente al de la referencia) y **Margen de latencia**, más tu mejor puntuación en este problema.
- Cada prueba con su veredicto, su p95 (y su p99 si el problema la fija), la tasa de errores y la disponibilidad y, si ha fallado, el cuello de botella: por ejemplo «PostgreSQL saturado desde los 34 s.» o «Redis estuvo caído desde los 30 s.».
- **Repetir desde el fallo** inicia la ejecución en vivo de esa prueba unos dos segundos antes de que apareciera el cuello de botella, con la tarjeta que falla seleccionada. También funciona con las pruebas ocultas.
- **¿Te has atascado?** muestra tu última pista o desbloquea la siguiente.
- Cuando el problema está resuelto, aquí también está **Comparar con la referencia**.

La primera vez que superas todas las pruebas, los Resultados dicen «¡Resuelto! La solución comentada ya está en la pestaña Enunciado.».

### Después de resolverlo

El Enunciado gana una sección **Comparar**:

- **Comparar con la referencia** abre tu diseño junto al diseño de referencia. **Lado a lado** los muestra en dos paneles que hacen zoom y se desplazan a la vez; **Superpuestos** dibuja la referencia tenue detrás del tuyo.
- Un gráfico sitúa tus envíos aceptados según su coste al mes y su latencia p95, junto a la referencia. Abajo a la izquierda es mejor: más barato y más rápido.

## Cómo se califica

El juez es determinista: el mismo diseño obtiene siempre el mismo veredicto y la misma puntuación, en cualquier equipo. Cada prueba usa su propia semilla fija, así que ninguna ejecución tiene buena o mala suerte.

Antes de ejecutar ninguna prueba, se revisa el diseño: las reglas de conexión, que cada tipo de petición tenga un camino y que estén los componentes que exige el problema. Si falta algo, el resultado completo es **Diseño no válido**, con los motivos, y la puntuación es 0.

Después, cada prueba recibe un veredicto. Gana el primero que se cumpla:

1. **Diseño no válido**: el diseño incumple una regla.
2. **Solicitudes perdidas**: fallaron demasiadas peticiones o se perdieron mensajes.
3. **SLO incumplido**: latencia p95 por encima del objetivo, disponibilidad por debajo del objetivo o demasiadas lecturas obsoletas.
4. **Sobre presupuesto**: el coste mensual (medido en la primera prueba) supera el presupuesto.
5. **Aceptado**: todo se ha sostenido.

El problema queda resuelto cuando todas las pruebas son **Aceptado**.

La puntuación es sobre 100:

| Parte | Puntos | Cómo se gana |
| --- | --- | --- |
| Corrección | 70 | En proporción a las pruebas aceptadas: 3 de 4 dan 52. |
| Coste | 20 | Todos los puntos si no cuestas más que la referencia; menos cuanto más la superes. Se multiplica por la parte de pruebas aceptadas. |
| Margen de latencia | 10 | Cuánto quedan tus pruebas aceptadas por debajo del objetivo de p95. Con la mitad del objetivo o menos, los 10. |

Un diseño que no supera ninguna prueba no gana puntos de coste ni de latencia, así que un diseño barato y roto no puntúa.

Las **pruebas ocultas** reproducen lo que sale mal en producción: un pico, un nodo que se cae, una clave muy solicitada. Los Resultados muestran su nombre, su veredicto y sus métricas, nunca su tráfico ni su calendario de fallos.

## El Sandbox

El **Sandbox** es un lienzo libre, sin problema asociado. Tiene tres zonas: la **Biblioteca** a la izquierda, el lienzo en el centro y el panel lateral a la derecha, con las pestañas **Inspeccionar** y **Métricas** (y **Enunciado** cuando resuelves un problema).

### Añadir componentes

La **Biblioteca** agrupa los componentes en **Clientes y borde**, **Cómputo**, **Datos**, **Asíncrono** y **Operaciones**, seguidos de **Zonas y notas**. **Buscar componentes** filtra la lista.

- Arrastra un elemento al lienzo, o selecciónalo con el teclado y pulsa **Intro** para añadirlo en el centro de la vista.
- Si un componente existe en varios productos, un selector pregunta «¿Qué Caché?» (por ejemplo **Redis** o **Memcached**). Puedes cambiar el producto más tarde en el inspector.
- Cada elemento tiene un botón **Acerca de** con una descripción breve, sus variantes, a qué prestar atención y **Abrir en la guía**.

### Conexiones

Arrastra de una tarjeta a otra para conectarlas. La app comprueba las reglas de conexión mientras arrastras y explica por qué rechaza una conexión, por ejemplo:

- «Un cliente no puede llegar directamente a un almacén de datos. Pon un servicio en medio.»
- «Los mensajes salen de una cola o un stream solo a través de un servicio consumidor.»
- «Los almacenes de datos responden consultas; no inician llamadas.»

Cuando dos tarjetas se pueden unir de más de una forma, el selector **Conectar con** pide el protocolo: **HTTP**, **gRPC**, **WebSocket**, **Consulta**, **Publicar**, **Consumir** o **Replicar**.

Con el teclado: selecciona una tarjeta y pulsa **C**. Elige el destino con las flechas o con **Tab** y pulsa **Intro**. **Esc** cancela.

### El inspector

Selecciona una tarjeta, una conexión o una zona para editarla en la pestaña **Inspeccionar**. En una tarjeta puedes cambiar la **Etiqueta**, la **Variante** (el producto) y sus **Propiedades**: instancias, tiempo de servicio, réplicas de lectura, TTL, memoria, etc., según el tipo.

Algunas propiedades llevan una marca:

- **citado**: un valor por defecto calibrado, tomado de una fuente citada.
- **aproximado**: un valor por defecto calibrado, una cifra prudente cuya fuente aún se está confirmando.

Los valores sin marca son decisiones de diseño que tomas tú. Una propiedad que has cambiado muestra un botón para volver a su valor por defecto. **Saber más** abre la entrada de la Guía del componente en su sección «En el simulador».

Sin nada seleccionado, el inspector muestra el **Nombre del diseño** y el recuento de componentes, conexiones, zonas y notas.

### Zonas y notas

- Los marcos **Región** y **Zona de disponibilidad** agrupan tarjetas. Las tarjetas dentro de una zona se mueven con ella, y una zona de disponibilidad puede ir dentro de una región.
- **Autoescalado** (en el grupo Cómputo) añade un marco **Grupo de autoescalado**. Las tarjetas de cómputo que hay dentro escalan entre las **Instancias mínimas** y las **Instancias máximas**, y añaden instancias cuando la utilización se mantiene por encima de la **Utilización objetivo**. La **Política de escalado** se ajusta en el inspector.
- **Nota adhesiva**: haz doble clic para escribir.

### Edición

- **Deshacer** (Ctrl+Z) y **Rehacer** (Ctrl+Y o Ctrl+Mayús+Z) están en la barra de herramientas.
- **Supr** o **Retroceso** borran la selección.
- **Ajustar a la cuadrícula** coloca las tarjetas en una cuadrícula de 8 px. Mantén **Alt** al arrastrar para colocar una tarjeta libremente. También puedes cambiar el ajuste en Ajustes.
- Los botones de zoom y **Ajustar el diseño** están en una esquina del lienzo, junto al minimapa.

## Ejecuciones en vivo

Una ejecución en vivo anima cada petición a través de tu diseño, en tiempo simulado.

**Iniciar una ejecución.** En el Sandbox, elige el **Tráfico** (**Tráfico normal**, **Pico ×4** o **Matar un nodo a los 20 s**) y pulsa **Ejecutar**. En un problema, elige una prueba. Cuando termina, **Ejecutar de nuevo** la vuelve a lanzar.

**La línea de tiempo**, bajo el lienzo, tiene **Pausar** y **Continuar**, el reloj simulado («12.0 s de 60 s»), una barra para desplazarte y la **Velocidad** (de 0,25× a 8×). Los rombos de la barra marcan fallos, con una descripción como «Una tarjeta muere a los 20 s». Al arrastrar la barra saltas a ese momento y la ejecución se pausa ahí; como la ejecución es determinista, ves exactamente lo que pasó.

**Inyectar fallos.** Durante una ejecución, selecciona una tarjeta y abre **Inyectar**:

- **Matarla**
- **Ralentizarla (+200 ms durante 10 s)**
- **Vaciarla (vaciar la caché)**, para una tarjeta de Caché o CDN
- **Triplicar el tráfico durante 10 s** (no necesita ninguna tarjeta seleccionada)

**Detener** termina la ejecución. Si editas el diseño durante una ejecución, la app empieza una nueva ejecución del diseño editado con la misma semilla y el mismo tráfico.

**La pestaña Métricas** muestra la **Latencia p95**, el **Rendimiento**, la **Tasa de errores** y el **Coste** al mes del último segundo simulado, además de una línea con la evolución de la p95 y los **Componentes más ocupados**. Algunos diseños muestran también la **Latencia p99**, la **Disponibilidad** o las **Lecturas obsoletas**. Con mucho tráfico, el simulador sigue solo una parte de las peticiones y escala los recuentos; la pestaña lo indica cuando ocurre.

**Buscar el punto de ruptura** sube el tráfico en ejecuciones de 20 segundos hasta que el diseño se rompe, es decir, p95 por encima de 200 ms o más de un 1 % de errores. Indica cuánto aguanta («Aguanta unas 9600 solicitudes/s (2.4× el tráfico de los clientes)») y qué tarjeta cede primero. Si cambias el diseño después, vuelve a buscarlo.

## Archivos y exportación

Los diseños se guardan como archivos `.kbarch`. Las acciones de archivo están en la barra de herramientas, en la paleta de comandos y en el teclado:

- **Nuevo diseño** (Ctrl+N), **Abrir diseño** (Ctrl+O), **Guardar** (Ctrl+S), **Guardar como…** (Ctrl+Mayús+S).
- El nombre del archivo aparece junto al nombre del diseño; un punto indica cambios sin guardar.
- Si cierras la ventana o abres otro diseño con cambios sin guardar, la app pregunta **¿Guardar los cambios?** con **Guardar**, **No guardar** y **Cancelar**.

**Importar docker-compose…** (barra de herramientas o paleta de comandos) convierte un `docker-compose.yml` en un diseño nuevo sin guardar:

- Los servicios se convierten en tarjetas según su imagen: PostgreSQL, MySQL/MariaDB, Redis/Valkey, Memcached, NGINX, HAProxy, Kafka/Redpanda, RabbitMQ, Elasticsearch, OpenSearch, MongoDB y Cassandra/Scylla. Cualquier otra cosa se convierte en un App Server, y el resumen dice cuáles se han supuesto.
- `deploy.replicas` fija las instancias de la tarjeta (como mucho 64; un número mayor se recorta y se indica).
- `depends_on` y `links` se convierten en conexiones, con un protocolo que permitan las reglas de conexión. Una dependencia que las reglas no permiten, o una hacia un servicio que no está en el archivo, se lista en vez de dibujarse.
- Se rechazan los archivos de más de 1 MB, con más de 200 servicios o con YAML hecho para crecer sin límite (demasiados alias o niveles).

El menú **Exportar** de la barra de herramientas convierte el diseño en:

- **Copiar como Mermaid** y **Guardar como Mermaid…**: un diagrama Mermaid que puedes pegar en documentos Markdown.
- **Guardar como imagen SVG…** y **Guardar como imagen PNG…**: una imagen del diseño.
- **Guardar animación (WebM)…**: un vídeo de la ejecución, con las peticiones en movimiento, la carga de cada tarjeta y
  una línea con el tiempo, el p95, el rendimiento y los errores. Elige un inicio y una duración (hasta 60 s); se graba en
  tiempo real, así que mantén la ventana abierta. Muestra el tráfico del Sandbox, o la prueba que esté reproduciendo
  Ejecutar.
- En **Resultados**, una prueba fallida tiene **Exportar repetición** junto a Repetir: el mismo vídeo de la parte que
  muestra la repetición, también en pruebas ocultas (nunca se muestra nada antes de donde empieza la repetición).

## Aprender: la Guía, los patrones y el glosario

### La Guía de componentes

La **Guía** tiene una entrada para cada tipo de componente, con un recuento de cuántas has leído. **Empieza aquí** reúne las seis piezas que casi todo diseño usa. Busca con palabras como «caché», «fan-out» o «replicación».

Una entrada cubre **Qué hace**, **Úsalo cuando**, **Evítalo cuando**, **En el simulador** (las cifras que usa el simulador para cada producto), **Errores comunes**, **Compromisos**, **Los productos comparados**, **Practícalo** (problemas que lo usan), **Funciona con**, un **Consejo para la entrevista** y **Fuentes**. Muchas entradas tienen una **Tarjeta de concepto**: un ejemplo animado breve, que se inicia con **Ver la tarjeta de concepto**. **Marcar como leído** registra la entrada como leída.

Cada entrada termina con una **Comprobación rápida** de tres preguntas. Una pregunta fallada vuelve para repasarla (ver Repaso).

También llegas a la Guía desde el lienzo: **Abrir en la guía** en el panel Acerca de de la Biblioteca y **Saber más** en el inspector.

**Imprimir** (en la Guía, en cada entrada y patrón, y en cada ruta de aprendizaje) guarda una copia en papel: **Guardar como PDF…** (Windows) o **Guardar página lista para imprimir…** (en todas partes; ábrela en un navegador para imprimirla). La Guía entera se imprime con una entrada por página y luego el glosario; un patrón incluye la imagen de su plantilla; una ruta lista lo que cubre, con sus entradas de la Guía y sus patrones. Si no se puede crear un PDF, la app ofrece el HTML en su lugar.

### Patrones

Los **Patrones** son formas probadas para problemas habituales: Contrapresión, Cache-aside, Circuit breaker, CQRS, Fan-out, Outbox transaccional, Limitación de tasa, Réplicas de lectura, Saga y Sharding. Cada uno explica **El problema** y **Cómo funciona**, muestra **La plantilla** y termina con una comprobación rápida. **Insertar en el Sandbox** añade las tarjetas de la plantilla al diseño que estás editando, en el centro de la vista; un solo deshacer las quita.

### Guías de decisión

En **¿Cuál necesito?**, en la página de la Guía, hay cuatro recorridos breves de preguntas: ¿Caché o CDN?, ¿Balanceador de carga o API Gateway?, ¿Cola, stream o pub/sub? y ¿SQL o NoSQL? Respondes una pregunta cada vez y la guía termina en la entrada de la Guía que te conviene. **Empezar de nuevo** la reinicia.

### Glosario

El **Glosario** define los términos que usa la app, como p95, y se puede buscar. En los enunciados, las pistas, las soluciones comentadas, las entradas de la Guía y los patrones, el primer uso de cada término del glosario aparece subrayado con puntos: pasa el ratón por encima (o llega a él con el teclado) para ver la definición, o haz clic para abrir el Glosario en ese término.

## Practicar: ejemplos, ejercicios e itinerarios

### Ejemplos

Los **Ejemplos** son historias guiadas de unos pocos pasos: Hola, tráfico (el recorrido de bienvenida), Acortador de URLs a escala, Feed de noticias: fan-out, Chat con WebSockets y Venta relámpago. Cada paso carga un diseño, lo ejecuta y muestra las cifras que han cambiado frente a un objetivo. Usa **Anterior**, **Siguiente** y los puntos de los pasos para moverte, y **Repetir el paso** para volver a verlo. **Pruébalo tú** copia el diseño del paso al Sandbox como «<título> (mi copia)».

### Chuleta de números

Velocidades y tamaños aproximados para estimar: memoria, almacenamiento y bases de datos, red, servidores, tamaños, tiempo y tráfico. Son órdenes de magnitud, no benchmarks. Las cifras marcadas como del propio simulador son las que usa el simulador, así que tus estimaciones y sus ejecuciones coinciden.

### Ejercicios de estimación

Práctica de cálculo rápido para cada problema. Elige un **Problema** (o pulsa **Ejercicio al azar**), calcula la cifra, escríbela en **Tu respuesta** y pulsa **Comprobar**. Cualquier valor dentro de la banda del ejercicio vale; si no, la app te dice si te has pasado, si te has quedado corto o si estás a más de diez veces. Una solución resuelta muestra una forma de llegar. Los mismos ejercicios aparecen en la sección **Estima primero** de cada problema.

### Encuentra el fallo

Diez diseños que parecen correctos y aguantan un día normal, pero cada uno tiene un fallo que una prueba deja al descubierto. Abre uno, ejecútalo, encuentra el fallo, corrígelo y envíalo hasta que pase todas las pruebas. Una vez corregido, la explicación y la solución de referencia aparecen en el enunciado.

### Itinerarios

Los **Itinerarios** son cinco recorridos ordenados: Fundamentos, Sistemas asíncronos, Datos a escala, Fiabilidad y Tiempo real. Cada uno mezcla entradas de la Guía, tarjetas de concepto, patrones, problemas, ejercicios y cuestionarios. El progreso sale de lo que ya has hecho en cualquier parte de la app. Pulsa **Seguir este itinerario** y la tarjeta **Siguiente recomendado** de Inicio mostrará su siguiente paso.

## Repaso

Cada entrada de la Guía y cada patrón terminan con tres preguntas de cuestionario. Una pregunta que aciertas a la primera no vuelve nunca. Una pregunta que fallas entra en una pequeña cola de repaso:

- Toca repasarla al día siguiente.
- Cada vez que la aciertas en un repaso, vuelve más tarde: al cabo de 1, luego 3, luego 7 y luego 21 días. Si la aciertas de nuevo tras los 21 días, sale de la cola.
- Si la fallas en un repaso, vuelve a empezar y toca al día siguiente.

Cuando hay preguntas pendientes, Inicio muestra la tarjeta **Para repasar**. Ábrela, o abre **Repaso** desde la paleta de comandos, y responde las preguntas una a una con **Siguiente pregunta** y **Terminar**. Primero salen las que más has fallado. Un día sin nada pendiente, la página dice «Nada que repasar hoy».

## Progreso

La página **Progreso** todavía es provisional («Todavía no hay nada que mostrar»). Por ahora, tu progreso aparece aquí:

- **Problemas**: el estado y la mejor puntuación de cada problema.
- **Inicio**: tu racha y las recomendaciones.
- **Guía** y **Patrones**: cuántas páginas has leído.
- **Itinerarios**, **Ejercicios de estimación** y **Encuentra el fallo**: cada uno con su propio recuento.

## Ajustes

**Apariencia**

- **Tema**: Sistema, Claro u Oscuro.
- **Contraste**: Normal o Alto. El alto contraste usa texto, bordes y anillos de foco más marcados.
- **Movimiento**: Seguir al sistema o Reducido. Reducido recorta las animaciones en toda la app.

**Idioma**: **Idioma de la app**, English o Español. Todo el contenido (problemas, pistas, Guía, ejemplos) está en los dos idiomas.

**Lienzo**: **Ajustar a la cuadrícula**, Sí o No.

**Paquetes de problemas**: los paquetes instalados con **Desinstalar**, **Importar paquete…** y la casilla **Recomendar en Inicio problemas de los paquetes**.

**Acerca de**: la versión de la app y la declaración de privacidad.

## Paquetes de problemas

Un paquete de problemas es un conjunto de problemas extra que alguien ha compartido como archivo `.kbpack`: la serie de un profesor para su clase, la práctica de entrevistas de un equipo, una colección de la comunidad. Un paquete contiene solo texto y diseños, nunca código.

**Importar.** Pulsa **Importar paquete…** en la página Problemas o en Ajustes y elige el archivo. La app comprueba el paquete antes de instalar nada («Comprobando el paquete…»); un paquete grande puede tardar unos segundos. Las comprobaciones:

- Cada archivo figura en el paquete con su suma SHA-256, y todas deben coincidir. Se rechazan los archivos desconocidos, los que faltan o los que están fuera de sitio.
- Los nombres, los tamaños, los tipos de archivo y el número de entradas están limitados, para que un paquete no pueda escribir fuera de su carpeta ni llenar tu disco.
- Se rechaza el texto con scripts, marcos incrustados o marcado parecido. El texto de un paquete nunca carga imágenes ni sigue enlaces.
- Cada problema se carga y se valida como uno incluido, y se juzga: su diseño de referencia debe superar todas las pruebas y su diseño ingenuo debe fallar una prueba oculta.

Si el paquete las supera, la vista previa **¿Instalar este paquete?** muestra su nombre, autoría, versión, licencia, número de problemas y SHA-256, y qué va a pasar (un paquete nuevo, una actualización o una reinstalación). También dice «Paquete sin firmar, de <autor> (según el propio paquete). Instala solo paquetes de personas en las que confíes.». Los paquetes no van firmados; el nombre del autor es el que pone el propio paquete. Si quien lo compartió publicó el SHA-256, compáralo con el de la vista previa. No se instala nada hasta que pulsas **Instalar**.

**Versiones.** Un paquete solo puede sustituirse por una versión más nueva. Se rechazan las versiones más antiguas y la misma versión con otro contenido.

**Desinstalar.** En Ajustes, **Desinstalar** saca los problemas del paquete del catálogo. Tus envíos se quedan en tu historial.

Los problemas de un paquete llevan una marca y se pueden filtrar con **Origen**. Inicio solo los recomienda si activas **Recomendar en Inicio problemas de los paquetes**.

## Privacidad

KataBox Arch funciona sin conexión. Nunca envía telemetría; Ajustes › Acerca de también lo dice. Tus diseños, tu progreso y tus ajustes se quedan en tu ordenador.

La barra de herramientas muestra un botón **Revisión con IA** que indica que llega en M4. Cuando llegue, explicará tus resultados y cuestionará tu diseño usando un proveedor de IA que tú elijas y configures, y solo cuando tú inicies una revisión. Nunca cambiará una puntuación: la calificación sigue en manos del juez determinista.

## Atajos de teclado

| Atajo | Acción |
| --- | --- |
| Ctrl+K | Abrir o cerrar la paleta de comandos |
| Ctrl+N | Nuevo diseño |
| Ctrl+O | Abrir un diseño |
| Ctrl+S | Guardar |
| Ctrl+Mayús+S | Guardar como |
| Ctrl+Z | Deshacer (Sandbox y problemas) |
| Ctrl+Y o Ctrl+Mayús+Z | Rehacer (Sandbox y problemas) |
| Supr o Retroceso | Borrar la selección del lienzo |
| C | Empezar a conectar desde la tarjeta seleccionada |
| Flechas o Tab, luego Intro | Elegir el destino de una conexión y conectar |
| Esc | Cancelar una conexión o cerrar un selector |
| Alt (al arrastrar) | Colocar una tarjeta libremente, sin ajuste a la cuadrícula |
| Intro (en un elemento de la Biblioteca) | Añadirlo al lienzo |

Al pulsar Tab al principio de una página aparece el enlace **Saltar al contenido**.

## Solución de problemas

**«Este diseño se guardó con una versión más nueva de KataBox Arch.»** El archivo viene de una versión posterior de la app. Actualiza KataBox Arch para abrirlo. Los archivos más antiguos se abren con normalidad; la app los actualiza al leerlos.

**«No es un archivo de diseño válido de KataBox Arch.»** o **«El archivo es demasiado grande para abrirlo.»** El archivo está dañado, no es un diseño `.kbarch` o supera el límite de 8 MB.

**«El diseño incumple una regla (conexiones, ids o valores), así que no se guardó.»** Busca una conexión que las reglas no permitan o una propiedad fuera de su rango; el inspector muestra el rango permitido cuando escribes un valor.

**«Ninguna solicitud puede atravesar este diseño.»** Conecta un Cliente a un servicio y comprueba que cada tipo de petición del enunciado llega a los componentes que necesita.

**No se ha instalado un paquete.** «Este paquete no ha superado las comprobaciones de seguridad y contenido» significa que ha fallado alguna comprobación; el diálogo explica por qué en **Motivos**. «Este paquete no puede sustituir al instalado» significa que es más antiguo que el que tienes, o la misma versión con otro contenido. Pide a quien lo creó un paquete corregido o más nuevo.

**Un diseño reciente dice «Archivo no encontrado».** Se ha movido o borrado. Usa **Olvidar** para quitarlo de la lista y **Abrir…** para buscarlo de nuevo.

**Dónde se guardan tus datos.** Tu progreso, tus ajustes, los archivos recientes y los paquetes instalados están en la carpeta de datos de la app. En Windows es `%APPDATA%\dev.katabox.arch`. Tus diseños `.kbarch` están donde los guardaste. Las compilaciones de desarrollo muestran la marca **DATOS DE DESARROLLO** y guardan sus datos en una carpeta aparte terminada en `-dev`, así que nunca tocan tu perfil real.
