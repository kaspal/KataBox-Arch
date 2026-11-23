## 1

Una conexión abierta cuesta memoria en un gateway, y un gateway puede fallar. Si desaparece una tarjeta de gateway, ¿tienen las demás sitio para sus aficionados? ¿Y qué pasa cuando todos esos aficionados intentan volver en el mismo momento?

## 2

Dimensiona los gateways para N−1: sin una tarjeta cualquiera, las demás tienen que seguir alojando a los 5 000 000 de aficionados y guardar margen de mensajes para una ráfaga de actualizaciones. Después, reparte las reconexiones. Si todos los clientes caídos reintentan a la vez, los supervivientes rechazan casi todos los intentos, segundo tras segundo; una espera aleatoria antes de reconectar convierte esa tormenta en un flujo constante que los gateways pueden aceptar.

## 3

Usa 4 tarjetas Gateway de WebSocket Socket.IO detrás de un AWS ALB, cada una con 2 instancias, Conexiones máximas 1 000 000, Memoria 32 GB, Mensajes por segundo 200 000 y Conexiones nuevas por segundo 20 000. Pon la Variación al reconectar del Cliente en 20 s. Para las páginas de partido bastan de sobra 4 servidores de aplicaciones Go con una caché Redis (TTL 2 s) delante de PostgreSQL.
