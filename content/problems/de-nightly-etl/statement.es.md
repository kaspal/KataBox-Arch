# Exportación nocturna desde PostgreSQL

## El producto

Tienes una tienda online. Sus pedidos viven en una base de datos PostgreSQL, y la app de la tienda los lee y los actualiza todo el día y toda la noche. Cada noche, el equipo de datos copia los pedidos de esa base de datos a sus ficheros de análisis: un trabajo de exportación, que lanza un planificador, lee la tabla de pedidos página a página. La gente sigue comprando de noche, así que la exportación corre mientras la app está ocupada, y la app tiene que seguir siendo rápida mientras tanto.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2000 peticiones/s (peticiones de la app y páginas de la exportación juntas) |
| Latencia p95 | 300 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 1700 $ al mes |

El p95 cuenta todas las peticiones: para la app, el tiempo hasta su respuesta; para una página de la exportación, el tiempo desde que entra en la cola hasta que se ha leído de la base de datos.

## El tráfico

- **Abrir un pedido** (60 %, 1200 por segundo). La petición puede pasar por un balanceador de carga, llega a tus Servidores de aplicaciones y estos leen un pedido de la base de datos.
- **Actualizar un pedido** (10 %, 200 por segundo). El mismo camino, pero el Servidor de aplicaciones escribe el pedido, así que tiene que ir a la principal de la base de datos.
- **Exportar una página de pedidos** (30 %, 600 páginas por segundo). Una tarjeta Planificador publica un mensaje por página en una Cola. Unos Workers de exportación consumen los mensajes y leen cada página de la base de datos. Una página es grande, de unos 576 KB, así que a la base de datos le cuesta mucho más que leer un pedido: unas 10 veces más. Escribir los ficheros de análisis no forma parte de este problema.

Una exportación real dura horas cada noche. El simulador no puede simular horas, así que cada prueba simula entre un minuto y un minuto y medio de la exportación a su ritmo de trabajo.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una noche con la exportación en marcha**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **La exportación lleva el triple de datos** y **Una réplica cae durante la exportación**. Tu diseño tiene que superar las tres.
