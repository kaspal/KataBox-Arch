El DNS es la guía telefónica de internet. Antes de que un navegador pueda enviar una petición a example.com, pregunta a un **resolvedor** por la dirección que hay detrás de ese nombre. La respuesta viene con un **TTL**, y los resolvedores y los clientes la guardan ese tiempo, así que la mayoría de las peticiones nunca provocan una consulta nueva.

Como las respuestas se cachean y el servicio está repartido en muchos servidores, el DNS casi nunca es lo que limita un sistema. Importa más para decidir adónde va el tráfico: un DNS gestionado puede mandar a los usuarios a la región más cercana, o apartarlos de una que ha caído, aunque solo tan rápido como caducan las respuestas cacheadas.

En el simulador, una tarjeta de DNS es un salto rápido en el camino de la petición, con capacidad prácticamente ilimitada. Añade un pequeño tiempo de consulta y un precio por consulta. El TTL es una nota de diseño: no cambia la ejecución.
