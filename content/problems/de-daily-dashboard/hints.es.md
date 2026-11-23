## 1

A las 9 de la mañana el tráfico se multiplica por diez, pero el panel no: siguen siendo los mismos 400 recuadros. ¿Cuántas de esas cargas necesitan de verdad que el almacén recorra algo, y qué ocupa un hueco del almacén mientras esperan?

## 2

La Caché de resultados ya responde las repeticiones, pero cada consulta sigue yendo al almacén, y el 40 % recorre datos durante 0,8 s. Una caché delante, con el recuadro como clave, convierte la avalancha en unos cientos de consultas como mucho, sea cual sea el tráfico. Cuando la caché empieza vacía, mucha gente pide el mismo recuadro que falta en el mismo momento: haz que esperen a una sola consulta en lugar de lanzar cada uno la suya.

## 3

Pon un AWS ALB delante de 2 Servidores de aplicaciones Go. Conéctalos a una Caché Redis con un TTL de 600 s y "Agrupar fallos (protección de estampida)" activado, y a un Almacén de datos Snowflake con sus valores por defecto (1 clúster, Caché de resultados activada).
