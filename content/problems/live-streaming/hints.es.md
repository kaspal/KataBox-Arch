## 1

Casi todo el tráfico son espectadores descargando los mismos pocos segmentos de los mismos pocos canales. ¿Tiene que llegar cada una de esas descargas al almacenamiento que guarda el vídeo? Y el chat: ¿qué pasa con el número de mensajes que hay que entregar cuando chatea y mira cuatro veces más gente?

## 2

El vídeo en directo es casi todo lectura: miles de espectadores piden el mismo segmento en pocos segundos. Una CDN delante del almacenamiento de objetos les responde con su propia copia, así que el origen solo ve los fallos de caché, y un origen lento apenas se nota. El chat es un problema de difusión: cada mensaje enviado se entrega a muchos espectadores. Dimensiona los gateways de WebSocket por mensajes por segundo, no solo por conexiones, y guarda margen para una raid que multiplica el chat por cuatro.

## 3

Pon una CDN CloudFront con un TTL de 4 s delante de un bucket de Amazon S3 para los segmentos. Para el chat, conecta el Cliente a través de un AWS ALB a 2 tarjetas Gateway de WebSocket Socket.IO, cada una con 2 instancias, Conexiones máximas 200 000, Mensajes por segundo 200 000 y Conexiones nuevas por segundo 20 000: 800 000 mensajes por segundo para una raid de 400 000. Pon la Variación al reconectar del Cliente en 10 s. Los mensajes enviados van a 2 servidores de aplicaciones Go y a DynamoDB con 16 particiones.
