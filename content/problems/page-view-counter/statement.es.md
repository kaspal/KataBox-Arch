# Contador de visitas

## El producto

Tienes un servicio que cuenta visitas para sitios web. Cada vez que alguien abre una página, el sitio te envía un pequeño aviso y tú sumas uno al contador de esa página. Los dueños de los sitios abren un panel de estadísticas para ver cómo van sus páginas. Los avisos llegan sin parar, unas pocas páginas son mucho más populares que el resto y todas las visitas tienen que acabar en el contador: no se puede perder ninguna.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 3000 peticiones/s |
| Latencia p95 hasta completar | 3 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 3500 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Los sitios web llegan a una Collector API, un App Server, normalmente a través de un Load Balancer.

- **Contar una visita** (99 %). La Collector publica la visita en un stream y responde enseguida. Un Worker toma la visita del stream y la suma al contador de la página en la base de datos. La latencia se mide hasta que la visita queda contada, no hasta que responde la Collector.
- **Abrir el panel de estadísticas** (1 %). La Collector lee el contador de la página, primero de una Cache si tu diseño la tiene y, si no, de la base de datos.

Hay 1 000 000 de páginas y su popularidad sigue una curva Zipf 1,0: unas pocas páginas se llevan buena parte de las visitas. No interviene ningún proveedor externo.

Si guardas los contadores en caché, el panel puede mostrar una cifra de hace unos segundos. Aquí no importa: a nadie le molesta un panel que va un poco por detrás, así que las lecturas desactualizadas no se evalúan.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **La portada se hace viral** y **Los consumidores se ralentizan**. Tu diseño tiene que superar las tres.
