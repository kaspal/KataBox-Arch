# Solución comentada

## Adónde va la carga

Con 40 envíos/s y unos 2 segundos cada uno, hay unos 80 envíos evaluándose en cada momento. Una instancia de Judge Worker ejecuta 8 a la vez, así que una tarde tranquila necesita 10 instancias solo para seguir el ritmo, y 12 dejan un poco de margen. La Submit API casi no hace nada: pone el envío en la Queue y responde. Todo el coste real está en los jueces, y la latencia que cuenta es el tiempo hasta que el veredicto está guardado. Cuando los jueces se retrasan, ese tiempo es sobre todo espera en la Queue.

## El diseño de referencia

Un Load Balancer reparte el tráfico entre 2 servidores Go de la Submit API. Publican en una Queue SQS con un max depth de 1 000 000. La consumen Judge Workers, empezando con 12 instancias de 8 huecos, dentro de un Autoscaling group con un mínimo de 12, un máximo de 80, una utilización objetivo del 60 % y un retardo de escalado de 10 segundos. Cada veredicto se escribe en PostgreSQL.

Cuesta unos 2 008 $ al mes. En una tarde tranquila el p95 ronda los 5,7 s. Cuando empieza el concurso, el tráfico sube de 1 a 6 veces a lo largo del minuto, hasta 240 envíos por segundo; el grupo sigue añadiendo instancias a medida que sube la utilización, y el p95 ronda los 21 s. Cuando se cae una máquina de evaluación, la tarjeta de los Judge Workers queda fuera de servicio durante 20 segundos; los envíos esperan en la Queue, no se pierde ninguno y el p95 ronda los 23 s. Las dos pruebas ocultas admiten un p95 de hasta 30 s.

## Por qué falla el primer intento

Un primer intento habitual es la misma cadena con 12 instancias fijas y sin Autoscaling group. Una tarde tranquila pasa con un p95 cercano a 6,5 s. Durante el concurso, los envíos pronto llegan más rápido de lo que 96 huecos pueden evaluar. No se pierde nada, porque la Queue es profunda, pero la acumulación crece cada segundo y el p95 llega a unos 51 s.

## Para probar

- Sube el retardo de escalado a 60 segundos y vuelve a ejecutar el concurso. ¿Dónde queda el p95?
- Baja el máximo a 30 instancias y averigua si el grupo sigue el ritmo.
- Baja el mínimo a 4 y compara el p95 de la tarde tranquila y el coste mensual.
