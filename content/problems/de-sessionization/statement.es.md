# Sesionización de clics

## El producto

Llevas la analítica de una gran tienda online. Cada clic en la web (ver una página, buscar, "añadir al carrito") llega a tu backend, y el equipo de producto quiere la sesión actual de cada usuario en una tabla que pueda leer: qué páginas, en qué orden, cuánto dura hasta ahora. Un recolector en Go recibe los clics y los escribe en un stream de Kafka. Un procesador de streams lee el stream, guarda cada sesión abierta en su estado, con el usuario como clave, y escribe cada sesión actualizada en una tabla NoSQL.

Los analistas leen esa tabla cada pocos segundos, así que una sesión debería mostrar un clic pocos segundos después de que ocurra. Piden una cosa más: cuando el procesador se cae y se reinicia, no debe escribir en la tabla una avalancha de actualizaciones de sesión repetidas. Aceptan unos pocos segundos (como mucho 6000 resultados repetidos por caída, 15 s de clics en el pico), pero no minutos.

## Lo que debes cumplir

| Objetivo | Meta |
| --- | --- |
| Tráfico pico | 400 clics/s |
| Tiempo p95 del clic a la tabla de sesiones | 10 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Clics perdidos | ninguno |
| Resultados repetidos (duplicados) por caída | 6000 o menos |
| Presupuesto | 2500 $ al mes |

## El tráfico

- **Registrar un clic** (100 %). El clic puede pasar por un balanceador de carga, llega a tus Servidores de aplicaciones, entra en un Stream, lo procesa un Procesador de streams y se escribe en un almacén de datos. Se mide hasta que su resultado llega al almacén de datos. Hay 1 000 000 de usuarios, y unos son mucho más activos que otros (una popularidad Zipf de 0,8).

Cómo modelar las sesiones:

- Una sesión real termina tras 30 minutos sin clics. El simulador corre un minuto o dos, no horas, y no tiene ventanas de sesión, así que el tiempo está a escala: modela una sesión como una ventana deslizante (**Ventana** `sliding`) con una **Duración de la ventana** de 60 s y un **Desplazamiento** de 1 s. Así el procesador actualiza la sesión de cada usuario cada segundo. Deja el **Retraso de la marca de agua** en 5 s.
- Cada sesión abierta guarda unos **128 KB** de estado (su recorrido de clics, marcas de tiempo y atributos): pon **Estado por clave** en 128 KB.
- Un hueco del Procesador de streams procesa 20 000 registros simples por segundo (el valor por defecto de **Registros por segundo por hueco**). Una ventana deslizante cuesta ventana ÷ desplazamiento veces eso por clic: aquí, 60 veces.
- Cuando el estado del procesador supera su **Memoria** por sus **Instancias**, se vuelca a disco, y el simulador hace cada registro 4 veces más lento.

## Las pruebas

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico de arriba, en el pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El procesador de streams se cae** y **Cuatro veces más usuarios**. Tu diseño tiene que pasar las tres.
