## 1

Con un solo proveedor, su minuto malo es tu minuto malo. Un segundo proveedor solo ayuda si tus llamadas pasan de verdad a él. Un proveedor caído se detecta en unos segundos, pero uno que acepta tu llamada y nunca responde parece sano. ¿Cuánto tiempo lo espera tu Servidor de aplicaciones, y qué le pasa al mensaje que estaba esperando?

## 2

Deja de esperar en algún momento: un tiempo límite de llamada un poco por encima de tu respuesta normal más larga. Cuando fallan muchas llamadas a un proveedor, deja de enviarle llamadas durante un rato (un cortocircuito), para que los fallos no cuesten nada en vez de un tiempo límite cada uno. Después reintenta la llamada fallida: el siguiente intento va al otro proveedor. Y el otro proveedor tiene que poder llevar toda la carga él solo, dentro de sus límites.

## 3

Pon un AWS ALB delante de 2 Servidores de aplicaciones Go con un Tiempo límite de llamada de 5000 ms, el Cortocircuito con errores al 20 % y el Cortocircuito abierto durante 60 s. Mete los Servidores de aplicaciones en una Zona de disponibilidad con una Malla de servicios Linkerd y Reintentos de la malla 3. Conéctalos a una tarjeta API de Claude (4000 peticiones y 2 000 000 de tokens por minuto) y a una tarjeta API de OpenAI (1500 peticiones y 1 000 000 de tokens por minuto), y pon el Tiempo límite del Cliente web en 30 000 ms.
