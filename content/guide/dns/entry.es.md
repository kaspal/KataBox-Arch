El DNS es la guía telefónica de internet. Antes de que un navegador pueda enviar una petición a example.com, pregunta a un **resolvedor** por la dirección que hay detrás de ese nombre. La respuesta viene con un **TTL**, y los resolvedores y los clientes la guardan ese tiempo, así que la mayoría de las peticiones nunca provocan una consulta nueva.

Como las respuestas se cachean y el servicio está repartido en muchos servidores, el DNS casi nunca es lo que limita un sistema. Importa más para decidir adónde va el tráfico: un DNS gestionado puede mandar a los usuarios a la región más cercana, o apartarlos de una que ha caído, aunque solo tan rápido como caducan las respuestas cacheadas.

En el simulador, una tarjeta de DNS es un salto rápido en el camino de la petición, con capacidad prácticamente ilimitada. Añade un pequeño tiempo de consulta y un precio por consulta. El TTL es una nota de diseño: no cambia la ejecución.

**Cómo lo factura el simulador.** Los clientes y los resolvedores guardan cada respuesta durante su TTL, así que solo alrededor de 1 petición de cada 100 llega al servicio de DNS como consulta, y solo esas se cobran. Por eso el DNS sale casi gratis en cualquier diseño. Route 53 y Cloudflare DNS se cobran igual, a 0,40 $ por millón de consultas (precio de lista, us-east-1, consultado el 2026-11-22).
